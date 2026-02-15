// Spawn Orchestrator Worker
//
// Background worker that processes SpawnOrchestratorJob queue entries.
// Each job spawns an orchestrator-ideation agent via Claude CLI.
//
// Flow:
// 1. Claims pending job from repository (atomic claim_pending operation)
// 2. Builds prompt from job description with session context
// 3. Spawns orchestrator-ideation agent via ClaudeCodeClient
// 4. Waits for completion
// 5. Updates job status (complete/fail)
//
// Retry Logic:
// - Failed jobs can be re-claimed (can_claim() returns true for Failed status)
// - Max 3 attempts before giving up (checked via attempt_count field)

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error, info, warn};

use crate::domain::agents::AgentConfig;
use crate::domain::entities::SpawnOrchestratorJobStatus;
use crate::domain::repositories::SpawnOrchestratorJobRepository;
use crate::error::AppResult;
use crate::infrastructure::agents::claude::{
    agent_names::AGENT_ORCHESTRATOR_IDEATION, StreamMessage, StreamProcessor,
    StreamingSpawnResult,
};
use crate::infrastructure::agents::ClaudeCodeClient;

/// Maximum number of retry attempts for a job
const MAX_ATTEMPTS: u32 = 3;

/// Environment variable name for project ID
const ENV_RALPHX_PROJECT_ID: &str = "RALPHX_PROJECT_ID";

/// Environment variable name for session ID
const ENV_RALPHX_SESSION_ID: &str = "RALPHX_SESSION_ID";

/// Worker that processes spawn orchestrator jobs from the queue.
///
/// This worker runs in the background and continuously processes jobs
/// that were created when ideation sessions are accepted.
pub struct SpawnOrchestratorWorker {
    /// Repository for spawn orchestrator job persistence
    job_repo: Arc<dyn SpawnOrchestratorJobRepository>,
    /// Claude CLI client for spawning agents
    claude_client: Arc<ClaudeCodeClient>,
    /// Path to the ralphx-plugin directory
    plugin_dir: PathBuf,
    /// Path to the project root (working directory for agent)
    project_root: PathBuf,
}

impl SpawnOrchestratorWorker {
    /// Create a new SpawnOrchestratorWorker
    ///
    /// # Arguments
    /// * `job_repo` - Repository for job persistence
    /// * `claude_client` - Claude CLI client for spawning agents
    /// * `plugin_dir` - Path to ralphx-plugin directory for agent discovery
    /// * `project_root` - Path to project root (agent working directory)
    pub fn new(
        job_repo: Arc<dyn SpawnOrchestratorJobRepository>,
        claude_client: Arc<ClaudeCodeClient>,
        plugin_dir: PathBuf,
        project_root: PathBuf,
    ) -> Self {
        Self {
            job_repo,
            claude_client,
            plugin_dir,
            project_root,
        }
    }

    /// Run a single iteration of the worker.
    ///
    /// Claims a pending job, spawns an orchestrator agent, and updates status.
    /// Returns Ok(true) if a job was processed, Ok(false) if no jobs pending.
    pub async fn run_once(&self) -> AppResult<bool> {
        debug!("SpawnOrchestratorWorker: checking for pending jobs");

        // Atomically claim a pending job
        let job = match self.job_repo.claim_pending().await? {
            Some(job) => job,
            None => {
                debug!("SpawnOrchestratorWorker: no pending jobs");
                return Ok(false);
            }
        };

        info!(
            job_id = %job.id,
            session_id = %job.session_id,
            project_id = %job.project_id,
            attempt = job.attempt_count,
            "SpawnOrchestratorWorker: claimed job"
        );

        // Check retry limit
        if job.attempt_count > MAX_ATTEMPTS {
            warn!(
                job_id = %job.id,
                attempts = job.attempt_count,
                max = MAX_ATTEMPTS,
                "SpawnOrchestratorWorker: job exceeded max attempts, marking failed"
            );
            self.job_repo
                .update_status(
                    &job.id,
                    SpawnOrchestratorJobStatus::Failed,
                    Some(format!(
                        "Exceeded maximum retry attempts ({})",
                        MAX_ATTEMPTS
                    )),
                )
                .await?;
            return Ok(true);
        }

        // Process the job
        match self.process_job(&job).await {
            Ok(()) => {
                info!(job_id = %job.id, "SpawnOrchestratorWorker: job completed successfully");
                self.job_repo
                    .update_status(&job.id, SpawnOrchestratorJobStatus::Done, None)
                    .await?;
            }
            Err(e) => {
                error!(job_id = %job.id, error = %e, "SpawnOrchestratorWorker: job failed");
                self.job_repo
                    .update_status(
                        &job.id,
                        SpawnOrchestratorJobStatus::Failed,
                        Some(e.to_string()),
                    )
                    .await?;
            }
        }

        Ok(true)
    }

