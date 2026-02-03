// Reconciliation runner for agent-active task states
//
// Ensures tasks don't get stuck when agent runs finish without transitions.
// Can be used on startup and during runtime polling.

use std::collections::HashSet;
use std::sync::Arc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::warn;
use tokio::sync::Mutex;

use crate::application::{chat_service::reconcile_merge_auto_complete, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus, Task, TaskId,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryContext {
    Execution,
    Review,
    Merge,
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
                if evidence.run_status.is_none() {
                    if evidence.can_start {
                        return RecoveryDecision {
                            action: RecoveryActionKind::ExecuteEntryActions,
                            reason: None,
                        };
                    }
                    return RecoveryDecision {
                        action: RecoveryActionKind::Prompt,
                        reason: Some("Merge run missing but max concurrency is reached.".to_string()),
                    };
                }
                RecoveryDecision {
                    action: RecoveryActionKind::None,
                    reason: None,
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
                        reason: Some("QA task is stale but max concurrency is reached.".to_string()),
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
    running_agent_registry: Arc<RunningAgentRegistry>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    transition_service: Arc<TaskTransitionService<R>>,
    execution_state: Arc<ExecutionState>,
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
        running_agent_registry: Arc<RunningAgentRegistry>,
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
            agent_run_repo,
            transition_service,
            execution_state,
            app_handle,
            policy: RecoveryPolicy::default(),
            prompt_tracker: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    pub async fn reconcile_stuck_tasks(&self) {
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

    pub async fn reconcile_task(&self, task: &Task, status: InternalStatus) -> bool {
        match status {
            InternalStatus::Executing | InternalStatus::ReExecuting => {
                self.reconcile_completed_execution(task, status).await
            }
            InternalStatus::Reviewing => self.reconcile_reviewing_task(task, status).await,
            InternalStatus::Merging => self.reconcile_merging_task(task, status).await,
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

        let run = self
            .load_execution_run(task, status)
            .await;
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
        let evidence = self
            .build_run_evidence(task, ChatContextType::Merge, run.as_ref())
            .await;
        if evidence.run_status == Some(AgentRunStatus::Running) && evidence.registry_running {
            return true;
        }

        let decision = self
            .policy
            .decide_reconciliation(RecoveryContext::Merge, evidence);

        self.apply_recovery_decision(task, status, RecoveryContext::Merge, decision)
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
            is_stale: age >= chrono::Duration::minutes(5),
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

        let run = self
            .load_execution_run(&task, task.internal_status)
            .await;
        let evidence = RecoveryEvidence {
            run_status: run.as_ref().map(|r| r.status),
            registry_running,
            can_start: self.execution_state.can_start_task(),
            is_stale: false,
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
                    &self.execution_state,
                    self.app_handle.as_ref(),
                )
                .await;
                true
            }
            RecoveryActionKind::Prompt => {
                let reason = decision.reason.unwrap_or_else(|| {
                    "Recovery decision requires user input.".to_string()
                });
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
        }
    }

    async fn load_execution_run(
        &self,
        task: &Task,
        status: InternalStatus,
    ) -> Option<AgentRun> {
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
            let Some(agent_run_id) = transition.agent_run_id.as_ref() else {
                return None;
            };

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
        let status_history = self.task_repo.get_status_history(&task.id).await.ok()?;
        let latest_transition = status_history
            .iter()
            .rev()
            .find(|transition| transition.to == status)?;
        Some(chrono::Utc::now() - latest_transition.timestamp)
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
mod tests {
    use super::*;

    #[test]
    fn execution_policy_advances_on_completed_run() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: Some(AgentRunStatus::Completed),
            registry_running: false,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
        assert_eq!(
            decision.action,
            RecoveryActionKind::Transition(InternalStatus::PendingReview)
        );
    }

    #[test]
    fn execution_policy_restarts_when_run_missing() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
        assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    }

    #[test]
    fn execution_policy_prompts_on_conflict() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: Some(AgentRunStatus::Running),
            registry_running: false,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::Execution, evidence);
        assert_eq!(decision.action, RecoveryActionKind::Prompt);
    }

    #[test]
    fn review_policy_restarts_on_completed_run() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: Some(AgentRunStatus::Completed),
            registry_running: false,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::Review, evidence);
        assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    }

    #[test]
    fn merge_policy_verifies_on_completed_run() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: Some(AgentRunStatus::Completed),
            registry_running: false,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::Merge, evidence);
        assert_eq!(decision.action, RecoveryActionKind::AttemptMergeAutoComplete);
    }

    #[test]
    fn qa_policy_retries_when_stale() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: None,
            registry_running: false,
            can_start: true,
            is_stale: true,
        };

        let decision = policy.decide_reconciliation(RecoveryContext::QaTesting, evidence);
        assert_eq!(decision.action, RecoveryActionKind::ExecuteEntryActions);
    }

    #[test]
    fn stop_policy_resets_when_not_completed() {
        let policy = RecoveryPolicy::default();
        let evidence = RecoveryEvidence {
            run_status: Some(AgentRunStatus::Running),
            registry_running: true,
            can_start: true,
            is_stale: false,
        };

        let decision = policy.decide_execution_stop(evidence);
        assert_eq!(
            decision.action,
            RecoveryActionKind::Transition(InternalStatus::Ready)
        );
    }
}
