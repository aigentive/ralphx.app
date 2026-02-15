// Spawn orchestrator job entity - background job for spawning orchestrator agents
//
// This module defines the entity for the spawn orchestrator system,
// which manages asynchronous spawning of orchestrator agents from ideation sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::domain::entities::types::{IdeationSessionId, ProjectId};
use crate::error::AppError;

/// Unique identifier for spawn orchestrator jobs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpawnOrchestratorJobId(pub String);

impl fmt::Display for SpawnOrchestratorJobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SpawnOrchestratorJobId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SpawnOrchestratorJobId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Status of a spawn orchestrator job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnOrchestratorJobStatus {
    /// Job is waiting to be processed
    Pending,
    /// Job is currently being processed
    Running,
    /// Job completed successfully
    Done,
    /// Job failed with error
    Failed,
}

impl fmt::Display for SpawnOrchestratorJobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Done => write!(f, "done"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for SpawnOrchestratorJobStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::Validation(format!(
                "Invalid spawn orchestrator job status: {}",
                s
            ))),
        }
    }
}

/// Spawn orchestrator job entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnOrchestratorJob {
    /// Unique identifier for this job
    pub id: SpawnOrchestratorJobId,
    /// The ideation session that triggered this spawn
    pub session_id: IdeationSessionId,
    /// The project context for the spawn
    pub project_id: ProjectId,
    /// Human-readable description of the spawn task
    pub description: String,
    /// Current status of the job
    pub status: SpawnOrchestratorJobStatus,
    /// Error message if the job failed
    pub error_message: Option<String>,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
    /// When the job started processing
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of attempts to process this job (for retry tracking)
    pub attempt_count: u32,
}

impl SpawnOrchestratorJob {
    /// Create a new pending spawn orchestrator job
    pub fn new(
        session_id: IdeationSessionId,
        project_id: ProjectId,
        description: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: SpawnOrchestratorJobId(uuid::Uuid::new_v4().to_string()),
            session_id,
            project_id,
            description: description.into(),
            status: SpawnOrchestratorJobStatus::Pending,
            error_message: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            attempt_count: 0,
        }
    }

    /// Mark job as running, incrementing attempt count
    pub fn start(&mut self) {
        self.status = SpawnOrchestratorJobStatus::Running;
        self.attempt_count += 1;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark job as completed successfully
    pub fn complete(&mut self) {
        self.status = SpawnOrchestratorJobStatus::Done;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark job as failed with error message
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = SpawnOrchestratorJobStatus::Failed;
        self.error_message = Some(error.into());
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if job can be claimed (is pending or failed with retries available)
    pub fn can_claim(&self) -> bool {
        matches!(
            self.status,
            SpawnOrchestratorJobStatus::Pending | SpawnOrchestratorJobStatus::Failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_orchestrator_job_status_serialization() {
        assert_eq!(SpawnOrchestratorJobStatus::Pending.to_string(), "pending");
        assert_eq!(SpawnOrchestratorJobStatus::Running.to_string(), "running");
        assert_eq!(SpawnOrchestratorJobStatus::Done.to_string(), "done");
        assert_eq!(SpawnOrchestratorJobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_spawn_orchestrator_job_status_parsing() {
        assert_eq!(
            "pending".parse::<SpawnOrchestratorJobStatus>().unwrap(),
            SpawnOrchestratorJobStatus::Pending
        );
        assert_eq!(
            "running".parse::<SpawnOrchestratorJobStatus>().unwrap(),
            SpawnOrchestratorJobStatus::Running
        );
        assert_eq!(
            "done".parse::<SpawnOrchestratorJobStatus>().unwrap(),
            SpawnOrchestratorJobStatus::Done
        );
        assert_eq!(
            "failed".parse::<SpawnOrchestratorJobStatus>().unwrap(),
            SpawnOrchestratorJobStatus::Failed
        );
        assert!("invalid".parse::<SpawnOrchestratorJobStatus>().is_err());
    }

    #[test]
    fn test_spawn_orchestrator_job_lifecycle() {
        let session_id = IdeationSessionId::from_string("test-session");
        let project_id = ProjectId::from_string("test-project".to_string());
        let mut job = SpawnOrchestratorJob::new(session_id, project_id.clone(), "Test spawn job");

        // Initially pending
        assert_eq!(job.status, SpawnOrchestratorJobStatus::Pending);
        assert!(job.can_claim());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
        assert_eq!(job.attempt_count, 0);

        // Start job
        job.start();
        assert_eq!(job.status, SpawnOrchestratorJobStatus::Running);
        assert!(!job.can_claim());
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_none());
        assert_eq!(job.attempt_count, 1);

        // Complete job
        job.complete();
        assert_eq!(job.status, SpawnOrchestratorJobStatus::Done);
        assert!(!job.can_claim());
        assert!(job.completed_at.is_some());
        assert!(job.error_message.is_none());
    }

    #[test]
    fn test_spawn_orchestrator_job_failure() {
        let session_id = IdeationSessionId::from_string("test-session");
        let project_id = ProjectId::from_string("test-project".to_string());
        let mut job = SpawnOrchestratorJob::new(session_id, project_id, "Test spawn job");

        job.start();
        job.fail("Test error");

        assert_eq!(job.status, SpawnOrchestratorJobStatus::Failed);
        assert!(job.can_claim()); // Failed jobs can be retried
        assert!(job.completed_at.is_some());
        assert_eq!(job.error_message.as_deref(), Some("Test error"));
    }

    #[test]
    fn test_spawn_orchestrator_job_retry_tracking() {
        let session_id = IdeationSessionId::from_string("test-session");
        let project_id = ProjectId::from_string("test-project".to_string());
        let mut job = SpawnOrchestratorJob::new(session_id, project_id, "Test spawn job");

        // First attempt
        job.start();
        assert_eq!(job.attempt_count, 1);
        job.fail("First failure");

        // Second attempt (retry)
        job.start();
        assert_eq!(job.attempt_count, 2);
        job.complete();

        assert_eq!(job.status, SpawnOrchestratorJobStatus::Done);
        assert_eq!(job.attempt_count, 2);
    }

    #[test]
    fn test_spawn_orchestrator_job_id_display() {
        let id = SpawnOrchestratorJobId::from("job-123");
        assert_eq!(format!("{}", id), "job-123");
    }

    #[test]
    fn test_spawn_orchestrator_job_id_from_str() {
        let id = SpawnOrchestratorJobId::from("job-456");
        assert_eq!(id.0, "job-456");
    }
}