    /// Process a single spawn orchestrator job.
    ///
    /// Builds the agent config with session context, spawns the orchestrator,
    /// and waits for completion.
    async fn process_job(
        &self,
        job: &crate::domain::entities::SpawnOrchestratorJob,
    ) -> AppResult<()> {
        // Build prompt from job description
        let prompt = format!(
            "Session {}: {}",
            job.session_id.as_str(),
            job.description
        );

        debug!(job_id = %job.id, prompt = %prompt, "Building agent config");

        // Build agent config with orchestrator-ideation agent
        let config = AgentConfig {
            role: crate::domain::agents::AgentRole::Custom("orchestrator".to_string()),
            prompt,
            working_directory: self.project_root.clone(),
            plugin_dir: Some(self.plugin_dir.clone()),
            agent: Some(AGENT_ORCHESTRATOR_IDEATION.to_string()),
            model: None, // Use agent default from ralphx.yaml
            max_tokens: None,
            timeout_secs: None,
            env: {
                let mut env = std::collections::HashMap::new();
                env.insert(
                    ENV_RALPHX_PROJECT_ID.to_string(),
                    job.project_id.as_str().to_string(),
                );
                env.insert(
                    ENV_RALPHX_SESSION_ID.to_string(),
                    job.session_id.as_str().to_string(),
                );
                env
            },
        };

        // Spawn the orchestrator agent in streaming mode
        let result = self
            .claude_client
            .spawn_agent_streaming(config, None)
            .await
            .map_err(|e| crate::error::AppError::Agent(format!("Failed to spawn agent: {}", e)))?;

        info!(
            job_id = %job.id,
            handle_id = %result.handle.id,
            "SpawnOrchestratorWorker: agent spawned, waiting for completion"
        );

        // Wait for completion by processing the stream
        self.wait_for_completion(job.id.clone(), result).await
    }

    /// Wait for the spawned agent to complete.
    ///
    /// Processes stdout stream events until completion or error.
    async fn wait_for_completion(
        &self,
        job_id: crate::domain::entities::SpawnOrchestratorJobId,
        result: StreamingSpawnResult,
    ) -> AppResult<()> {
        let stdout = result
            .child
            .stdout
            .ok_or_else(|| {
                crate::error::AppError::Infrastructure("Agent stdout not available".to_string())
            })?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut processor = StreamProcessor::new();

        loop {
            let line = match lines.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => break, // EOF
                Err(e) => {
                    return Err(crate::error::AppError::Infrastructure(format!(
                        "Failed to read agent output: {}",
                        e
                    )));
                }
            };

            // Parse and process the stream message
            if let Some(parsed) = StreamProcessor::parse_line(&line) {
                // Check for result message which indicates completion
                if let StreamMessage::Result {
                    is_error,
                    ref errors,
                    ref session_id,
                    ..
                } = parsed.message
                {
                    if is_error {
                        let error_msg = errors.join("; ");
                        error!(
                            job_id = %job_id,
                            session_id = ?session_id,
                            error = %error_msg,
                            "SpawnOrchestratorWorker: agent reported error"
                        );
                        return Err(crate::error::AppError::Agent(format!(
                            "Agent error: {}",
                            error_msg
                        )));
                    } else {
                        info!(
                            job_id = %job_id,
                            session_id = ?session_id,
                            "SpawnOrchestratorWorker: agent completed"
                        );
                        return Ok(());
                    }
                }

                // Process the message to track state (after checking for completion)
                processor.process_parsed_line(parsed);
            }
        }

