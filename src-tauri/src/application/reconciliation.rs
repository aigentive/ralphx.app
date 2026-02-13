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

use crate::application::{chat_service::reconcile_merge_auto_complete, TaskTransitionService};
use crate::commands::execution_commands::{ExecutionState, AGENT_ACTIVE_STATUSES};
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus,
    MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
    MergeRecoverySource, MergeRecoveryState, Task, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, MemoryEventRepository, PlanBranchRepository, ProjectRepository,
    TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::transition_handler::{has_branch_missing_metadata, set_trigger_origin};

const MERGING_TIMEOUT_SECONDS: i64 = 300;
const MERGING_MAX_AUTO_RETRIES: u32 = 3;
const PENDING_MERGE_STALE_MINUTES: i64 = 5;
const QA_STALE_MINUTES: i64 = 5;
const MERGE_INCOMPLETE_AUTO_RETRY_BASE_SECONDS: i64 = 30;
const MERGE_INCOMPLETE_AUTO_RETRY_MAX_SECONDS: i64 = 300;
const MERGE_INCOMPLETE_MAX_AUTO_RETRIES: u32 = 5;

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

pub struct ReconciliationRunner<R: Runtime = tauri::Wry> {
    task_repo: Arc<dyn TaskRepository>,
    task_dep_repo: Arc<dyn TaskDependencyRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
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
                InternalStatus::QaRefining,
                InternalStatus::QaTesting,
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
            InternalStatus::QaRefining | InternalStatus::QaTesting => {
                self.reconcile_qa_task(task, status).await
            }
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

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Execution, evidence);

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

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Review, evidence);

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
        let age = match self.latest_status_transition_age(task, status).await {
            Some(age) => age,
            None => return false,
        };
        let mut evidence = self
            .build_run_evidence(task, ChatContextType::Merge, run.as_ref())
            .await;
        evidence.is_stale = age >= chrono::Duration::seconds(MERGING_TIMEOUT_SECONDS);

        // Agent is running, registered, and not stale — let it work
        if evidence.run_status == Some(AgentRunStatus::Running)
            && evidence.registry_running
            && !evidence.is_stale
        {
            return true;
        }

        if evidence.is_stale {
            self.record_merge_timeout_event(task, age).await;
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
                "Merging retry limit reached — escalating to MergeConflict"
            );
            return self
                .apply_recovery_decision(
                    &updated_task,
                    status,
                    RecoveryContext::Merge,
                    RecoveryDecision {
                        action: RecoveryActionKind::Transition(InternalStatus::MergeConflict),
                        reason: Some(format!(
                            "Merge failed {} times — escalating to MergeConflict for manual resolution",
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
            self.apply_recovery_decision(
                &updated_task,
                status,
                RecoveryContext::Merge,
                decision,
            )
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

        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: self.execution_state.can_start_task(),
            is_stale: age >= chrono::Duration::minutes(QA_STALE_MINUTES),
            is_deferred: false,
        };
        let context = if status == InternalStatus::QaRefining {
            RecoveryContext::QaRefining
        } else {
            RecoveryContext::QaTesting
        };
        let decision = self.policy.decide_reconciliation(context, evidence);

        self.apply_recovery_decision(task, status, context, decision)
            .await
    }

    async fn reconcile_pending_merge_task(&self, task: &Task, status: InternalStatus) -> bool {
        use crate::domain::state_machine::transition_handler::has_merge_deferred_metadata;

        if status != InternalStatus::PendingMerge {
            return false;
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
        if let Err(e) = self.record_merge_auto_retry_event(task, attempt).await {
            warn!(
                task_id = task.id.as_str(),
                attempt = attempt,
                error = %e,
                "Failed to record merge auto-retry metadata"
            );
        }

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

    async fn record_merge_auto_retry_event(&self, task: &Task, attempt: u32) -> Result<(), String> {
        let mut updated = task.clone();
        let mut recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
            .unwrap_or(None)
            .unwrap_or_default();

        let mut event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::GitError,
            format!(
                "Auto-retry triggered after MergeIncomplete (attempt {})",
                attempt
            ),
        )
        .with_attempt(attempt);

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

        self.task_repo.update(&updated).await.map_err(|e| e.to_string())
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
        );
        recovery.append_event_with_state(failed_event, MergeRecoveryState::Failed);

        let mut metadata = match recovery.update_task_metadata(updated.metadata.as_deref()) {
            Ok(json) => serde_json::from_str::<serde_json::Value>(&json)
                .unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        };

        if let Some(obj) = metadata.as_object_mut() {
            obj.insert(
                "error".to_string(),
                serde_json::json!(format!(
                    "Merge timed out after {}s without complete_merge callback",
                    MERGING_TIMEOUT_SECONDS
                )),
            );
            obj.insert(
                "merge_timeout_seconds".to_string(),
                serde_json::json!(MERGING_TIMEOUT_SECONDS),
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
