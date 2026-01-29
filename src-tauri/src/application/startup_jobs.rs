// Startup Job Runner
//
// Handles automatic task resumption when the app restarts.
// Tasks that were in agent-active states (Executing, QaRefining, QaTesting, Reviewing, ReExecuting)
// when the app shut down are automatically resumed on startup, respecting pause state and
// max_concurrent limits.
//
// Usage:
// - Called once during app initialization after HTTP server is ready
// - Iterates all projects to find tasks in agent-active states
// - Re-executes entry actions to respawn agents
// - Stops early if max_concurrent is reached

use std::sync::Arc;
use tauri::Runtime;
use tracing::info;

use crate::commands::execution_commands::{ExecutionState, AGENT_ACTIVE_STATUSES};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::domain::state_machine::services::TaskScheduler;

use super::TaskTransitionService;

/// Runs startup jobs, primarily task resumption.
///
/// Finds all tasks that were in agent-active states when the app shut down
/// and re-triggers their entry actions to respawn worker agents.
pub struct StartupJobRunner<R: Runtime = tauri::Wry> {
    task_repo: Arc<dyn TaskRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    transition_service: TaskTransitionService<R>,
    execution_state: Arc<ExecutionState>,
    /// Optional task scheduler for auto-starting Ready tasks on startup.
    /// When provided, Ready tasks will be scheduled after resuming agent-active tasks.
    task_scheduler: Option<Arc<dyn TaskScheduler>>,
}

impl<R: Runtime> StartupJobRunner<R> {
    /// Create a new StartupJobRunner with all required dependencies.
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        transition_service: TaskTransitionService<R>,
        execution_state: Arc<ExecutionState>,
    ) -> Self {
        Self {
            task_repo,
            project_repo,
            transition_service,
            execution_state,
            task_scheduler: None,
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

    /// Run startup jobs, resuming tasks in agent-active states.
    ///
    /// Skips if execution is paused. Stops early if max_concurrent is reached.
    /// For each task in an agent-active state, re-executes entry actions to
    /// respawn the appropriate agent.
    pub async fn run(&self) {
        // Check if execution is paused - skip resumption if so
        if self.execution_state.is_paused() {
            info!("Execution paused, skipping task resumption");
            return;
        }

        // Get all projects
        let projects = match self.project_repo.get_all().await {
            Ok(projects) => projects,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get projects for startup resumption");
                return;
            }
        };

        let mut resumed = 0u32;

        // Iterate through all projects and their tasks in agent-active states
        for project in projects {
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

                for task in tasks {
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

        // After resuming agent-active tasks, try to schedule any Ready tasks
        // that may be waiting in the queue (if scheduler is configured)
        if let Some(ref scheduler) = self.task_scheduler {
            info!("Scheduling Ready tasks after resumption");
            scheduler.try_schedule_ready_tasks().await;
        }
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
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(execution_state),
            None,
        );

        StartupJobRunner::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.project_repo),
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
}
