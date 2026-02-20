// Reconciliation runner for agent-active task states
//
// Ensures tasks don't get stuck when agent runs finish without transitions.
// Can be used on startup and during runtime polling.
//
// Submodules:
// - policy.rs: types + pure decision logic (RecoveryPolicy, RecoveryContext, etc.)
// - handlers.rs: all reconcile_* methods, orchestration, apply_recovery_decision
// - (events.rs, metadata.rs, helpers.rs: to be extracted in future passes)

pub(crate) mod handlers;
pub(crate) mod policy;

use std::collections::HashSet;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex;
use tracing::warn;

use crate::application::{
    chat_service::reconcile_merge_auto_complete, GitService, TaskTransitionService,
};
use crate::commands::execution_commands::{ExecutionState, AGENT_ACTIVE_STATUSES};
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus, MergeFailureSource,
    MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
    MergeRecoverySource, MergeRecoveryState, Task, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatAttachmentRepository,
    ChatConversationRepository, ChatMessageRepository, IdeationSessionRepository,
    MemoryEventRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::transition_handler::{
    has_branch_missing_metadata, set_trigger_origin,
};
use crate::infrastructure::agents::claude::reconciliation_config;

pub use policy::UserRecoveryAction;
use policy::{
    RecoveryActionKind, RecoveryContext, RecoveryDecision, RecoveryEvidence, RecoveryPolicy,
    RecoveryPromptAction, RecoveryPromptEvent, ShaComparisonResult,
};

pub struct ReconciliationRunner<R: Runtime = tauri::Wry> {
    pub(crate) task_repo: Arc<dyn TaskRepository>,
    pub(crate) task_dep_repo: Arc<dyn TaskDependencyRepository>,
    pub(crate) project_repo: Arc<dyn ProjectRepository>,
    pub(crate) chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    pub(crate) chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub(crate) chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub(crate) ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub(crate) activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub(crate) message_queue: Arc<MessageQueue>,
    pub(crate) running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub(crate) memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub(crate) agent_run_repo: Arc<dyn AgentRunRepository>,
    pub(crate) transition_service: Arc<TaskTransitionService<R>>,
    pub(crate) execution_state: Arc<ExecutionState>,
    pub(crate) plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    pub(crate) app_handle: Option<AppHandle<R>>,
    pub(crate) policy: RecoveryPolicy,
    pub(crate) prompt_tracker: Arc<Mutex<HashSet<String>>>,
}

impl<R: Runtime> ReconciliationRunner<R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_dep_repo: Arc<dyn TaskDependencyRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        chat_conversation_repo: Arc<dyn ChatConversationRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        transition_service: Arc<TaskTransitionService<R>>,
        execution_state: Arc<ExecutionState>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        Self {
            task_repo,
            task_dep_repo,
            project_repo,
            chat_conversation_repo,
            chat_message_repo,
            chat_attachment_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            agent_run_repo,
            transition_service,
            execution_state,
            plan_branch_repo: None,
            app_handle,
            policy: RecoveryPolicy,
            prompt_tracker: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }
}

// Remaining impl methods (to be extracted in future passes):
impl<R: Runtime> ReconciliationRunner<R> {
    async fn build_run_evidence(
        &self,
        task: &Task,
        context_type: ChatContextType,
        run: Option<&AgentRun>,
    ) -> RecoveryEvidence {
        let registry_running = self
            .running_agent_registry
            .is_running(&crate::domain::services::RunningAgentKey::new(
                context_type.to_string(),
                task.id.as_str(),
            ))
            .await;

        RecoveryEvidence {
            run_status: run.map(|r| r.status),
            registry_running,
            can_start: self.execution_state.can_start_task(),
            is_stale: false,
            is_deferred: false,
        }
    }

