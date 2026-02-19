// Reconciliation runner for agent-active task states
//
// Ensures tasks don't get stuck when agent runs finish without transitions.
// Can be used on startup and during runtime polling.

use serde::Serialize;
use std::collections::HashSet;
use std::str::FromStr;
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

/// Merger agent timeout. After this many seconds without a completion signal,
/// the reconciler marks the task as MergeIncomplete so it surfaces to the user
/// and can be auto-retried. Configurable via RALPHX_MERGER_TIMEOUT_SECS env var
/// (default: 600 seconds / 10 minutes).
fn merging_timeout_seconds() -> i64 {
    std::env::var("RALPHX_MERGER_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(600)
}
const MERGING_MAX_AUTO_RETRIES: u32 = 3;
const PENDING_MERGE_STALE_MINUTES: i64 = 5;
const QA_STALE_MINUTES: i64 = 5;
const MERGE_INCOMPLETE_AUTO_RETRY_BASE_SECONDS: i64 = 30;
const MERGE_INCOMPLETE_AUTO_RETRY_MAX_SECONDS: i64 = 300;
const MERGE_INCOMPLETE_MAX_AUTO_RETRIES: u32 = 5;
/// Maximum number of validation-revert cycles before stopping auto-retry.
/// After this many reverts, the reconciler surfaces the task to the user.
const VALIDATION_REVERT_MAX_COUNT: u32 = 2;
const MERGE_CONFLICT_AUTO_RETRY_BASE_SECONDS: i64 = 60;
const MERGE_CONFLICT_AUTO_RETRY_MAX_SECONDS: i64 = 600;
const MERGE_CONFLICT_MAX_AUTO_RETRIES: u32 = 3;
const EXECUTING_MAX_AUTO_RETRIES: u32 = 5;
const REVIEWING_MAX_AUTO_RETRIES: u32 = 3;
const QA_MAX_AUTO_RETRIES: u32 = 3;
const EXECUTING_MAX_WALL_CLOCK_MINUTES: i64 = 60;
const REVIEWING_MAX_WALL_CLOCK_MINUTES: i64 = 30;
const QA_MAX_WALL_CLOCK_MINUTES: i64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryContext {
    Execution,
    Review,
    Merge,
    PendingMerge,
    QaRefining,
    QaTesting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecoveryActionKind {
    None,
    ExecuteEntryActions,
    Transition(InternalStatus),
    AttemptMergeAutoComplete,
    Prompt,
}

#[derive(Debug, Clone)]
struct RecoveryDecision {
    action: RecoveryActionKind,
    reason: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct RecoveryEvidence {
    run_status: Option<AgentRunStatus>,
    registry_running: bool,
    can_start: bool,
    is_stale: bool,
    is_deferred: bool,
}

impl RecoveryEvidence {
    fn has_conflict(&self) -> bool {
        match self.run_status {
            Some(AgentRunStatus::Running) => !self.registry_running,
            Some(_) => self.registry_running,
            None => self.registry_running,
        }
    }
}

#[derive(Default)]
struct RecoveryPolicy;

impl RecoveryPolicy {
    fn decide_reconciliation(
        &self,
        context: RecoveryContext,
        evidence: RecoveryEvidence,
    ) -> RecoveryDecision {
        match context {
            RecoveryContext::Execution => {
                if evidence.has_conflict() {
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Execution run state conflicts with running process tracking."
                                .to_string(),
                        ),
                    };
                }
                if evidence.run_status == Some(AgentRunStatus::Completed) {
                    return RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::PendingReview),
                        reason: None,
                    };
                }
                if evidence.run_status.is_none() {
                    if evidence.can_start {
                        return RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: None,
                        };
                    }
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Execution run missing but max concurrency is reached.".to_string(),
                        ),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::None,
                    reason: None,
                }
            }
            RecoveryContext::Review => {
                if evidence.has_conflict() {
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Review run state conflicts with running process tracking.".to_string(),
                        ),
                    };
                }
                if evidence.run_status == Some(AgentRunStatus::Completed) {
                    return RecoveryDecision {
                        action: RecoveryActionKind::ExecuteEntryActions,
                        reason: None,
                    };
                }
                if evidence.run_status.is_none() {
                    if evidence.can_start {
                        return RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: None,
                        };
                    }
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Review run missing but max concurrency is reached.".to_string(),
                        ),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::None,
                    reason: None,
                }
            }
            RecoveryContext::Merge => {
                if evidence.has_conflict() {
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Merge run state conflicts with running process tracking.".to_string(),
                        ),
                    };
                }
                if evidence.run_status == Some(AgentRunStatus::Completed) {
                    return RecoveryDecision {
                        action: RecoveryActionKind::AttemptMergeAutoComplete,
                        reason: None,
                    };
                }
                if evidence.is_stale {
                    return RecoveryDecision {
                        action: RecoveryActionKind::AttemptMergeAutoComplete,
                        reason: Some(
                            "Merge timed out — attempting auto-complete before escalating."
                                .to_string(),
                        ),
                    };
                }
                if evidence.run_status.is_none() {
                    if evidence.can_start {
                        return RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: None,
                        };
                    }
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "Merge run missing but max concurrency is reached.".to_string(),
                        ),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::None,
                    reason: None,
                }
            }
            RecoveryContext::PendingMerge => {
                if !evidence.is_stale {
                    return RecoveryDecision {
                        action: RecoveryActionKind::None,
                        reason: None,
                    };
                }
                if evidence.is_deferred {
                    return RecoveryDecision {
                        action: RecoveryActionKind::ExecuteEntryActions,
                        reason: Some(
                            "Stale deferred merge — re-triggering entry actions.".to_string(),
                        ),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                    reason: Some(
                        "Stale pending merge with no deferred flag — surfacing to user."
                            .to_string(),
                    ),
                }
            }
            RecoveryContext::QaRefining | RecoveryContext::QaTesting => {
                if !evidence.is_stale {
                    return RecoveryDecision {
                        action: RecoveryActionKind::None,
                        reason: None,
                    };
                }
                if !evidence.can_start {
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some(
                            "QA task is stale but max concurrency is reached.".to_string(),
                        ),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                }
            }
        }
    }

    fn decide_execution_stop(&self, evidence: RecoveryEvidence) -> RecoveryDecision {
        if evidence.has_conflict() {
            return RecoveryDecision {
                action: RecoveryActionKind::Prompt,
                reason: Some(
                    "Execution run state conflicts with running process tracking.".to_string(),
                ),
            };
        }
        if evidence.run_status == Some(AgentRunStatus::Completed) {
            return RecoveryDecision {
                action: RecoveryActionKind::Transition(InternalStatus::PendingReview),
                reason: None,
            };
        }
        RecoveryDecision {
            action: RecoveryActionKind::Transition(InternalStatus::Ready),
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryPromptAction {
    id: String,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryPromptEvent {
    task_id: String,
    status: InternalStatus,
    context_type: String,
    reason: String,
    primary_action: RecoveryPromptAction,
    secondary_action: RecoveryPromptAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRecoveryAction {
    Restart,
    Cancel,
}

/// Result of comparing source branch SHA at failure time vs current SHA.
#[derive(Debug)]
enum ShaComparisonResult {
    /// SHA unchanged — retrying would produce the same conflict
    Unchanged(String),
    /// SHA changed — new commits pushed, retry may succeed
    Changed { old_sha: String, new_sha: String },
    /// No SHA was stored at failure time — cannot compare, allow retry
    NoStoredSha,
    /// Git error while reading current SHA — allow retry conservatively
    GitError,
}

pub struct ReconciliationRunner<R: Runtime = tauri::Wry> {
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
    plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<AppHandle<R>>,
    policy: RecoveryPolicy,
    prompt_tracker: Arc<Mutex<HashSet<String>>>,
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

    /// Startup-only recovery: re-queue Failed tasks that failed due to transient timeouts.
    ///
    /// Runs once at app startup (not in the recurring loop). Finds tasks in Failed state where
    /// `is_timeout: true` (in metadata) and `auto_retry_count_executing < 3`, then transitions
    /// them back to Ready so they can be picked up on the next scheduling cycle.
    ///
    /// Cap at 3 attempts prevents infinite retry on persistent failures; timeout-only filter
    /// ensures we only recover transient overnight stalls, not logic errors.
    pub async fn recover_timeout_failures(&self) {
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                warn!(error = %e, "recover_timeout_failures: failed to get projects");
                return;
            }
        };

        for project in &projects {
            let failed_tasks = match self
                .task_repo
                .get_by_status(&project.id, InternalStatus::Failed)
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    warn!(
                        project_id = project.id.as_str(),
                        error = %e,
                        "recover_timeout_failures: failed to get Failed tasks"
                    );
                    continue;
                }
            };

            for task in failed_tasks {
                let is_timeout = self.task_is_timeout_failure(&task);
                let attempt_count =
                    Self::auto_retry_count_for_status(&task, InternalStatus::Executing);

                if !is_timeout || attempt_count >= 3 {
                    continue;
                }

                tracing::info!(
                    task_id = task.id.as_str(),
                    attempt_count = attempt_count,
                    "Startup recovery: re-queuing timeout-failed task"
                );

                // Increment attempt count before transitioning so the count persists
                // across recovery cycles and the cap is enforced correctly next startup.
                if let Err(e) = self
                    .record_auto_retry_metadata(&task, InternalStatus::Executing, attempt_count + 1)
                    .await
                {
                    warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Startup recovery: failed to increment attempt count (proceeding anyway)"
                    );
                }

                match self
                    .transition_service
                    .transition_task(&task.id, InternalStatus::Ready)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(
                            task_id = task.id.as_str(),
                            "Startup recovery: task transitioned Failed -> Ready"
                        );
                    }
                    Err(e) => {
                        warn!(
                            task_id = task.id.as_str(),
                            error = %e,
                            "Startup recovery: failed to transition task to Ready"
                        );
                    }
                }
            }
        }
    }

    /// Returns true if the task's metadata indicates it failed due to a timeout.
    fn task_is_timeout_failure(&self, task: &Task) -> bool {
        let metadata = match task.metadata.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let json: serde_json::Value = match serde_json::from_str(metadata) {
            Ok(v) => v,
            Err(_) => return false,
        };
        json.get("is_timeout")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub async fn reconcile_stuck_tasks(&self) {
        self.prune_stale_running_registry_entries().await;

        if self.execution_state.is_paused() {
            return;
        }

        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                warn!(error = %e, "Failed to get projects for reconciliation");
                return;
            }
        };

        for project in &projects {
            for status in [
                InternalStatus::Executing,
                InternalStatus::ReExecuting,
                InternalStatus::Reviewing,
                InternalStatus::Merging,
                InternalStatus::PendingMerge,
                InternalStatus::MergeIncomplete,
                InternalStatus::MergeConflict,
                InternalStatus::QaRefining,
                InternalStatus::QaTesting,
                InternalStatus::Paused,
            ] {
                let tasks = match self.task_repo.get_by_status(&project.id, status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for reconciliation"
                        );
                        continue;
                    }
                };

                for task in tasks {
                    let _ = self.reconcile_task(&task, status).await;
                }
            }
        }
    }

    async fn prune_stale_running_registry_entries(&self) {
        let entries = self.running_agent_registry.list_all().await;
        if entries.is_empty() {
            self.execution_state.set_running_count(0);
            return;
        }

        let mut removed = 0u32;

        for (key, info) in entries {
            let context_type = ChatContextType::from_str(&key.context_type).ok();
            let pid_alive = process_is_alive(info.pid);

            let run = match self
                .agent_run_repo
                .get_by_id(&AgentRunId::from_string(&info.agent_run_id))
                .await
            {
                Ok(run) => run,
                Err(e) => {
                    warn!(
                        context_type = key.context_type,
                        context_id = key.context_id,
                        run_id = info.agent_run_id,
                        error = %e,
                        "Failed to load agent_run while pruning running registry; keeping entry"
                    );
                    continue;
                }
            };

            let mut stale_reasons: Vec<&str> = Vec::new();

            if !pid_alive {
                stale_reasons.push("pid_missing");
            }

            match run.as_ref() {
                Some(agent_run) if agent_run.status != AgentRunStatus::Running => {
                    stale_reasons.push("run_not_running");
                }
                None => {
                    stale_reasons.push("run_missing");
                }
                _ => {}
            }

            if let Some(ctx) = context_type {
                if matches!(
                    ctx,
                    ChatContextType::TaskExecution
                        | ChatContextType::Review
                        | ChatContextType::Merge
                ) {
                    let task_id = TaskId::from_string(key.context_id.clone());
                    match self.task_repo.get_by_id(&task_id).await {
                        Ok(Some(task)) => {
                            if !context_matches_task_status(ctx, task.internal_status) {
                                stale_reasons.push("task_status_mismatch");
                            }
                        }
                        Ok(None) => stale_reasons.push("task_missing"),
                        Err(e) => {
                            warn!(
                                context_type = key.context_type,
                                context_id = key.context_id,
                                error = %e,
                                "Failed to load task while pruning running registry; keeping entry"
                            );
                            continue;
                        }
                    }
                }
            }

            if stale_reasons.is_empty() {
                continue;
            }

            if pid_alive {
                let _ = self.running_agent_registry.stop(&key).await;
            } else {
                let _ = self.running_agent_registry.unregister(&key).await;
            }
            removed += 1;

            if let Some(agent_run) = run {
                if agent_run.status == AgentRunStatus::Running {
                    let _ = self
                        .agent_run_repo
                        .cancel(&AgentRunId::from_string(&info.agent_run_id))
                        .await;
                }
            }

            warn!(
                context_type = key.context_type,
                context_id = key.context_id,
                pid = info.pid,
                run_id = info.agent_run_id,
                reasons = stale_reasons.join(","),
                "Pruned stale running agent registry entry"
            );
        }

        let registry_count = self.running_agent_registry.list_all().await.len() as u32;
        self.execution_state.set_running_count(registry_count);
        if removed > 0 {
            if let Some(handle) = self.app_handle.as_ref() {
                self.execution_state
                    .emit_status_changed(handle, "runtime_registry_gc");
            }
        }
    }

    pub async fn reconcile_task(&self, task: &Task, status: InternalStatus) -> bool {
        match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => {
                self.reconcile_completed_execution(task, status).await
            }
            InternalStatus::Reviewing => self.reconcile_reviewing_task(task, status).await,
            InternalStatus::Merging => self.reconcile_merging_task(task, status).await,
            InternalStatus::PendingMerge => self.reconcile_pending_merge_task(task, status).await,
            InternalStatus::MergeIncomplete => {
                self.reconcile_merge_incomplete_task(task, status).await
            }
            InternalStatus::MergeConflict => self.reconcile_merge_conflict_task(task, status).await,
            InternalStatus::QaRefining | InternalStatus::QaTesting => {
                self.reconcile_qa_task(task, status).await
            }
            InternalStatus::Paused => self.reconcile_paused_provider_error(task).await,
            _ => false,
        }
    }

    async fn reconcile_completed_execution(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Executing && status != InternalStatus::ReExecuting {
            return false;
        }

        let run = self.load_execution_run(task, status).await;
        let evidence = self
            .build_run_evidence(task, ChatContextType::TaskExecution, run.as_ref())
            .await;
        if evidence.run_status == Some(AgentRunStatus::Running) && evidence.registry_running {
            return true;
        }

        // C5: Wall-clock timeout for long-running executions
        if let Some(age) = self.latest_status_transition_age(task, status).await {
            let max_minutes = EXECUTING_MAX_WALL_CLOCK_MINUTES;
            if age >= chrono::Duration::minutes(max_minutes) {
                warn!(
                    task_id = task.id.as_str(),
                    age_minutes = age.num_minutes(),
                    max_minutes = max_minutes,
                    "Execution wall-clock timeout exceeded"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Execution,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Failed),
                            reason: Some(format!(
                                "Execution timed out after {} minutes (wall-clock limit: {}m)",
                                age.num_minutes(),
                                max_minutes
                            )),
                        },
                    )
                    .await;
            }
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Execution, evidence);

        // E7: Enforce retry limit for execution re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= EXECUTING_MAX_AUTO_RETRIES {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = EXECUTING_MAX_AUTO_RETRIES,
                    "Execution retry limit reached — escalating to Failed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Execution,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Failed),
                            reason: Some(format!(
                                "Execution failed {} times — escalating to Failed",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            // Record the retry attempt
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record execution retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, RecoveryContext::Execution, decision)
            .await
    }

    async fn reconcile_reviewing_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Reviewing {
            return false;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Review)
            .await;
        let evidence = self
            .build_run_evidence(task, ChatContextType::Review, run.as_ref())
            .await;
        if evidence.run_status == Some(AgentRunStatus::Running) && evidence.registry_running {
            return true;
        }

        // C5: Wall-clock timeout for long-running reviews
        if let Some(age) = self.latest_status_transition_age(task, status).await {
            let max_minutes = REVIEWING_MAX_WALL_CLOCK_MINUTES;
            if age >= chrono::Duration::minutes(max_minutes) {
                warn!(
                    task_id = task.id.as_str(),
                    age_minutes = age.num_minutes(),
                    max_minutes = max_minutes,
                    "Review wall-clock timeout exceeded"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Review,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Escalated),
                            reason: Some(format!(
                                "Review timed out after {} minutes (wall-clock limit: {}m)",
                                age.num_minutes(),
                                max_minutes
                            )),
                        },
                    )
                    .await;
            }
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Review, evidence);

        // E7: Enforce retry limit for review re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= REVIEWING_MAX_AUTO_RETRIES {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = REVIEWING_MAX_AUTO_RETRIES,
                    "Review retry limit reached — escalating to Escalated"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::Review,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::Escalated),
                            reason: Some(format!(
                                "Review failed {} times — escalating for manual review",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record review retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, RecoveryContext::Review, decision)
            .await
    }

    async fn reconcile_merging_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Merging {
            return false;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Merge)
            .await;
        let transition_age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };
        // Read last_active_at from registry to compute activity-based effective age.
        // Falls back to wall-clock transition age when no heartbeat exists.
        let merge_key = crate::domain::services::RunningAgentKey::new(
            ChatContextType::Merge.to_string(),
            task.id.as_str(),
        );
        let agent_info = self.running_agent_registry.get(&merge_key).await;
        let effective_age = agent_info
            .as_ref()
            .and_then(|info| info.last_active_at)
            .map(|last_active| chrono::Utc::now() - last_active)
            .unwrap_or(transition_age);
        let mut evidence = self
            .build_run_evidence(task, ChatContextType::Merge, run.as_ref())
            .await;
        evidence.is_stale = effective_age >= chrono::Duration::seconds(merging_timeout_seconds());

        // Agent is running, registered, and not stale — let it work
        if evidence.run_status == Some(AgentRunStatus::Running)
            && evidence.registry_running
            && !evidence.is_stale
        {
            return true;
        }

        if evidence.is_stale {
            self.record_merge_timeout_event(task, effective_age).await;
        }

        // Gap 1: Check retry count — escalate to MergeConflict after max retries.
        // Re-read task to get updated metadata after record_merge_timeout_event.
        let updated_task = match self.task_repo.get_by_id(&task.id).await {
            Ok(Some(t)) => t,
            Ok(None) => return false,
            Err(_) => return false,
        };
        let retry_count = Self::merging_auto_retry_count(&updated_task);
        if retry_count >= MERGING_MAX_AUTO_RETRIES {
            warn!(
                task_id = task.id.as_str(),
                retry_count = retry_count,
                max = MERGING_MAX_AUTO_RETRIES,
                "Merging retry limit reached — transitioning to MergeIncomplete for user retry"
            );
            // Use MergeIncomplete (not MergeConflict) because timeout indicates a hung agent,
            // not an explicit merge conflict. MergeIncomplete surfaces the task in the
            // needs_attention panel and allows auto-retry via reconcile_merge_incomplete_task.
            return self
                .apply_recovery_decision(
                    &updated_task,
                    status,
                    RecoveryContext::Merge,
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                        reason: Some(format!(
                            "Merger agent timed out {} times — transitioning to MergeIncomplete for retry",
                            retry_count
                        )),
                    },
                )
                .await;
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Merge, evidence);

        // Gap 2: Don't re-spawn agent if one is still running in registry
        if decision.action == RecoveryActionKind::ExecuteEntryActions && evidence.registry_running {
            warn!(
                task_id = task.id.as_str(),
                "Skipping merger agent re-spawn — agent still running in registry"
            );
            return false;
        }

        // Gap 3: After auto-complete, check if task is still stuck in Merging
        if decision.action == RecoveryActionKind::AttemptMergeAutoComplete {
            self.apply_recovery_decision(&updated_task, status, RecoveryContext::Merge, decision)
                .await;
            // Re-read to see if auto-complete transitioned the task
            if let Ok(Some(post_task)) = self.task_repo.get_by_id(&task.id).await {
                if post_task.internal_status == InternalStatus::Merging {
                    warn!(
                        task_id = task.id.as_str(),
                        "Auto-complete did not transition task out of Merging — will escalate on next timeout"
                    );
                }
            }
            return true;
        }

        self.apply_recovery_decision(&updated_task, status, RecoveryContext::Merge, decision)
            .await
    }

    async fn reconcile_qa_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::QaRefining && status != InternalStatus::QaTesting {
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let context = if status == InternalStatus::QaRefining {
            RecoveryContext::QaRefining
        } else {
            RecoveryContext::QaTesting
        };

        // C5: Wall-clock timeout for long-running QA
        let max_qa_minutes = QA_MAX_WALL_CLOCK_MINUTES;
        if age >= chrono::Duration::minutes(max_qa_minutes) {
            warn!(
                task_id = task.id.as_str(),
                age_minutes = age.num_minutes(),
                max_minutes = max_qa_minutes,
                "QA wall-clock timeout exceeded"
            );
            return self
                .apply_recovery_decision(
                    task,
                    status,
                    context,
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::QaFailed),
                        reason: Some(format!(
                            "QA timed out after {} minutes (wall-clock limit: {}m)",
                            age.num_minutes(),
                            max_qa_minutes
                        )),
                    },
                )
                .await;
        }

        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: self.execution_state.can_start_task(),
            is_stale: age >= chrono::Duration::minutes(QA_STALE_MINUTES),
            is_deferred: false,
        };
        let decision = self.policy.decide_reconciliation(context, evidence);

        // E7: Enforce retry limit for QA re-spawns
        if decision.action == RecoveryActionKind::ExecuteEntryActions {
            let retry_count = Self::auto_retry_count_for_status(task, status);
            if retry_count >= QA_MAX_AUTO_RETRIES {
                warn!(
                    task_id = task.id.as_str(),
                    retry_count = retry_count,
                    max = QA_MAX_AUTO_RETRIES,
                    "QA retry limit reached — escalating to QaFailed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        context,
                        RecoveryDecision {
                            action: RecoveryActionKind::Transition(InternalStatus::QaFailed),
                            reason: Some(format!(
                                "QA failed {} times — escalating to QaFailed",
                                retry_count
                            )),
                        },
                    )
                    .await;
            }
            if let Err(e) = self
                .record_auto_retry_metadata(task, status, retry_count + 1)
                .await
            {
                warn!(task_id = task.id.as_str(), error = %e, "Failed to record QA retry metadata");
            }
        }

        self.apply_recovery_decision(task, status, context, decision)
            .await
    }

    async fn reconcile_pending_merge_task(&self, task: &Task, status: InternalStatus) -> bool {
        use crate::domain::state_machine::transition_handler::{
            has_main_merge_deferred_metadata, has_merge_deferred_metadata,
        };

        if status != InternalStatus::PendingMerge {
            return false;
        }

        // Phase 4: Handle main-merge-deferred tasks (deferred because agents were running)
        // If no agents are running now, retry the merge. If agents are still running, skip.
        if has_main_merge_deferred_metadata(task) {
            let running_count = self.execution_state.running_count();
            if running_count == 0 {
                tracing::info!(
                    task_id = task.id.as_str(),
                    "Main-merge-deferred task ready to retry: all agents completed"
                );
                return self
                    .apply_recovery_decision(
                        task,
                        status,
                        RecoveryContext::PendingMerge,
                        RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: Some(
                                "Main merge deferred while agents running — now retrying.".to_string(),
                            ),
                        },
                    )
                    .await;
            } else {
                tracing::debug!(
                    task_id = task.id.as_str(),
                    running_count = running_count,
                    "Main-merge-deferred task waiting for global idle: agents still running"
                );
                // Task is correctly deferred, not orphaned — skip reconciliation
                return true;
            }
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let is_deferred = has_merge_deferred_metadata(task);

        // Deferred-orphan watchdog: if the recorded blocker is no longer active
        // (missing, archived, or no longer in merge workflow), immediately re-trigger
        // entry actions instead of waiting for stale timeout.
        if is_deferred {
            if let Some(blocker_id) = self.latest_deferred_blocker_id(task) {
                if !self.deferred_blocker_is_active(&blocker_id).await {
                    return self
                        .apply_recovery_decision(
                            task,
                            status,
                            RecoveryContext::PendingMerge,
                            RecoveryDecision {
                                action: RecoveryActionKind::ExecuteEntryActions,
                                reason: Some(
                                    "Deferred merge blocker is no longer active — re-triggering."
                                        .to_string(),
                                ),
                            },
                        )
                        .await;
                }
            }
        }

        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: true,
            is_stale: age >= chrono::Duration::minutes(PENDING_MERGE_STALE_MINUTES),
            is_deferred,
        };
        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::PendingMerge, evidence);

        self.apply_recovery_decision(task, status, RecoveryContext::PendingMerge, decision)
            .await
    }

    async fn reconcile_merge_incomplete_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::MergeIncomplete {
            return false;
        }

        // Skip retry when branch_missing flag is set - surface to user instead
        if has_branch_missing_metadata(task) {
            return false;
        }

        // Smart retry guard: if the incomplete was explicitly reported by the agent,
        // it was a deliberate decision — do NOT auto-retry without human intervention.
        if Self::is_agent_reported_failure(task) {
            warn!(
                task_id = task.id.as_str(),
                "Skipping auto-retry of MergeIncomplete — agent explicitly reported this failure (AgentReported)"
            );
            return false;
        }

        // Loop-breaking guard: if validation has reverted the merge >VALIDATION_REVERT_MAX_COUNT
        // times, stop auto-retrying and surface to user — the code changes must fix the failures.
        let revert_count = Self::validation_revert_count(task);
        if revert_count > VALIDATION_REVERT_MAX_COUNT {
            warn!(
                task_id = task.id.as_str(),
                revert_count = revert_count,
                max = VALIDATION_REVERT_MAX_COUNT,
                "Stopping auto-retry of MergeIncomplete — validation revert loop detected (ValidationFailed)"
            );
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let retry_count = Self::merge_incomplete_auto_retry_count(task);
        if retry_count >= MERGE_INCOMPLETE_MAX_AUTO_RETRIES {
            return false;
        }

        let retry_delay = Self::merge_incomplete_retry_delay(retry_count);
        if age < retry_delay {
            return false;
        }

        let attempt = retry_count + 1;
        if let Err(e) = self
            .record_merge_auto_retry_event(
                task,
                attempt,
                MergeFailureSource::TransientGit,
                "MergeIncomplete auto-retry — transient git failure",
            )
            .await
        {
            warn!(
                task_id = task.id.as_str(),
                attempt = attempt,
                error = %e,
                "Failed to record merge auto-retry metadata"
            );
        }

        warn!(
            task_id = task.id.as_str(),
            attempt = attempt,
            failure_source = "transient_git",
            "Auto-retrying MergeIncomplete — transient git failure, transitioning to PendingMerge"
        );

        match self
            .transition_service
            .transition_task(&task.id, InternalStatus::PendingMerge)
            .await
        {
            Ok(_) => true,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to transition MergeIncomplete -> PendingMerge during recovery"
                );
                false
            }
        }
    }

    async fn reconcile_merge_conflict_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::MergeConflict {
            return false;
        }

        // Skip retry when branch_missing flag is set - surface to user instead
        if has_branch_missing_metadata(task) {
            return false;
        }

        // Smart retry guard: if the conflict was explicitly reported by the agent,
        // the agent made a deliberate decision — do NOT auto-retry without human intervention.
        if Self::is_agent_reported_failure(task) {
            warn!(
                task_id = task.id.as_str(),
                "Skipping auto-retry of MergeConflict — agent explicitly reported this conflict (AgentReported)"
            );
            return false;
        }

        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };

        let retry_count = Self::merge_conflict_auto_retry_count(task);
        if retry_count >= MERGE_CONFLICT_MAX_AUTO_RETRIES {
            return false;
        }

        let retry_delay = Self::merge_conflict_retry_delay(retry_count);
        if age < retry_delay {
            return false;
        }

        // SHA comparison guard: if source branch hasn't changed since last failure,
        // retrying the same commit will produce the same conflict — skip.
        let sha_changed = self.check_source_sha_changed(task).await;
        match sha_changed {
            ShaComparisonResult::Unchanged(sha) => {
                warn!(
                    task_id = task.id.as_str(),
                    source_sha = %sha,
                    "Skipping MergeConflict auto-retry — source branch SHA unchanged since last failure"
                );
                return false;
            }
            ShaComparisonResult::Changed { old_sha, new_sha } => {
                warn!(
                    task_id = task.id.as_str(),
                    old_sha = %old_sha,
                    new_sha = %new_sha,
                    "Source branch SHA changed since last failure — proceeding with retry"
                );
            }
            ShaComparisonResult::NoStoredSha | ShaComparisonResult::GitError => {
                // No SHA stored (first attempt) or git error — allow retry
            }
        }

        // Get current source SHA to record with the retry event
        let current_sha = self.get_current_source_sha(task).await;

        let attempt = retry_count + 1;
        let failure_source = MergeFailureSource::SystemDetected;
        if let Err(e) = self
            .record_merge_auto_retry_event_with_sha(
                task,
                attempt,
                failure_source,
                "MergeConflict auto-retry — system-detected conflict",
                current_sha.as_deref(),
            )
            .await
        {
            warn!(
                task_id = task.id.as_str(),
                attempt = attempt,
                error = %e,
                "Failed to record merge auto-retry metadata"
            );
        }

        warn!(
            task_id = task.id.as_str(),
            attempt = attempt,
            failure_source = "system_detected",
            source_sha = current_sha.as_deref().unwrap_or("unknown"),
            "Auto-retrying MergeConflict — system-detected conflict, transitioning to PendingMerge"
        );

        match self
            .transition_service
            .transition_task(&task.id, InternalStatus::PendingMerge)
            .await
        {
            Ok(_) => true,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to transition MergeConflict -> PendingMerge during recovery"
                );
                false
            }
        }
    }

    /// Auto-resume paused tasks that were paused due to recoverable provider errors.
    ///
    /// Checks pause_reason metadata in task.metadata (with backward compat for provider_error):
    /// - UserInitiated → skip (user-paused tasks stay paused until user resumes)
    /// - ProviderError with retry_after passed and resume_attempts < MAX → resume via entry actions
    /// - ProviderError with max attempts exceeded → transition to Failed
    /// - No metadata → skip (user-paused task)
    async fn reconcile_paused_provider_error(&self, task: &Task) -> bool {
        use crate::application::chat_service::{PauseReason, ProviderErrorMetadata};

        // Read from new pause_reason key (with backward compat for old provider_error key)
        let pause_reason = match PauseReason::from_task_metadata(task.metadata.as_deref()) {
            Some(reason) => reason,
            None => {
                // Also try legacy ProviderErrorMetadata directly
                match ProviderErrorMetadata::from_task_metadata(task.metadata.as_deref()) {
                    Some(_) => {
                        // Fall through to legacy handling below
                        return self.reconcile_paused_provider_error_legacy(task).await;
                    }
                    None => return false, // No metadata — user-paused, skip
                }
            }
        };

        // UserInitiated pauses should NOT be auto-resumed
        let (category, message, retry_after, previous_status, auto_resumable, resume_attempts) =
            match pause_reason {
                PauseReason::UserInitiated { .. } => return false,
                PauseReason::ProviderError {
                    category,
                    message,
                    retry_after,
                    previous_status,
                    auto_resumable,
                    resume_attempts,
                    ..
                } => (category, message, retry_after, previous_status, auto_resumable, resume_attempts),
            };

        if !auto_resumable {
            return false;
        }

        // Check if max resume attempts exceeded → transition to Failed
        if resume_attempts >= ProviderErrorMetadata::MAX_RESUME_ATTEMPTS {
            warn!(
                task_id = task.id.as_str(),
                attempts = resume_attempts,
                max = ProviderErrorMetadata::MAX_RESUME_ATTEMPTS,
                category = %category,
                "Provider error auto-resume limit reached — transitioning to Failed"
            );
            // Clear pause metadata and transition to Failed
            let mut updated = task.clone();
            updated.metadata = Some(PauseReason::clear_from_task_metadata(
                updated.metadata.as_deref(),
            ));
            updated.touch();
            let _ = self.task_repo.update(&updated).await;

            let _ = self
                .transition_service
                .transition_task(&task.id, InternalStatus::Failed)
                .await;
            return true;
        }

        // Build a ProviderErrorMetadata to check retry eligibility
        let meta = ProviderErrorMetadata {
            category: category.clone(),
            message: message.clone(),
            retry_after: retry_after.clone(),
            previous_status: previous_status.clone(),
            paused_at: String::new(),
            auto_resumable,
            resume_attempts,
        };

        // Check if retry_after time has passed
        if !meta.is_retry_eligible() {
            return false; // Not eligible yet — wait for retry_after
        }

        // Can we start a task right now?
        if !self.execution_state.can_start_task() {
            return false; // At max concurrency — retry on next reconciliation cycle
        }

        // Increment resume attempts
        let updated_reason = PauseReason::ProviderError {
            category: category.clone(),
            message,
            retry_after,
            previous_status: previous_status.clone(),
            paused_at: chrono::Utc::now().to_rfc3339(),
            auto_resumable,
            resume_attempts: resume_attempts + 1,
        };
        let mut updated_task = task.clone();
        updated_task.metadata = Some(
            updated_reason.write_to_task_metadata(updated_task.metadata.as_deref()),
        );
        updated_task.touch();
        if let Err(e) = self.task_repo.update(&updated_task).await {
            warn!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to update resume attempt count"
            );
            return false;
        }

        // Determine the target state to resume to (from previous_status)
        let resume_status = match previous_status.as_str() {
            "executing" => InternalStatus::Executing,
            "re_executing" => InternalStatus::ReExecuting,
            "qa_refining" => InternalStatus::QaRefining,
            "qa_testing" => InternalStatus::QaTesting,
            "reviewing" => InternalStatus::Reviewing,
            "merging" => InternalStatus::Merging,
            _ => InternalStatus::Executing, // Safe default
        };

        tracing::info!(
            task_id = task.id.as_str(),
            category = %category,
            resume_status = %resume_status,
            attempt = resume_attempts + 1,
            "Auto-resuming provider-error-paused task"
        );

        // Emit event for frontend
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit(
                "task:provider_error_resuming",
                serde_json::json!({
                    "task_id": task.id.as_str(),
                    "category": category.to_string(),
                    "resume_status": resume_status.to_string(),
                    "attempt": resume_attempts + 1,
                }),
            );
        }

        // Transition back to the previous active state
        // The TransitionHandler's on_enter will re-spawn the agent
        match self
            .transition_service
            .transition_task(&task.id, resume_status)
            .await
        {
            Ok(_) => {
                // Clear pause metadata on successful resume
                let mut cleared_task = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(t)) => t,
                    _ => return true,
                };
                cleared_task.metadata = Some(PauseReason::clear_from_task_metadata(
                    cleared_task.metadata.as_deref(),
                ));
                cleared_task.touch();
                let _ = self.task_repo.update(&cleared_task).await;
                true
            }
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    resume_status = %resume_status,
                    "Failed to auto-resume provider-error-paused task"
                );
                false
            }
        }
    }

    /// Legacy handler for tasks with old provider_error key (backward compat).
    async fn reconcile_paused_provider_error_legacy(&self, task: &Task) -> bool {
        use crate::application::chat_service::ProviderErrorMetadata;

        let meta = match ProviderErrorMetadata::from_task_metadata(task.metadata.as_deref()) {
            Some(meta) => meta,
            None => return false,
        };

        if !meta.auto_resumable {
            return false;
        }

        if meta.resume_attempts >= ProviderErrorMetadata::MAX_RESUME_ATTEMPTS {
            warn!(
                task_id = task.id.as_str(),
                attempts = meta.resume_attempts,
                "Legacy provider error auto-resume limit reached — transitioning to Failed"
            );
            let mut updated = task.clone();
            updated.metadata = Some(ProviderErrorMetadata::clear_from_task_metadata(
                updated.metadata.as_deref(),
            ));
            updated.touch();
            let _ = self.task_repo.update(&updated).await;
            let _ = self
                .transition_service
                .transition_task(&task.id, InternalStatus::Failed)
                .await;
            return true;
        }

        if !meta.is_retry_eligible() {
            return false;
        }

        if !self.execution_state.can_start_task() {
            return false;
        }

        let mut updated_meta = meta.clone();
        updated_meta.resume_attempts += 1;
        let mut updated_task = task.clone();
        updated_task.metadata = Some(
            updated_meta.write_to_task_metadata(updated_task.metadata.as_deref()),
        );
        updated_task.touch();
        if let Err(e) = self.task_repo.update(&updated_task).await {
            warn!(task_id = task.id.as_str(), error = %e, "Failed to update legacy resume attempts");
            return false;
        }

        let resume_status = match meta.previous_status.as_str() {
            "executing" => InternalStatus::Executing,
            "re_executing" => InternalStatus::ReExecuting,
            "qa_refining" => InternalStatus::QaRefining,
            "qa_testing" => InternalStatus::QaTesting,
            "reviewing" => InternalStatus::Reviewing,
            "merging" => InternalStatus::Merging,
            _ => InternalStatus::Executing,
        };

        match self.transition_service.transition_task(&task.id, resume_status).await {
            Ok(_) => {
                let mut cleared = match self.task_repo.get_by_id(&task.id).await {
                    Ok(Some(t)) => t,
                    _ => return true,
                };
                cleared.metadata = Some(ProviderErrorMetadata::clear_from_task_metadata(
                    cleared.metadata.as_deref(),
                ));
                cleared.touch();
                let _ = self.task_repo.update(&cleared).await;
                true
            }
            Err(e) => {
                warn!(task_id = task.id.as_str(), error = %e, "Failed legacy auto-resume");
                false
            }
        }
    }

    pub async fn recover_execution_stop(&self, task_id: &TaskId) -> bool {
        let task = match self.task_repo.get_by_id(task_id).await {
            Ok(Some(task)) => task,
            Ok(None) => return false,
            Err(e) => {
                warn!(task_id = task_id.as_str(), error = %e, "Failed to load task for stop recovery");
                return false;
            }
        };

        if task.internal_status != InternalStatus::Executing
            && task.internal_status != InternalStatus::ReExecuting
        {
            return false;
        }

        let key = crate::domain::services::RunningAgentKey::new(
            ChatContextType::TaskExecution.to_string(),
            task.id.as_str(),
        );

        let registry_running = self.running_agent_registry.is_running(&key).await;
        if registry_running {
            let _ = self.running_agent_registry.stop(&key).await;
        }

        let run = self.load_execution_run(&task, task.internal_status).await;
        let evidence = RecoveryEvidence {
            run_status: run.as_ref().map(|r| r.status),
            registry_running,
            can_start: self.execution_state.can_start_task(),
            is_stale: false,
            is_deferred: false,
        };
        let decision = self.policy.decide_execution_stop(evidence);

        self.apply_recovery_decision(
            &task,
            task.internal_status,
            RecoveryContext::Execution,
            decision,
        )
        .await
    }

    pub async fn apply_user_recovery_action(
        &self,
        task: &Task,
        action: UserRecoveryAction,
    ) -> bool {
        let status = task.internal_status;
        let decision = match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => RecoveryDecision {
                    action: RecoveryActionKind::Transition(InternalStatus::Ready),
                    reason: None,
                },
            },
            InternalStatus::PendingMerge => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => RecoveryDecision {
                    action: RecoveryActionKind::Transition(InternalStatus::MergeIncomplete),
                    reason: None,
                },
            },
            InternalStatus::Reviewing
            | InternalStatus::Merging
            | InternalStatus::QaRefining
            | InternalStatus::QaTesting => match action {
                UserRecoveryAction::Restart => RecoveryDecision {
                    action: RecoveryActionKind::ExecuteEntryActions,
                    reason: None,
                },
                UserRecoveryAction::Cancel => {
                    let next_status = match status {
                        InternalStatus::Reviewing => InternalStatus::Escalated,
                        InternalStatus::Merging => InternalStatus::MergeConflict,
                        InternalStatus::QaRefining | InternalStatus::QaTesting => {
                            InternalStatus::QaFailed
                        }
                        _ => InternalStatus::Escalated,
                    };
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(next_status),
                        reason: None,
                    }
                }
            },
            _ => return false,
        };

        self.clear_prompt_marker(task.id.as_str(), status).await;

        let context = match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => RecoveryContext::Execution,
            InternalStatus::Reviewing => RecoveryContext::Review,
            InternalStatus::Merging => RecoveryContext::Merge,
            InternalStatus::PendingMerge => RecoveryContext::PendingMerge,
            InternalStatus::QaRefining => RecoveryContext::QaRefining,
            InternalStatus::QaTesting => RecoveryContext::QaTesting,
            _ => return false,
        };

        self.apply_recovery_decision(task, status, context, decision)
            .await
    }

    async fn apply_recovery_decision(
        &self,
        task: &Task,
        status: InternalStatus,
        context: RecoveryContext,
        decision: RecoveryDecision,
    ) -> bool {
        match decision.action {
            RecoveryActionKind::None => false,
            RecoveryActionKind::ExecuteEntryActions => {
                // Set trigger_origin="recovery" before resuming agent
                let mut task_mut = task.clone();
                set_trigger_origin(&mut task_mut, "recovery");
                if let Err(e) = self.task_repo.update(&task_mut).await {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to set trigger_origin=recovery in metadata"
                    );
                }

                self.transition_service
                    .execute_entry_actions(&task.id, task, status)
                    .await;
                true
            }
            RecoveryActionKind::Transition(next_status) => {
                if let Err(e) = self
                    .transition_service
                    .transition_task(&task.id, next_status)
                    .await
                {
                    tracing::error!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task during recovery"
                    );
                    return false;
                }
                true
            }
            RecoveryActionKind::AttemptMergeAutoComplete => {
                reconcile_merge_auto_complete(
                    task.id.as_str(),
                    &self.task_repo,
                    &self.task_dep_repo,
                    &self.project_repo,
                    &self.chat_message_repo,
                    &self.chat_attachment_repo,
                    &self.chat_conversation_repo,
                    &self.agent_run_repo,
                    &self.ideation_session_repo,
                    &self.activity_event_repo,
                    &self.message_queue,
                    &self.running_agent_registry,
                    &self.memory_event_repo,
                    &self.execution_state,
                    &self.plan_branch_repo,
                    self.app_handle.as_ref(),
                )
                .await;
                true
            }
            RecoveryActionKind::Prompt => {
                let reason = decision
                    .reason
                    .unwrap_or_else(|| "Recovery decision requires user input.".to_string());
                self.emit_recovery_prompt(task, status, context, reason)
                    .await
            }
        }
    }

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
        let exponent = retry_count.min(6);
        let scaled = MERGE_INCOMPLETE_AUTO_RETRY_BASE_SECONDS.saturating_mul(1_i64 << exponent);
        chrono::Duration::seconds(scaled.min(MERGE_INCOMPLETE_AUTO_RETRY_MAX_SECONDS))
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
        let exponent = retry_count.min(6);
        let scaled = MERGE_CONFLICT_AUTO_RETRY_BASE_SECONDS.saturating_mul(1_i64 << exponent);
        chrono::Duration::seconds(scaled.min(MERGE_CONFLICT_AUTO_RETRY_MAX_SECONDS))
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

        let timeout_secs = merging_timeout_seconds();
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

fn context_matches_task_status(context_type: ChatContextType, status: InternalStatus) -> bool {
    match context_type {
        ChatContextType::TaskExecution => {
            status == InternalStatus::Executing || status == InternalStatus::ReExecuting
        }
        ChatContextType::Review => status == InternalStatus::Reviewing,
        ChatContextType::Merge => status == InternalStatus::Merging,
        ChatContextType::Task | ChatContextType::Ideation | ChatContextType::Project => {
            AGENT_ACTIVE_STATUSES.contains(&status)
        }
    }
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }

    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .map(|output| {
                if !output.status.success() {
                    return true;
                }
                let text = String::from_utf8_lossy(&output.stdout);
                !text.to_ascii_lowercase().contains("no tasks are running")
            })
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests;
