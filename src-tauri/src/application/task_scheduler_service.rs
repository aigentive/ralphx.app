// Task Scheduler Service
//
// Production implementation of the TaskScheduler trait for auto-scheduling Ready tasks.
// This service checks execution capacity and transitions the oldest Ready task to Executing
// when slots are available.
//
// Called from:
// - TransitionHandler::on_exit() when an agent-active task completes (slot freed)
// - TransitionHandler::on_enter(Ready) when a task becomes Ready
// - StartupJobRunner after resuming agent-active tasks
// - resume_execution and set_max_concurrent commands (future Phase 26 tasks)

use std::sync::Arc;
use async_trait::async_trait;
use tauri::{AppHandle, Runtime};

use crate::commands::ExecutionState;
use crate::domain::entities::{GitMode, InternalStatus, Task};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;

/// States that indicate a task is "running" (actively executing or being processed)
/// Used for Local-mode single-task enforcement
const LOCAL_MODE_RUNNING_STATES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::ReExecuting,
    InternalStatus::Reviewing,
    InternalStatus::Merging,
];

use super::TaskTransitionService;

/// Production implementation of TaskScheduler for auto-scheduling Ready tasks.
///
/// This service queries for the oldest Ready task across all projects and
/// transitions it to Executing when execution slots are available.
pub struct TaskSchedulerService<R: Runtime = tauri::Wry> {
    execution_state: Arc<ExecutionState>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<RunningAgentRegistry>,
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> TaskSchedulerService<R> {
    /// Create a new TaskSchedulerService with all required dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        execution_state: Arc<ExecutionState>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<RunningAgentRegistry>,
        app_handle: Option<AppHandle<R>>,
    ) -> Self {
        Self {
            execution_state,
            project_repo,
            task_repo,
            task_dependency_repo,
            chat_message_repo,
            conversation_repo,
            agent_run_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            app_handle,
        }
    }

    /// Find the oldest schedulable task across all projects.
    ///
    /// For Worktree-mode projects, any Ready task is schedulable.
    /// For Local-mode projects, a task is only schedulable if no other task
    /// in the same project is in a "running" state (Executing, ReExecuting,
    /// Reviewing, or Merging).
    ///
    /// Returns None if no schedulable tasks exist or if there's an error querying.
    async fn find_oldest_schedulable_task(&self) -> Option<Task> {
        // Get a batch of oldest Ready tasks to evaluate
        let ready_tasks = match self.task_repo.get_oldest_ready_tasks(50).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get Ready tasks for scheduling");
                return None;
            }
        };

        for task in ready_tasks {
            // Get the project to check its git mode
            let project = match self.project_repo.get_by_id(&task.project_id).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        project_id = task.project_id.as_str(),
                        "Task has non-existent project, skipping"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        task_id = task.id.as_str(),
                        "Failed to get project for task, skipping"
                    );
                    continue;
                }
            };

            // For Local-mode projects, check if another task is already running
            if project.git_mode == GitMode::Local {
                let has_running = match self
                    .task_repo
                    .has_task_in_states(&project.id, LOCAL_MODE_RUNNING_STATES)
                    .await
                {
                    Ok(running) => running,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            project_id = project.id.as_str(),
                            "Failed to check running tasks for Local-mode project, skipping"
                        );
                        continue;
                    }
                };

                if has_running {
                    tracing::debug!(
                        task_id = task.id.as_str(),
                        project_id = project.id.as_str(),
                        "Skipping task: Local-mode project already has a running task"
                    );
                    continue;
                }
            }

            // This task is schedulable
            return Some(task);
        }

        None
    }

    /// Build a TaskTransitionService for transitioning tasks.
    ///
    /// Creates a fresh instance to avoid circular dependency issues when
    /// the scheduler is called from within TransitionHandler.
    fn build_transition_service(&self) -> TaskTransitionService<R>
    where
        R: Runtime,
    {
        TaskTransitionService::new(
            Arc::clone(&self.task_repo),
            Arc::clone(&self.task_dependency_repo),
            Arc::clone(&self.project_repo),
            Arc::clone(&self.chat_message_repo),
            Arc::clone(&self.conversation_repo),
            Arc::clone(&self.agent_run_repo),
            Arc::clone(&self.ideation_session_repo),
            Arc::clone(&self.activity_event_repo),
            Arc::clone(&self.message_queue),
            Arc::clone(&self.running_agent_registry),
            Arc::clone(&self.execution_state),
            self.app_handle.clone(),
        )
    }
}

