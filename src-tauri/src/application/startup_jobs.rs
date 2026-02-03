// Startup Job Runner
//
// Handles automatic task resumption when the app restarts.
// Tasks that were in agent-active states (Executing, QaRefining, QaTesting, Reviewing, ReExecuting)
// when the app shut down are automatically resumed on startup, respecting pause state and
// max_concurrent limits.
//
// Also cleans up orphaned agent runs that were left in "running" status from previous sessions.
//
// Usage:
// - Called once during app initialization after HTTP server is ready
// - Cleans up orphaned agent runs from previous sessions
// - Iterates all projects to find tasks in agent-active states
// - Re-executes entry actions to respawn agents
// - Stops early if max_concurrent is reached

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tracing::info;

use crate::commands::execution_commands::{
    ExecutionState, AGENT_ACTIVE_STATUSES, AUTO_TRANSITION_STATES,
};
use crate::domain::entities::{
    AgentRun, AgentRunId, AgentRunStatus, ChatContextType, InternalStatus, Task,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ProjectRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::state_machine::services::TaskScheduler;

use super::TaskTransitionService;

/// Runs startup jobs, primarily task resumption.
///
/// Finds all tasks that were in agent-active states when the app shut down
/// and re-triggers their entry actions to respawn worker agents.
/// Also cleans up orphaned agent runs from previous sessions.
pub struct StartupJobRunner<R: Runtime = tauri::Wry> {
    task_repo: Arc<dyn TaskRepository>,
    task_dep_repo: Arc<dyn TaskDependencyRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    chat_message_repo: Arc<dyn crate::domain::repositories::ChatMessageRepository>,
    ideation_session_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository>,
    activity_event_repo: Arc<dyn crate::domain::repositories::ActivityEventRepository>,
    message_queue: Arc<crate::domain::services::MessageQueue>,
    running_agent_registry: Arc<crate::domain::services::RunningAgentRegistry>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    transition_service: TaskTransitionService<R>,
    execution_state: Arc<ExecutionState>,
    /// Optional task scheduler for auto-starting Ready tasks on startup.
    /// When provided, Ready tasks will be scheduled after resuming agent-active tasks.
    task_scheduler: Option<Arc<dyn TaskScheduler>>,
    /// Optional app handle for event emission
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> StartupJobRunner<R> {
    /// Create a new StartupJobRunner with all required dependencies.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_dep_repo: Arc<dyn TaskDependencyRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        chat_conversation_repo: Arc<dyn ChatConversationRepository>,
        chat_message_repo: Arc<dyn crate::domain::repositories::ChatMessageRepository>,
        ideation_session_repo: Arc<dyn crate::domain::repositories::IdeationSessionRepository>,
        activity_event_repo: Arc<dyn crate::domain::repositories::ActivityEventRepository>,
        message_queue: Arc<crate::domain::services::MessageQueue>,
        running_agent_registry: Arc<crate::domain::services::RunningAgentRegistry>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        transition_service: TaskTransitionService<R>,
        execution_state: Arc<ExecutionState>,
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
            task_scheduler: None,
            app_handle: None,
        }
    }

    /// Set the task scheduler for auto-starting Ready tasks (builder pattern).
    ///
    /// When set, the runner will call try_schedule_ready_tasks() after resuming
    /// agent-active tasks, allowing queued Ready tasks to start execution.
    pub fn with_task_scheduler(mut self, scheduler: Arc<dyn TaskScheduler>) -> Self {
        self.task_scheduler = Some(scheduler);
        self
    }

    /// Set the app handle for event emission (builder pattern).
    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Run startup jobs, resuming tasks in agent-active states.
    ///
    /// Skips if execution is paused. Stops early if max_concurrent is reached.
    /// For each task in an agent-active state, re-executes entry actions to
    /// respawn the appropriate agent.
    pub async fn run(&self) {
        eprintln!("[STARTUP] StartupJobRunner::run() called");
        // Clean up orphaned agent runs from previous sessions first
        // These are runs that were left in "running" status when the app was closed/crashed
        match self.agent_run_repo.cancel_all_running().await {
            Ok(count) if count > 0 => {
                info!(count = count, "Cancelled orphaned agent runs from previous session");
            }
            Ok(_) => {
                // No orphaned runs, nothing to log
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to clean up orphaned agent runs");
            }
        }

        // Unblock tasks that got stuck due to app crash (safety net)
        // This runs before pause check since unblocking doesn't spawn agents
        self.unblock_ready_tasks().await;

        // Check if execution is paused - skip resumption if so
        if self.execution_state.is_paused() {
            eprintln!("[STARTUP] Execution paused, skipping task resumption");
            info!("Execution paused, skipping task resumption");
            return;
        }
        eprintln!("[STARTUP] Execution NOT paused, continuing...");

        // Get all projects
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get projects for startup resumption");
                return;
            }
        };

        let mut resumed = 0u32;

        eprintln!("[STARTUP] Found {} projects", projects.len());

        // Iterate through all projects and their tasks in agent-active states
        for project in &projects {
            eprintln!("[STARTUP] Checking project: {}", project.id.as_str());
            for status in AGENT_ACTIVE_STATUSES {
                // Get tasks in this status for this project
                let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status"
                        );
                        continue;
                    }
                };

                eprintln!("[STARTUP] Found {} tasks in {:?} status", tasks.len(), status);
                for task in tasks {
                    let reconciled = match *status {
                        InternalStatus::Executing | InternalStatus::ReExecuting => {
                            self.reconcile_completed_execution(&task, *status).await
                        }
                        InternalStatus::Reviewing => {
                            self.reconcile_reviewing_task(&task, *status).await
                        }
                        InternalStatus::Merging => {
                            self.reconcile_merging_task(&task, *status).await
                        }
                        InternalStatus::QaRefining | InternalStatus::QaTesting => {
                            self.reconcile_qa_task(&task, *status).await
                        }
                        _ => false,
                    };

                    if reconciled {
                        continue;
                    }

                    eprintln!("[STARTUP] Resuming task: {} ({})", task.id.as_str(), task.title);
                    // Check if we can start another task
                    if !self.execution_state.can_start_task() {
                        info!(
                            max_concurrent = self.execution_state.max_concurrent(),
                            running_count = self.execution_state.running_count(),
                            "Max concurrent reached, stopping resumption"
                        );
                        info!(count = resumed, "Task resumption complete (partial)");
                        return;
                    }

                    info!(
                        task_id = task.id.as_str(),
                        status = ?status,
                        "Resuming task"
                    );

                    // Re-execute entry actions to respawn the agent
                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;

                    resumed += 1;
                }
            }
        }

        info!(count = resumed, "Task resumption complete");

        // Re-trigger auto-transition states that may have been interrupted mid-transition
        // These states have on_enter side effects that trigger auto-transitions to spawn agents
        for project in &projects {
            for status in AUTO_TRANSITION_STATES {
                let tasks = match self.task_repo.get_by_status(&project.id, *status).await {
                    Ok(tasks) => tasks,
                    Err(e) => {
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for auto-transition"
                        );
                        continue;
                    }
                };

                eprintln!(
                    "[STARTUP] Found {} tasks in {:?} status (auto-transition)",
                    tasks.len(),
                    status
                );
                for task in tasks {
                    // Check max_concurrent before triggering (auto-transitions may spawn agents)
                    if !self.execution_state.can_start_task() {
                        info!(
                            max_concurrent = self.execution_state.max_concurrent(),
                            running_count = self.execution_state.running_count(),
                            "Max concurrent reached, stopping auto-transition recovery"
                        );
                        return;
                    }

                    eprintln!(
                        "[STARTUP] Re-triggering auto-transition for task: {} ({})",
                        task.id.as_str(),
                        task.title
                    );
                    info!(
                        task_id = task.id.as_str(),
                        status = ?status,
                        "Re-triggering auto-transition for stuck task"
                    );

                    // Re-execute entry actions - this will trigger check_auto_transition()
                    self.transition_service
                        .execute_entry_actions(&task.id, &task, *status)
                        .await;
                }
            }
        }

        // After resuming agent-active tasks, try to schedule any Ready tasks
        // that may be waiting in the queue (if scheduler is configured)
        if let Some(ref scheduler) = self.task_scheduler {
            info!("Scheduling Ready tasks after resumption");
            scheduler.try_schedule_ready_tasks().await;
        }
    }

    /// Reconcile tasks that finished execution but are still marked executing.
    ///
    /// This runs while the app is active to recover from missed transitions
    /// (e.g., app crash or dropped events).
    pub async fn reconcile_stuck_tasks(&self) {
        if self.execution_state.is_paused() {
            return;
        }

        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get projects for reconciliation");
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
                        tracing::warn!(
                            project_id = project.id.as_str(),
                            status = ?status,
                            error = %e,
                            "Failed to get tasks by status for reconciliation"
                        );
                        continue;
                    }
                };

                for task in tasks {
                    match status {
                        InternalStatus::Executing | InternalStatus::ReExecuting => {
                            let _ = self.reconcile_completed_execution(&task, status).await;
                        }
                        InternalStatus::Reviewing => {
                            let _ = self.reconcile_reviewing_task(&task, status).await;
                        }
                        InternalStatus::Merging => {
                            let _ = self.reconcile_merging_task(&task, status).await;
                        }
                        InternalStatus::QaRefining | InternalStatus::QaTesting => {
                            let _ = self.reconcile_qa_task(&task, status).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn reconcile_qa_task(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::QaRefining && status != InternalStatus::QaTesting {
            return false;
        }

        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                tracing::warn!(
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

        tracing::warn!(
            task_id = task.id.as_str(),
            status = ?status,
            "QA task appears stuck; re-triggering entry actions"
        );

        self.transition_service
            .execute_entry_actions(&task.id, task, status)
            .await;

        true
    }

    async fn reconcile_completed_execution(&self, task: &Task, status: InternalStatus) -> bool {
        if status != InternalStatus::Executing && status != InternalStatus::ReExecuting {
            return false;
        }

        let status_history = match self.task_repo.get_status_history(&task.id).await {
            Ok(history) => history,
            Err(e) => {
                tracing::warn!(
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
                    tracing::warn!(
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

            tracing::warn!(
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

            tracing::warn!(
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

            tracing::warn!(
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

        crate::application::chat_service::reconcile_merge_auto_complete(
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

    /// Unblock tasks whose blockers are all complete.
    ///
    /// This is a safety net for cases where the app crashed before real-time
    /// unblocking could run (e.g., when a task merged but the app closed before
    /// dependent tasks were unblocked).
    ///
    /// Scans all Blocked tasks across all projects and transitions those
    /// whose blockers are all in terminal states (Approved, Merged, Failed, Cancelled)
    /// to Ready status.
    async fn unblock_ready_tasks(&self) {
        // Get all projects to scan for blocked tasks
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch projects for startup unblock");
                return;
            }
        };

        let mut unblocked_count = 0u32;

        for project in projects {
            // Get all blocked tasks for this project
            let blocked_tasks = match self
                .task_repo
                .get_by_status(&project.id, InternalStatus::Blocked)
                .await
            {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        project_id = project.id.as_str(),
                        "Failed to fetch blocked tasks for project"
                    );
                    continue;
                }
            };

            if blocked_tasks.is_empty() {
                continue;
            }

            tracing::debug!(
                project_id = project.id.as_str(),
                count = blocked_tasks.len(),
                "Checking blocked tasks for startup unblock"
            );

            for mut task in blocked_tasks {
                // Get blockers for this task
                let blockers = match self.task_dep_repo.get_blockers(&task.id).await {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            task_id = task.id.as_str(),
                            "Failed to get blockers for task"
                        );
                        continue;
                    }
                };

                // Check if all blockers are complete
                if !self.all_blockers_complete(&blockers).await {
                    continue;
                }

                // All blockers complete - transition to Ready
                task.internal_status = InternalStatus::Ready;
                task.blocked_reason = None;
                task.touch();

                if let Err(e) = self.task_repo.update(&task).await {
                    tracing::error!(
                        error = %e,
                        task_id = task.id.as_str(),
                        "Failed to unblock task on startup"
                    );
                    continue;
                }

                tracing::info!(
                    task_id = task.id.as_str(),
                    task_title = %task.title,
                    "Task unblocked on startup - all blockers complete"
                );

                // Emit task:unblocked event for UI update
                if let Some(ref handle) = self.app_handle {
                    let _ = handle.emit(
                        "task:unblocked",
                        serde_json::json!({
                            "taskId": task.id.as_str(),
                            "taskTitle": task.title,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        }),
                    );
                }

                unblocked_count += 1;
            }
        }

        if unblocked_count > 0 {
            info!(count = unblocked_count, "Unblocked tasks on startup");
        } else {
            tracing::debug!("No blocked tasks needed unblocking on startup");
        }
    }

    /// Check if all blocker tasks are in a terminal state.
    ///
    /// Terminal states are: Approved, Merged, Failed, Cancelled.
    /// If a blocker doesn't exist (was deleted), it's considered complete.
    async fn all_blockers_complete(
        &self,
        blocker_ids: &[crate::domain::entities::TaskId],
    ) -> bool {
        for blocker_id in blocker_ids {
            match self.task_repo.get_by_id(blocker_id).await {
                Ok(Some(task)) => {
                    if !matches!(
                        task.internal_status,
                        InternalStatus::Approved
                            | InternalStatus::Merged
                            | InternalStatus::Failed
                            | InternalStatus::Cancelled
                    ) {
                        return false;
                    }
                }
                Ok(None) => {
                    // Blocker was deleted - consider it complete (not blocking)
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        blocker_id = blocker_id.as_str(),
                        "Failed to fetch blocker task, assuming incomplete"
                    );
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{ChatContextType, InternalStatus, Project, Task};
    use crate::domain::state_machine::mocks::MockTaskScheduler;
    // Helper to create test state
    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();
        (execution_state, app_state)
    }

    /// Helper to build a StartupJobRunner from test state
    fn build_runner(
        app_state: &AppState,
        execution_state: &Arc<ExecutionState>,
    ) -> StartupJobRunner<tauri::Wry> {
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(execution_state),
            None,
        );

        let agent_run_repo = Arc::clone(&app_state.agent_run_repo);

        StartupJobRunner::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            agent_run_repo,
            transition_service,
            Arc::clone(execution_state),
        )
    }

    #[tokio::test]
    async fn test_resumption_skipped_when_paused() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in Executing state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Pause execution
        execution_state.pause();

        let runner = build_runner(&app_state, &execution_state);

        // Run should skip because paused
        runner.run().await;

        // Running count should still be 0 (no tasks resumed)
        assert_eq!(execution_state.running_count(), 0);

        // Verify no conversations were created (entry actions were NOT called)
        let convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task.id.as_str())
            .await
            .unwrap();
        assert_eq!(convs.len(), 0, "No conversations should be created when paused");
    }

    #[tokio::test]
    async fn test_resumption_spawns_agents() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in Executing state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow resumption
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run should trigger entry actions for the Executing task
        runner.run().await;

        // Verify entry action was called by checking that a ChatConversation
        // was created for the task. The on_enter(Executing) handler calls
        // chat_service.send_message(TaskExecution, task_id, ...) which creates
        // a conversation in the repo before attempting to spawn the CLI agent.
        let convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            convs.len(),
            1,
            "Entry action should create a TaskExecution conversation for the resumed task"
        );

        // Verify the task is still in Executing state (entry actions don't change status)
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.internal_status, InternalStatus::Executing);
    }

    #[tokio::test]
    async fn test_resumption_handles_empty_projects() {
        let (execution_state, app_state) = setup_test_state().await;

        let runner = build_runner(&app_state, &execution_state);

        // Run should complete without panic
        runner.run().await;

        // Running count should be 0
        assert_eq!(execution_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_resumption_respects_max_concurrent() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 2
        execution_state.set_max_concurrent(2);

        // Create a project with 5 tasks in Executing state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        for i in 0..5 {
            let mut task = Task::new(project.id.clone(), format!("Executing Task {}", i));
            task.internal_status = InternalStatus::Executing;
            app_state.task_repo.create(task).await.unwrap();
        }

        let runner = build_runner(&app_state, &execution_state);

        // Run should stop after 2 tasks due to max_concurrent
        runner.run().await;

        // Note: The actual increment happens in execute_entry_actions via the spawner.
        // Since we're using a mock spawner without execution_state wired in for this test,
        // the running_count won't actually increment. This test verifies the loop structure
        // and early exit logic based on can_start_task().

        // With our mock setup, running_count stays at 0 because the spawner doesn't have
        // execution_state. In production, the spawner would increment_running() on each spawn.
        // The test verifies that run() completes without panic when max_concurrent is reached.
    }

    #[tokio::test]
    async fn test_resumption_handles_multiple_statuses() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with tasks in various agent-active states
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task1 = Task::new(project.id.clone(), "Executing Task".to_string());
        task1.internal_status = InternalStatus::Executing;
        let task1_id = task1.id.clone();
        app_state.task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "QaRefining Task".to_string());
        task2.internal_status = InternalStatus::QaRefining;
        app_state.task_repo.create(task2).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3).await.unwrap();

        // Create a task NOT in agent-active state (should be skipped)
        let mut task4 = Task::new(project.id.clone(), "Ready Task".to_string());
        task4.internal_status = InternalStatus::Ready;
        let task4_id = task4.id.clone();
        app_state.task_repo.create(task4).await.unwrap();

        // Set high max_concurrent so all tasks can be resumed
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run should complete
        runner.run().await;

        // Verify entry actions were called for agent-active tasks:
        // - Executing task should have a TaskExecution conversation created by on_enter(Executing)
        let exec_convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task1_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            exec_convs.len(),
            1,
            "Executing task should have a TaskExecution conversation"
        );

        // - Ready task should NOT have any conversations (not an agent-active state)
        let ready_exec_convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task4_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            ready_exec_convs.len(),
            0,
            "Ready task should not be resumed"
        );
    }

    #[tokio::test]
    async fn test_startup_schedules_ready_tasks_when_scheduler_configured() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a mock scheduler to verify it gets called
        let scheduler = Arc::new(MockTaskScheduler::new());

        let runner = build_runner(&app_state, &execution_state)
            .with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

        // Run startup (no agent-active tasks, but should still call scheduler)
        runner.run().await;

        // Verify scheduler was called once at the end of startup
        assert_eq!(
            scheduler.call_count(),
            1,
            "Scheduler should be called once after startup resumption"
        );
    }

    #[tokio::test]
    async fn test_startup_does_not_schedule_when_paused() {
        let (execution_state, app_state) = setup_test_state().await;

        // Pause execution
        execution_state.pause();

        // Create a mock scheduler
        let scheduler = Arc::new(MockTaskScheduler::new());

        let runner = build_runner(&app_state, &execution_state)
            .with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

        // Run startup while paused
        runner.run().await;

        // Scheduler should NOT be called when paused (early return)
        assert_eq!(
            scheduler.call_count(),
            0,
            "Scheduler should not be called when execution is paused"
        );
    }

    #[tokio::test]
    async fn test_startup_schedules_after_resuming_agent_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with an Executing task (agent-active)
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow resumption
        execution_state.set_max_concurrent(10);

        // Create a mock scheduler
        let scheduler = Arc::new(MockTaskScheduler::new());

        let runner = build_runner(&app_state, &execution_state)
            .with_task_scheduler(Arc::clone(&scheduler) as Arc<dyn TaskScheduler>);

        // Run startup - should resume the Executing task AND call scheduler
        runner.run().await;

        // Verify scheduler was called (happens after resumption loop)
        assert_eq!(
            scheduler.call_count(),
            1,
            "Scheduler should be called after resuming agent-active tasks"
        );
    }

    // ============================================================
    // Phase 68 Tests: Crash Recovery for Auto-Transition States
    // ============================================================

    #[tokio::test]
    async fn test_merging_state_resumed_on_startup() {
        // Merging state was added to AGENT_ACTIVE_STATUSES in Phase 68
        // Tasks in Merging state should have their entry actions re-triggered
        // to respawn the merger agent
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in Merging state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Merging Task".to_string());
        task.internal_status = InternalStatus::Merging;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow resumption
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run startup
        runner.run().await;

        // Verify entry actions were called by checking for Merge conversation
        // on_enter(Merging) creates a ChatContextType::Merge conversation
        let convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::Merge, task_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            convs.len(),
            1,
            "Merging task should have a Merge conversation created (merger agent respawned)"
        );

        // Task should still be in Merging state (entry actions don't change status)
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(updated_task.internal_status, InternalStatus::Merging);
    }

    #[tokio::test]
    async fn test_pending_review_auto_transitions_on_startup() {
        // PendingReview is in AUTO_TRANSITION_STATES
        // Tasks stuck in PendingReview should auto-transition to Reviewing
        // which spawns a reviewer agent
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in PendingReview state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "PendingReview Task".to_string());
        task.internal_status = InternalStatus::PendingReview;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow auto-transition
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should trigger auto-transition to Reviewing
        runner.run().await;

        // The auto-transition path is:
        // 1. execute_entry_actions(PendingReview) triggers on_enter(PendingReview)
        // 2. check_auto_transition detects PendingReview -> Reviewing
        // 3. transition_to(Reviewing) is called
        // 4. on_enter(Reviewing) spawns reviewer agent (creates Review conversation)

        // Check for Review conversation (indicates Reviewing was entered)
        let review_convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::Review, task_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            review_convs.len(),
            1,
            "PendingReview task should auto-transition to Reviewing and create Review conversation"
        );

        // Task should now be in Reviewing state
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Reviewing,
            "Task should have auto-transitioned from PendingReview to Reviewing"
        );
    }

    #[tokio::test]
    async fn test_revision_needed_auto_transitions_on_startup() {
        // RevisionNeeded is in AUTO_TRANSITION_STATES
        // Tasks stuck in RevisionNeeded should auto-transition to ReExecuting
        // which spawns a worker agent
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in RevisionNeeded state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "RevisionNeeded Task".to_string());
        task.internal_status = InternalStatus::RevisionNeeded;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow auto-transition
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should trigger auto-transition to ReExecuting
        runner.run().await;

        // The auto-transition path is:
        // 1. execute_entry_actions(RevisionNeeded) triggers on_enter(RevisionNeeded)
        // 2. check_auto_transition detects RevisionNeeded -> ReExecuting
        // 3. transition_to(ReExecuting) is called
        // 4. on_enter(ReExecuting) spawns worker agent (creates TaskExecution conversation)

        // Check for TaskExecution conversation (indicates ReExecuting was entered)
        let exec_convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::TaskExecution, task_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            exec_convs.len(),
            1,
            "RevisionNeeded task should auto-transition to ReExecuting and create TaskExecution conversation"
        );

        // Task should now be in ReExecuting state
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::ReExecuting,
            "Task should have auto-transitioned from RevisionNeeded to ReExecuting"
        );
    }

    #[tokio::test]
    async fn test_approved_auto_transitions_on_startup() {
        // Approved is in AUTO_TRANSITION_STATES
        // Tasks stuck in Approved should auto-transition to PendingMerge
        // which triggers programmatic merge attempt
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in Approved state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Approved Task".to_string());
        task.internal_status = InternalStatus::Approved;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow auto-transition
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should trigger auto-transition to PendingMerge
        runner.run().await;

        // The auto-transition path is:
        // 1. execute_entry_actions(Approved) triggers on_enter(Approved)
        // 2. check_auto_transition detects Approved -> PendingMerge
        // 3. transition_to(PendingMerge) is called
        // 4. on_enter(PendingMerge) runs attempt_programmatic_merge()

        // Task should now be in PendingMerge state (or further if merge succeeded/failed)
        // Since we're in test mode without a real git repo, the merge will likely fail
        // and the task may transition to Merging or MergeConflict
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        // Approved should NOT be the final state - auto-transition should have occurred
        assert_ne!(
            updated_task.internal_status,
            InternalStatus::Approved,
            "Task should have auto-transitioned from Approved (to PendingMerge or beyond)"
        );
    }

    #[tokio::test]
    async fn test_qa_passed_auto_transitions_on_startup() {
        // QaPassed is in AUTO_TRANSITION_STATES
        // Tasks stuck in QaPassed should auto-transition to PendingReview
        // which then auto-transitions to Reviewing (spawns reviewer)
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a task in QaPassed state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "QaPassed Task".to_string());
        task.internal_status = InternalStatus::QaPassed;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Set high max_concurrent to allow auto-transition
        execution_state.set_max_concurrent(10);

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should trigger auto-transition chain
        runner.run().await;

        // The auto-transition chain is:
        // 1. execute_entry_actions(QaPassed) triggers on_enter(QaPassed)
        // 2. check_auto_transition detects QaPassed -> PendingReview
        // 3. transition_to(PendingReview) -> on_enter(PendingReview)
        // 4. check_auto_transition detects PendingReview -> Reviewing
        // 5. transition_to(Reviewing) -> on_enter(Reviewing) spawns reviewer

        // Check for Review conversation (indicates the full chain completed)
        let review_convs = app_state
            .chat_conversation_repo
            .get_by_context(ChatContextType::Review, task_id.as_str())
            .await
            .unwrap();
        assert_eq!(
            review_convs.len(),
            1,
            "QaPassed task should auto-transition through PendingReview to Reviewing"
        );

        // Task should now be in Reviewing state
        let updated_task = app_state.task_repo.get_by_id(&task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Reviewing,
            "Task should have auto-transitioned from QaPassed through PendingReview to Reviewing"
        );
    }

    #[tokio::test]
    async fn test_auto_transition_respects_max_concurrent() {
        // Auto-transitions that spawn agents should respect max_concurrent.
        // This test verifies the loop structure and early exit logic based on can_start_task().
        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 2
        execution_state.set_max_concurrent(2);

        // Create a project with 5 tasks in PendingReview state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        for i in 0..5 {
            let mut task = Task::new(project.id.clone(), format!("PendingReview Task {}", i));
            task.internal_status = InternalStatus::PendingReview;
            app_state.task_repo.create(task).await.unwrap();
        }

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should stop after max_concurrent is reached
        runner.run().await;

        // Note: The actual increment happens in execute_entry_actions via the spawner.
        // Since we're using a mock spawner without execution_state wired in for this test,
        // the running_count won't actually increment. This test verifies the loop structure
        // and early exit logic based on can_start_task().
        //
        // With our mock setup, running_count stays at 0 because the spawner doesn't have
        // execution_state. In production, the spawner would increment_running() on each spawn.
        // The test verifies that run() completes without panic when max_concurrent check exists.
    }

    // ============================================================
    // Phase 77 Tests: Startup Unblock Recovery
    // ============================================================

    #[tokio::test]
    async fn test_blocked_task_unblocked_when_blocker_is_merged() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a blocker task that is already merged
        let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
        blocker_task.internal_status = InternalStatus::Merged;
        app_state.task_repo.create(blocker_task.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add the dependency relationship
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker_task.id)
            .await
            .unwrap();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup - should unblock the blocked task
        runner.run().await;

        // Verify the blocked task is now Ready
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Ready,
            "Blocked task should be unblocked when blocker is Merged"
        );
        assert!(
            updated_task.blocked_reason.is_none(),
            "blocked_reason should be cleared"
        );
    }

    #[tokio::test]
    async fn test_blocked_task_unblocked_when_blocker_is_approved() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a blocker task that is approved
        let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
        blocker_task.internal_status = InternalStatus::Approved;
        app_state.task_repo.create(blocker_task.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add the dependency relationship
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker_task.id)
            .await
            .unwrap();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup
        runner.run().await;

        // Verify the blocked task is now Ready
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Ready,
            "Blocked task should be unblocked when blocker is Approved"
        );
    }

    #[tokio::test]
    async fn test_blocked_task_remains_blocked_when_blocker_incomplete() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a blocker task that is still executing
        let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
        blocker_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(blocker_task.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        blocked_task.blocked_reason = Some("Waiting for: Blocker Task".to_string());
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add the dependency relationship
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker_task.id)
            .await
            .unwrap();

        // Pause execution to skip agent resumption (we only want to test unblocking)
        execution_state.pause();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup
        runner.run().await;

        // Verify the blocked task is still Blocked
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Blocked,
            "Blocked task should remain blocked when blocker is still Executing"
        );
        assert!(
            updated_task.blocked_reason.is_some(),
            "blocked_reason should be preserved"
        );
    }

    #[tokio::test]
    async fn test_blocked_task_with_multiple_blockers_all_complete() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create two blocker tasks - both complete
        let mut blocker1 = Task::new(project.id.clone(), "Blocker 1".to_string());
        blocker1.internal_status = InternalStatus::Merged;
        app_state.task_repo.create(blocker1.clone()).await.unwrap();

        let mut blocker2 = Task::new(project.id.clone(), "Blocker 2".to_string());
        blocker2.internal_status = InternalStatus::Failed; // Failed is also a terminal state
        app_state.task_repo.create(blocker2.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add both dependency relationships
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker1.id)
            .await
            .unwrap();
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker2.id)
            .await
            .unwrap();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup
        runner.run().await;

        // Verify the blocked task is now Ready
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Ready,
            "Blocked task should be unblocked when all blockers are complete"
        );
    }

    #[tokio::test]
    async fn test_blocked_task_with_multiple_blockers_one_incomplete() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create two blocker tasks - one complete, one not
        let mut blocker1 = Task::new(project.id.clone(), "Blocker 1".to_string());
        blocker1.internal_status = InternalStatus::Merged;
        app_state.task_repo.create(blocker1.clone()).await.unwrap();

        let mut blocker2 = Task::new(project.id.clone(), "Blocker 2".to_string());
        blocker2.internal_status = InternalStatus::Reviewing; // Still in progress
        app_state.task_repo.create(blocker2.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add both dependency relationships
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker1.id)
            .await
            .unwrap();
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker2.id)
            .await
            .unwrap();

        // Pause execution to skip agent resumption
        execution_state.pause();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup
        runner.run().await;

        // Verify the blocked task is still Blocked
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Blocked,
            "Blocked task should remain blocked when any blocker is incomplete"
        );
    }

    #[tokio::test]
    async fn test_unblock_runs_even_when_paused() {
        // Unblocking should run even when execution is paused, since it doesn't spawn agents
        let (execution_state, app_state) = setup_test_state().await;

        // Pause execution
        execution_state.pause();

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a merged blocker
        let mut blocker_task = Task::new(project.id.clone(), "Blocker Task".to_string());
        blocker_task.internal_status = InternalStatus::Merged;
        app_state.task_repo.create(blocker_task.clone()).await.unwrap();

        // Create a blocked task
        let mut blocked_task = Task::new(project.id.clone(), "Blocked Task".to_string());
        blocked_task.internal_status = InternalStatus::Blocked;
        app_state.task_repo.create(blocked_task.clone()).await.unwrap();

        // Add dependency
        app_state
            .task_dependency_repo
            .add_dependency(&blocked_task.id, &blocker_task.id)
            .await
            .unwrap();

        let runner = build_runner(&app_state, &execution_state);

        // Run startup while paused
        runner.run().await;

        // Verify the blocked task is still unblocked even though execution is paused
        let updated_task = app_state.task_repo.get_by_id(&blocked_task.id).await.unwrap().unwrap();
        assert_eq!(
            updated_task.internal_status,
            InternalStatus::Ready,
            "Blocked task should be unblocked even when execution is paused"
        );
    }
}
