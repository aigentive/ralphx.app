// Reconciliation runner for agent-active task states
//
// Ensures tasks don't get stuck when agent runs finish without transitions.
// Can be used on startup and during runtime polling.

use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tracing::warn;

use crate::application::{chat_service::reconcile_merge_auto_complete, TaskTransitionService};
use crate::commands::execution_commands::ExecutionState;
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus, Task,
};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};

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

        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to load status history for reconciliation"
                );
                return false;
            }
        };

        let latest_transition = status_history
            .iter()
            .rev()
            .find(|transition| transition.to == status && transition.agent_run_id.is_some());

        let run = if let Some(transition) = latest_transition {
            let Some(agent_run_id) = transition.agent_run_id.as_ref() else {
                return false;
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
                    return false;
                }
            }
        } else {
            self.lookup_latest_run_for_task_context(task, ChatContextType::TaskExecution)
                .await
        };

        let Some(run) = run else {
            if !self.execution_state.can_start_task() {
                return false;
            }

            warn!(
                task_id = task.id.as_str(),
                "No agent run metadata found; re-triggering entry actions"
            );
            self.transition_service
                .execute_entry_actions(&task.id, task, status)
                .await;
            return true;
        };

        if run.status == AgentRunStatus::Running {
            return true;
        }

        if run.status != AgentRunStatus::Completed {
            return false;
        }

        tracing::info!(
            task_id = task.id.as_str(),
            run_id = run.id.as_str(),
            "Reconciling completed execution - transitioning to PendingReview"
        );

        if let Err(e) = self
            .transition_service
            .transition_task(&task.id, InternalStatus::PendingReview)
            .await
        {
            tracing::error!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to reconcile completed execution"
            );
            return false;
        }

        true
    }

    async fn reconcile_reviewing_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Reviewing {
            return false;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Review)
            .await;

        let Some(run) = run else {
            if !self.execution_state.can_start_task() {
                return false;
            }

            warn!(
                task_id = task.id.as_str(),
                "No review agent run found; re-triggering entry actions"
            );
            self.transition_service
                .execute_entry_actions(&task.id, task, status)
                .await;
            return true;
        };

        if run.status == AgentRunStatus::Running {
            return true;
        }

        if run.status != AgentRunStatus::Completed {
            return false;
        }

        tracing::info!(
            task_id = task.id.as_str(),
            run_id = run.id.as_str(),
            "Review run completed without status change; escalating for human decision"
        );

        if let Err(e) = self
            .transition_service
            .transition_task(&task.id, InternalStatus::Escalated)
            .await
        {
            tracing::error!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to reconcile completed review"
            );
            return false;
        }

        true
    }

    async fn reconcile_merging_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Merging {
            return false;
        }

        let run = self
            .lookup_latest_run_for_task_context(task, ChatContextType::Merge)
            .await;

        let Some(run) = run else {
            if !self.execution_state.can_start_task() {
                return false;
            }

            warn!(
                task_id = task.id.as_str(),
                "No merge agent run found; re-triggering entry actions"
            );
            self.transition_service
                .execute_entry_actions(&task.id, task, status)
                .await;
            return true;
        };

        if run.status == AgentRunStatus::Running {
            return true;
        }

        if run.status != AgentRunStatus::Completed {
            return false;
        }

        tracing::info!(
            task_id = task.id.as_str(),
            run_id = run.id.as_str(),
            "Merge run completed without status change; reconciling"
        );

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

    async fn reconcile_qa_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::QaRefining && status != InternalStatus::QaTesting {
            return false;
        }

        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                warn!(
                    task_id = task.id.as_str(),
                    error = %e,
                    "Failed to load status history for QA reconciliation"
                );
                return false;
            }
        };

        let latest_transition = status_history
            .iter()
            .rev()
            .find(|transition| transition.to == status);

        let Some(transition) = latest_transition else {
            return false;
        };

        let age = chrono::Utc::now() - transition.timestamp;
        if age < chrono::Duration::minutes(5) {
            return false;
        }

        if !self.execution_state.can_start_task() {
            return false;
        }

        warn!(
            task_id = task.id.as_str(),
            status = ?status,
            "QA task appears stuck; re-triggering entry actions"
        );

        self.transition_service
            .execute_entry_actions(&task.id, task, status)
            .await;

        true
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