#[async_trait]
impl<R: Runtime> TaskScheduler for TaskSchedulerService<R> {
    /// Try to schedule Ready tasks if execution slots are available.
    ///
    /// This method:
    /// 1. Checks if execution is paused or at capacity
    /// 2. Finds the oldest Ready task across all projects
    /// 3. Transitions it to Executing state via the state machine
    async fn try_schedule_ready_tasks(&self) {
        // Check if we can start a new task
        if !self.execution_state.can_start_task() {
            tracing::debug!(
                is_paused = self.execution_state.is_paused(),
                running_count = self.execution_state.running_count(),
                max_concurrent = self.execution_state.max_concurrent(),
                "Cannot schedule: execution paused or at capacity"
            );
            return;
        }

        // Find the oldest schedulable task (accounting for Local-mode constraints)
        let Some(task) = self.find_oldest_schedulable_task().await else {
            tracing::debug!("No schedulable tasks found");
            return;
        };

        tracing::info!(
            task_id = task.id.as_str(),
            task_title = task.title.as_str(),
            created_at = %task.created_at,
            "Scheduling Ready task for execution"
        );

        // Transition the task to Executing
        // This triggers on_enter(Executing) which spawns the worker agent
        let transition_service = self.build_transition_service();

        if let Err(e) = transition_service
            .transition_task(&task.id, InternalStatus::Executing)
            .await
        {
            tracing::error!(
                task_id = task.id.as_str(),
                error = %e,
                "Failed to transition Ready task to Executing"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{Project, Task};

    /// Helper to create test state
    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();
        (execution_state, app_state)
    }

    /// Helper to build a TaskSchedulerService from test state
    fn build_scheduler(
        app_state: &AppState,
        execution_state: &Arc<ExecutionState>,
    ) -> TaskSchedulerService<tauri::Wry> {
        TaskSchedulerService::new(
            Arc::clone(execution_state),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            None,
        )
    }

    #[tokio::test]
    async fn test_no_schedule_when_paused() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project with a Ready task
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Pause execution
        execution_state.pause();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not schedule (paused)
        scheduler.try_schedule_ready_tasks().await;

        // Task should still be Ready
        let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_no_schedule_when_at_capacity() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set max concurrent to 1 and fill the slot
        execution_state.set_max_concurrent(1);
        execution_state.increment_running();

        // Create a project with a Ready task
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not schedule (at capacity)
        scheduler.try_schedule_ready_tasks().await;

        // Task should still be Ready
        let updated = app_state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
        assert_eq!(updated.internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_no_schedule_when_no_ready_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should complete without panic (no tasks to schedule)
        scheduler.try_schedule_ready_tasks().await;

        // Running count should still be 0
        assert_eq!(execution_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_schedules_oldest_ready_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create older task first
        let mut older_task = Task::new(project.id.clone(), "Older Task".to_string());
        older_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(older_task.clone()).await.unwrap();
        let older_task_id = older_task.id.clone();

        // Small delay to ensure different created_at timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer task
        let mut newer_task = Task::new(project.id.clone(), "Newer Task".to_string());
        newer_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(newer_task.clone()).await.unwrap();
        let newer_task_id = newer_task.id.clone();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should pick the older task
        scheduler.try_schedule_ready_tasks().await;

        // Older task should be Executing (transitioned)
        let updated_older = app_state.task_repo.get_by_id(&older_task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_older.internal_status,
            InternalStatus::Executing,
            "Older task should be scheduled (now Executing)"
        );

        // Newer task should still be Ready
        let updated_newer = app_state.task_repo.get_by_id(&newer_task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_newer.internal_status,
            InternalStatus::Ready,
            "Newer task should still be Ready"
        );
    }

    #[tokio::test]
    async fn test_schedules_across_projects() {
        let (execution_state, app_state) = setup_test_state().await;

        // Set high max concurrent
        execution_state.set_max_concurrent(10);

        // Create two projects
        let project1 = Project::new("Project 1".to_string(), "/test/path1".to_string());
        app_state.project_repo.create(project1.clone()).await.unwrap();

        let project2 = Project::new("Project 2".to_string(), "/test/path2".to_string());
        app_state.project_repo.create(project2.clone()).await.unwrap();

        // Create older task in project 2
        let mut older_task = Task::new(project2.id.clone(), "Older Task (P2)".to_string());
        older_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(older_task.clone()).await.unwrap();
        let older_task_id = older_task.id.clone();

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create newer task in project 1
        let mut newer_task = Task::new(project1.id.clone(), "Newer Task (P1)".to_string());
        newer_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(newer_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Schedule - should pick the older task from project 2
        scheduler.try_schedule_ready_tasks().await;

        // Older task should be Executing
        let updated_older = app_state.task_repo.get_by_id(&older_task_id).await.unwrap().unwrap();
        assert_eq!(
            updated_older.internal_status,
            InternalStatus::Executing,
            "Older task from Project 2 should be scheduled"
        );
    }

    #[tokio::test]
    async fn test_find_oldest_schedulable_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project (default is Local mode)
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks with different statuses
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(ready_task.clone()).await.unwrap();

        let mut backlog_task = Task::new(project.id.clone(), "Backlog Task".to_string());
        backlog_task.internal_status = InternalStatus::Backlog;
        app_state.task_repo.create(backlog_task).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find only the Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_trait_object_safety() {
        let (execution_state, app_state) = setup_test_state().await;
        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should be usable as trait object
        let scheduler_trait: Arc<dyn TaskScheduler> = Arc::new(scheduler);
        scheduler_trait.try_schedule_ready_tasks().await;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Local Mode Enforcement Tests (Phase 66)
    // ═══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_local_mode_skips_project_with_executing_task() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create an Executing task (blocks the project)
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a Ready task (should be skipped)
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(ready_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should not find the Ready task (Local project has running task)
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_none(), "Should not schedule task when Local-mode project has running task");
    }

    #[tokio::test]
    async fn test_local_mode_allows_scheduling_when_no_running_task() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project
        let mut project = Project::new("Local Project".to_string(), "/test/local".to_string());
        project.git_mode = GitMode::Local;
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create only a Ready task (no running tasks)
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(ready_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find the Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_some(), "Should schedule task when Local-mode project has no running task");
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_worktree_mode_allows_parallel_tasks() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Worktree-mode project
        let mut project = Project::new("Worktree Project".to_string(), "/test/wt".to_string());
        project.git_mode = GitMode::Worktree;
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create an Executing task
        let mut executing_task = Task::new(project.id.clone(), "Executing Task".to_string());
        executing_task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(executing_task).await.unwrap();

        // Create a Ready task
        let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
        ready_task.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(ready_task.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should find the Ready task (Worktree mode allows parallel)
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_some(), "Worktree mode should allow parallel task execution");
        assert_eq!(found.unwrap().id, ready_task.id);
    }

    #[tokio::test]
    async fn test_local_mode_checks_all_running_states() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Test that all running states block scheduling
        let running_states = vec![
            InternalStatus::Executing,
            InternalStatus::ReExecuting,
            InternalStatus::Reviewing,
            InternalStatus::Merging,
        ];

        for blocking_state in running_states {
            // Create a new Local-mode project for each test
            let mut project = Project::new(
                format!("Local Project {}", blocking_state.as_str()),
                format!("/test/local/{}", blocking_state.as_str()),
            );
            project.git_mode = GitMode::Local;
            app_state.project_repo.create(project.clone()).await.unwrap();

            // Create a task in the blocking state
            let mut blocking_task = Task::new(project.id.clone(), "Blocking Task".to_string());
            blocking_task.internal_status = blocking_state;
            app_state.task_repo.create(blocking_task).await.unwrap();

            // Create a Ready task
            let mut ready_task = Task::new(project.id.clone(), "Ready Task".to_string());
            ready_task.internal_status = InternalStatus::Ready;
            app_state.task_repo.create(ready_task).await.unwrap();

            let scheduler = build_scheduler(&app_state, &execution_state);

            // All these tasks should not be schedulable because their projects have a running task
            // We need to test that the specific project's ready task is not found
            let found = scheduler.find_oldest_schedulable_task().await;

            // The found task, if any, should not be from this project
            if let Some(task) = found {
                assert_ne!(
                    task.project_id, project.id,
                    "State {} should block scheduling in Local mode",
                    blocking_state.as_str()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_mixed_mode_projects_schedule_correctly() {
        use crate::domain::entities::GitMode;

        let (execution_state, app_state) = setup_test_state().await;
        execution_state.set_max_concurrent(10);

        // Create a Local-mode project with a running task
        let mut local_project = Project::new("Local Project".to_string(), "/test/local".to_string());
        local_project.git_mode = GitMode::Local;
        app_state.project_repo.create(local_project.clone()).await.unwrap();

        let mut local_executing = Task::new(local_project.id.clone(), "Local Executing".to_string());
        local_executing.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(local_executing).await.unwrap();

        // Create older Ready task in Local project (should be skipped)
        let mut local_ready = Task::new(local_project.id.clone(), "Local Ready".to_string());
        local_ready.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(local_ready).await.unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Create a Worktree-mode project with a running task
        let mut wt_project = Project::new("Worktree Project".to_string(), "/test/wt".to_string());
        wt_project.git_mode = GitMode::Worktree;
        app_state.project_repo.create(wt_project.clone()).await.unwrap();

        let mut wt_executing = Task::new(wt_project.id.clone(), "WT Executing".to_string());
        wt_executing.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(wt_executing).await.unwrap();

        // Create newer Ready task in Worktree project (should be schedulable)
        let mut wt_ready = Task::new(wt_project.id.clone(), "WT Ready".to_string());
        wt_ready.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(wt_ready.clone()).await.unwrap();

        let scheduler = build_scheduler(&app_state, &execution_state);

        // Should skip Local project's Ready task and find Worktree project's Ready task
        let found = scheduler.find_oldest_schedulable_task().await;
        assert!(found.is_some(), "Should find schedulable task from Worktree project");
        assert_eq!(
            found.unwrap().project_id, wt_project.id,
            "Should schedule task from Worktree project, not blocked Local project"
        );
    }
}