    async fn load_execution_run(&self, task: &Task, status: InternalStatus) -> Option<AgentRun> {
        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to load status history for reconciliation"
                );
                return None;
            }
        };

        let latest_transition = status_history
            .iter()
            .rev()
            .find(|transition| transition.to == status && transition.agent_run_id.is_some());

        let run = if let Some(transition) = latest_transition {
            let agent_run_id = transition.agent_run_id.as_ref()?;

            match self
                .agent_run_repo
                .get_by_id(&AgentRunId::from_string(agent_run_id))
                .await
            {
                Ok(run) => run,
                Err(e) => {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to load agent run for reconciliation"
                    );
                    return None;
                }
            }
        } else {
            self.lookup_latest_run_for_task_context(task, ChatContextType::TaskExecution)
                .await
        };

        run
    }

    async fn latest_status_transition_age(
        &self,
        task: &Task,
        status: InternalStatus,
    ) -> Option<chrono::Duration> {
        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    status = ?status,
                    error = %e,
                    "Failed to load status history; falling back to task.updated_at for age"
                );
                return Some(chrono::Utc::now() - task.updated_at);
            }
        };

        if let Some(latest_transition) = status_history
            .iter()
            .rev()
            .find(|transition| transition.to == status)
        {
            return Some(chrono::Utc::now() - latest_transition.timestamp);
        }

        // Fallback for manually-edited tasks (status changed without status history row).
        Some(chrono::Utc::now() - task.updated_at)
    }

    /// Count `AttemptFailed` events in merge recovery metadata (Merging state retries).
    fn merging_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AttemptFailed))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    /// Count auto-retry events for a given status in task metadata.
    /// Uses a metadata key like "auto_retry_count_{status}" to track retries.
    fn auto_retry_count_for_status(task: &Task, status: InternalStatus) -> u32 {
        let metadata = match task.metadata.as_deref() {
            Some(m) => m,
            None => return 0,
        };
        let json: serde_json::Value = match serde_json::from_str(metadata) {
            Ok(v) => v,
            Err(_) => return 0,
        };
        let key = format!("auto_retry_count_{}", status);
        json.get(&key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
    }

    /// Record an auto-retry attempt in task metadata.
    async fn record_auto_retry_metadata(
        &self,
        task: &Task,
        status: InternalStatus,
        attempt: u32,
    ) -> Result<(), String> {
        let mut updated = task.clone();
        let mut json: serde_json::Value = updated
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str(m).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let key = format!("auto_retry_count_{}", status);
        if let Some(obj) = json.as_object_mut() {
            obj.insert(key, serde_json::json!(attempt));
        }

        updated.metadata = Some(json.to_string());
        updated.touch();
        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    fn merge_incomplete_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    fn merge_incomplete_retry_delay(retry_count: u32) -> chrono::Duration {
        use rand::Rng;
        let exponent = retry_count.min(6);
        let scaled = (reconciliation_config().merge_incomplete_retry_base_secs as i64)
            .saturating_mul(1_i64 << exponent);
        let base_delay = scaled.min(reconciliation_config().merge_incomplete_retry_max_secs as i64);
        let jitter = rand::thread_rng().gen_range(0..=base_delay / 4);
        chrono::Duration::seconds(base_delay + jitter)
    }

    fn merge_conflict_auto_retry_count(task: &Task) -> u32 {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .map(|meta| {
                meta.events
                    .iter()
                    .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AutoRetryTriggered))
                    .count() as u32
            })
            .unwrap_or(0)
    }

    fn merge_conflict_retry_delay(retry_count: u32) -> chrono::Duration {
        use rand::Rng;
        let exponent = retry_count.min(6);
        let scaled = (reconciliation_config().merge_conflict_retry_base_secs as i64)
            .saturating_mul(1_i64 << exponent);
        let base_delay = scaled.min(reconciliation_config().merge_conflict_retry_max_secs as i64);
        let jitter = rand::thread_rng().gen_range(0..=base_delay / 4);
        chrono::Duration::seconds(base_delay + jitter)
    }

    /// Returns true if the task's failure was explicitly reported by the merger agent
    /// (via report_conflict or report_incomplete endpoints). These should NOT be auto-retried
    /// because the agent made a deliberate decision requiring human intervention.
    pub(crate) fn is_agent_reported_failure(task: &Task) -> bool {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("merge_failure_source").cloned())
            .and_then(|v| serde_json::from_value::<MergeFailureSource>(v).ok())
            .map(|source| matches!(source, MergeFailureSource::AgentReported))
            .unwrap_or(false)
    }

    /// Returns the number of times post-merge validation has reverted the merge commit.
    /// Used to break validation→revert→retry loops.
    pub(crate) fn validation_revert_count(task: &Task) -> u32 {
        task.metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v.get("validation_revert_count").and_then(|c| c.as_u64()).map(|c| c as u32))
            .unwrap_or(0)
    }

    /// Get rate_limit_retry_after from merge recovery metadata, if set.
    fn get_rate_limit_retry_after(task: &Task) -> Option<String> {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .and_then(|meta| meta.rate_limit_retry_after)
    }

    /// Clear the rate_limit_retry_after field from merge recovery metadata (after expiry).
    async fn clear_rate_limit_retry_after(&self, task: &Task) -> Result<(), String> {
        let mut updated = task.clone();
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        recovery.rate_limit_retry_after = None;
        if recovery.last_state == MergeRecoveryState::RateLimited {
            recovery.last_state = MergeRecoveryState::Retrying;
        }

        updated.metadata = Some(
            recovery
                .update_task_metadata(updated.metadata.as_deref())
                .map_err(|e| e.to_string())?,
        );
        updated.touch();

        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get the last stored source branch SHA from merge recovery events.
    pub(crate) fn last_stored_source_sha(task: &Task) -> Option<String> {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .and_then(|meta| {
                meta.events
                    .iter()
                    .rev()
                    .find_map(|e| e.source_sha.clone())
            })
    }

    /// Get the current HEAD SHA of the task's source branch using GitService.
    async fn get_current_source_sha(&self, task: &Task) -> Option<String> {
        let branch = task.task_branch.as_deref()?;
        let project_data = self.project_repo.get_by_id(&task.project_id).await.ok()??;
        let repo_path = std::path::Path::new(&project_data.working_directory);
        match GitService::get_branch_sha(repo_path, branch).await {
            Ok(sha) => Some(sha),
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    branch = branch,
                    error = %e,
                    "Failed to get current source branch SHA for comparison guard"
                );
                None
            }
        }
    }

    /// Compare the source branch SHA at last failure time with the current SHA.
    async fn check_source_sha_changed(&self, task: &Task) -> ShaComparisonResult {
        let last_sha = match Self::last_stored_source_sha(task) {
            Some(sha) => sha,
            None => return ShaComparisonResult::NoStoredSha,
        };

        let current_sha = match self.get_current_source_sha(task).await {
            Some(sha) => sha,
            None => return ShaComparisonResult::GitError,
        };

        if current_sha == last_sha {
            ShaComparisonResult::Unchanged(current_sha)
        } else {
            ShaComparisonResult::Changed {
                old_sha: last_sha,
                new_sha: current_sha,
            }
        }
    }

    /// Record auto-retry event with optional source SHA (for SHA comparison guard).
    async fn record_merge_auto_retry_event_with_sha(
        &self,
        task: &Task,
        attempt: u32,
        failure_source: MergeFailureSource,
        retry_reason: &str,
        source_sha: Option<&str>,
    ) -> Result<(), String> {
        let mut updated = task.clone();
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        let mut event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::GitError,
            format!(
                "Auto-retry triggered (attempt {}, source={:?}): {}",
                attempt, failure_source, retry_reason
            ),
        )
        .with_attempt(attempt)
        .with_failure_source(failure_source);

        if let Some(ref source_branch) = updated.task_branch {
            event = event.with_source_branch(source_branch.clone());
        }
        if let Some(sha) = source_sha {
            event = event.with_source_sha(sha);
        }

        recovery.append_event_with_state(event, MergeRecoveryState::Retrying);
        updated.metadata = Some(
            recovery
                .update_task_metadata(updated.metadata.as_deref())
                .map_err(|e| e.to_string())?,
        );
        set_trigger_origin(&mut updated, "recovery");
        updated.touch();

        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    fn latest_deferred_blocker_id(&self, task: &Task) -> Option<TaskId> {
        MergeRecoveryMetadata::from_task_metadata(task.metadata.as_deref())
            .ok()
            .flatten()
            .and_then(|meta| {
                meta.events
                    .iter()
                    .rev()
                    .find_map(|event| event.blocking_task_id.clone())
            })
    }

    async fn deferred_blocker_is_active(&self, blocker_id: &TaskId) -> bool {
        match self.task_repo.get_by_id(blocker_id).await {
            Ok(Some(blocker)) => {
                blocker.archived_at.is_none()
                    && matches!(
                        blocker.internal_status,
                        InternalStatus::PendingMerge | InternalStatus::Merging
                    )
            }
            Ok(None) => false,
            Err(e) => {
                warn!(
                    blocker_id = blocker_id.as_str(),
                    error = %e,
                    "Failed to load deferred merge blocker status"
                );
                true
            }
        }
    }

    async fn record_merge_auto_retry_event(
        &self,
        task: &Task,
        attempt: u32,
        failure_source: MergeFailureSource,
        retry_reason: &str,
    ) -> Result<(), String> {
        let mut updated = task.clone();
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        let mut event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::GitError,
            format!(
                "Auto-retry triggered (attempt {}, source={:?}): {}",
                attempt, failure_source, retry_reason
            ),
        )
        .with_attempt(attempt)
        .with_failure_source(failure_source);

        if let Some(ref source_branch) = updated.task_branch {
            event = event.with_source_branch(source_branch.clone());
        }

        recovery.append_event_with_state(event, MergeRecoveryState::Retrying);
        updated.metadata = Some(
            recovery
                .update_task_metadata(updated.metadata.as_deref())
                .map_err(|e| e.to_string())?,
        );
        set_trigger_origin(&mut updated, "recovery");
        updated.touch();

        self.task_repo
            .update(&updated)
            .await
            .map_err(|e| e.to_string())
    }

    async fn record_merge_timeout_event(&self, task: &Task, age: chrono::Duration) {
        let mut updated = task.clone();

        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        let failed_event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AttemptFailed,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::GitError,
            format!(
                "Merge timed out after {}s without completion signal",
                age.num_seconds().max(0)
            ),
        )
        .with_failure_source(MergeFailureSource::TransientGit);
        recovery.append_event_with_state(failed_event, MergeRecoveryState::Failed);

        let mut metadata = match recovery.update_task_metadata(updated.metadata.as_deref()) {
            Ok(json) => serde_json::from_str::<serde_json::Value>(&json)
                .unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        };

        let timeout_secs = reconciliation_config().merger_timeout_secs as i64;
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert(
                "error".to_string(),
                serde_json::json!(format!(
                    "Merge timed out after {}s without complete_merge callback",
                    timeout_secs
                )),
            );
            obj.insert(
                "merge_timeout_seconds".to_string(),
                serde_json::json!(timeout_secs),
            );
            obj.insert(
                "merge_timeout_at".to_string(),
                serde_json::json!(chrono::Utc::now().to_rfc3339()),
            );
        }

        updated.metadata = Some(metadata.to_string());
        updated.touch();
        if let Err(e) = self.task_repo.update(&updated).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to persist merge timeout metadata"
            );
        }
    }

    async fn emit_recovery_prompt(
        &self,
        task: &Task,
        status: InternalStatus,
        context: RecoveryContext,
        reason: String,
    ) -> bool {
        let Some(handle) = &self.app_handle else {
            return false;
        };

        let key = format!("{}:{:?}", task.id.as_str(), status);
        {
            let mut prompted = self.prompt_tracker.lock().await;
            if prompted.contains(&key) {
                return false;
            }
            prompted.insert(key);
        }

        let context_type = match context {
            RecoveryContext::Execution => "execution",
            RecoveryContext::Review => "review",
            RecoveryContext::Merge => "merge",
            RecoveryContext::PendingMerge => "pending_merge",
            RecoveryContext::QaRefining => "qa_refining",
            RecoveryContext::QaTesting => "qa_testing",
        };

        let payload = RecoveryPromptEvent {
            task_id: task.id.as_str().to_string(),
            status,
            context_type: context_type.to_string(),
            reason,
            primary_action: RecoveryPromptAction {
                id: "restart".to_string(),
                label: "Restart".to_string(),
            },
            secondary_action: RecoveryPromptAction {
                id: "cancel".to_string(),
                label: "Cancel".to_string(),
            },
        };

        let _ = handle.emit("recovery:prompt", payload);
        true
    }

    async fn clear_prompt_marker(&self, task_id: &str, status: InternalStatus) {
        let key = format!("{}:{:?}", task_id, status);
        let mut prompted = self.prompt_tracker.lock().await;
        prompted.remove(&key);
    }

    async fn lookup_latest_run_for_task_context(
        &self,
        task: &Task,
        context_type: ChatContextType,
    ) -> Option<AgentRun> {
        let conversations = self
            .chat_conversation_repo
            .get_by_context(context_type, task.id.as_str())
            .await
            .ok()?;

        let latest_conversation = conversations
            .into_iter()
            .max_by_key(|conv| conv.created_at)?;

        self.agent_run_repo
            .get_latest_for_conversation(&latest_conversation.id)
            .await
            .ok()
            .flatten()
    }
}

#[cfg(test)]
mod tests;