        // If we get here, the stream ended without a Result message
        // Check if the processor captured a session_id (might indicate success)
        let stream_result = processor.finish();
        if stream_result.is_error {
            let error_msg = stream_result.errors.join("; ");
            error!(
                job_id = %job_id,
                error = %error_msg,
                "SpawnOrchestratorWorker: stream ended with error"
            );
            return Err(crate::error::AppError::Agent(format!(
                "Agent error: {}",
                error_msg
            )));
        }

        warn!(
            job_id = %job_id,
            session_id = ?stream_result.session_id,
            "SpawnOrchestratorWorker: stream ended without result message"
        );

        Ok(())
    }
}

/// Run the spawn orchestrator worker in a loop.
///
/// This function runs continuously, processing jobs as they become available.
/// It sleeps for a short duration between iterations when no jobs are pending.
///
/// # Arguments
/// * `worker` - The worker instance
/// * `poll_interval` - Duration to sleep between polls when no jobs are pending
pub async fn run_worker_loop(worker: Arc<SpawnOrchestratorWorker>, poll_interval: Duration) {
    loop {
        match worker.run_once().await {
            Ok(true) => {
                // Job was processed, immediately check for more
                debug!("SpawnOrchestratorWorker: job processed, checking for more");
            }
            Ok(false) => {
                // No jobs pending, sleep before next check
                tokio::time::sleep(poll_interval).await;
            }
            Err(e) => {
                error!(error = %e, "SpawnOrchestratorWorker: error in run_once, sleeping before retry");
                tokio::time::sleep(poll_interval).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::types::{IdeationSessionId, ProjectId};
    use crate::domain::entities::SpawnOrchestratorJob;
    use crate::infrastructure::agents::claude::agent_names::AGENT_ORCHESTRATOR_IDEATION;

    #[test]
    fn test_agent_config_has_correct_agent_name() {
        // Verify the constant is what we expect
        assert_eq!(AGENT_ORCHESTRATOR_IDEATION, "ralphx:orchestrator-ideation");
    }

    #[test]
    fn test_env_constants() {
        assert_eq!(ENV_RALPHX_PROJECT_ID, "RALPHX_PROJECT_ID");
        assert_eq!(ENV_RALPHX_SESSION_ID, "RALPHX_SESSION_ID");
    }

    #[test]
    fn test_max_attempts() {
        assert_eq!(MAX_ATTEMPTS, 3);
    }

    #[test]
    fn test_spawn_orchestrator_job_creation() {
        let session_id = IdeationSessionId::from_string("test-session");
        let project_id = ProjectId::from_string("test-project".to_string());
        let job = SpawnOrchestratorJob::new(session_id, project_id, "Test description");

        assert_eq!(job.attempt_count, 0);
        assert!(job.can_claim());
        assert_eq!(job.status, SpawnOrchestratorJobStatus::Pending);
    }

    #[test]
    fn test_spawn_orchestrator_job_retry_logic() {
        let session_id = IdeationSessionId::from_string("test-session");
        let project_id = ProjectId::from_string("test-project".to_string());
        let mut job = SpawnOrchestratorJob::new(session_id, project_id, "Test");

        // Simulate multiple attempts
        job.start();
        assert_eq!(job.attempt_count, 1);

        job.fail("First failure");
        assert!(job.can_claim()); // Can retry

        job.start();
        assert_eq!(job.attempt_count, 2);

        job.fail("Second failure");
        assert!(job.can_claim());

        job.start();
        assert_eq!(job.attempt_count, 3);

        job.fail("Third failure");
        assert!(job.can_claim()); // Still can claim, worker checks MAX_ATTEMPTS
    }
}
