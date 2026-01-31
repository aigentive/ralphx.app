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
use crate::domain::entities::InternalStatus;
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, ProjectRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;

use super::TaskTransitionService;

/// Production implementation of TaskScheduler for auto-scheduling Ready tasks.
///
/// This service queries for the oldest Ready task across all projects and
/// transitions it to Executing when execution slots are available.
pub struct TaskSchedulerService<R: Runtime = tauri::Wry> {
    execution_state: Arc<ExecutionState>,
    project_repo: Arc<dyn ProjectRepository>,
    task_repo: Arc<dyn TaskRepository>,
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

    /// Find the oldest Ready task across all projects.
    ///
    /// Uses the repository's optimized cross-project query for efficient FIFO scheduling.
    /// Returns None if no Ready tasks exist or if there's an error querying.
    async fn find_oldest_ready_task(&self) -> Option<crate::domain::entities::Task> {
        match self.task_repo.get_oldest_ready_task().await {
            Ok(task) => task,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get oldest Ready task for scheduling");
                None
            }
        }
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

        // Find the oldest Ready task
        let Some(task) = self.find_oldest_ready_task().await else {
            tracing::debug!("No Ready tasks to schedule");
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
    async fn test_find_oldest_ready_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project
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
        let found = scheduler.find_oldest_ready_task().await;
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
}
