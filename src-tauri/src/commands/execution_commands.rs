// Tauri commands for execution control
// Manages global execution state: pause, resume, stop

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::application::{AppState, TaskTransitionService};
use crate::domain::entities::InternalStatus;

/// Statuses where an agent is actively running.
/// Tasks in these states need to be cancelled when stop is called,
/// and resumed when the app restarts.
///
/// Used by:
/// - `stop_execution` command to find tasks to cancel
/// - `StartupJobRunner` to find tasks to resume on app restart
pub const AGENT_ACTIVE_STATUSES: &[InternalStatus] = &[
    InternalStatus::Executing,
    InternalStatus::QaRefining,
    InternalStatus::QaTesting,
    InternalStatus::Reviewing,
    InternalStatus::ReExecuting,
];

/// Global execution state managed atomically for thread safety
pub struct ExecutionState {
    /// Whether execution is paused (stops picking up new tasks)
    is_paused: AtomicBool,
    /// Number of currently running tasks
    running_count: AtomicU32,
    /// Maximum concurrent tasks allowed
    max_concurrent: AtomicU32,
}

impl ExecutionState {
    /// Create a new ExecutionState with defaults
    pub fn new() -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(2),
        }
    }

    /// Create ExecutionState with custom max concurrent
    pub fn with_max_concurrent(max: u32) -> Self {
        Self {
            is_paused: AtomicBool::new(false),
            running_count: AtomicU32::new(0),
            max_concurrent: AtomicU32::new(max),
        }
    }

    /// Check if execution is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Pause execution (stops picking up new tasks)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
    }

    /// Resume execution
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
    }

    /// Get current running task count
    pub fn running_count(&self) -> u32 {
        self.running_count.load(Ordering::SeqCst)
    }

    /// Increment running count (when a task starts)
    pub fn increment_running(&self) -> u32 {
        self.running_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement running count (when a task completes)
    pub fn decrement_running(&self) -> u32 {
        let prev = self.running_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            // Prevent underflow
            self.running_count.store(0, Ordering::SeqCst);
            0
        } else {
            prev - 1
        }
    }

    /// Get max concurrent tasks
    pub fn max_concurrent(&self) -> u32 {
        self.max_concurrent.load(Ordering::SeqCst)
    }

    /// Set max concurrent tasks
    pub fn set_max_concurrent(&self, max: u32) {
        self.max_concurrent.store(max, Ordering::SeqCst);
    }

    /// Check if we can start a new task
    pub fn can_start_task(&self) -> bool {
        !self.is_paused() && self.running_count() < self.max_concurrent()
    }

    /// Emit execution:status_changed event with current state
    pub fn emit_status_changed<R: Runtime>(&self, handle: &AppHandle<R>, reason: &str) {
        let _ = handle.emit(
            "execution:status_changed",
            serde_json::json!({
                "isPaused": self.is_paused(),
                "runningCount": self.running_count(),
                "maxConcurrent": self.max_concurrent(),
                "reason": reason,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Response for execution status queries
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStatusResponse {
    /// Whether execution is paused
    pub is_paused: bool,
    /// Number of currently running tasks
    pub running_count: u32,
    /// Maximum concurrent tasks allowed
    pub max_concurrent: u32,
    /// Number of tasks queued (ready to execute)
    pub queued_count: u32,
    /// Whether new tasks can be started
    pub can_start_task: bool,
}

/// Response for pause/resume/stop commands
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCommandResponse {
    /// Whether the command succeeded
    pub success: bool,
    /// Current execution status after the command
    pub status: ExecutionStatusResponse,
}

/// Get current execution status
#[tauri::command]
pub async fn get_execution_status(
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionStatusResponse, String> {
    // Count queued tasks (tasks in Ready status)
    let all_projects = app_state
        .project_repo
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    let mut queued_count = 0u32;
    for project in all_projects {
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        queued_count += tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count() as u32;
    }

    Ok(ExecutionStatusResponse {
        is_paused: execution_state.is_paused(),
        running_count: execution_state.running_count(),
        max_concurrent: execution_state.max_concurrent(),
        queued_count,
        can_start_task: execution_state.can_start_task(),
    })
}

/// Pause execution (stops picking up new tasks)
/// Currently running tasks will continue until completion
#[tauri::command]
pub async fn pause_execution(
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    execution_state.pause();

    // Emit status_changed event for real-time UI update
    if let Some(ref handle) = app_state.app_handle {
        execution_state.emit_status_changed(handle, "paused");
    }

    // Get current status
    let status = get_execution_status(execution_state, app_state).await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Resume execution (allows picking up new tasks again)
#[tauri::command]
pub async fn resume_execution(
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    execution_state.resume();

    // Emit status_changed event for real-time UI update
    if let Some(ref handle) = app_state.app_handle {
        execution_state.emit_status_changed(handle, "resumed");
    }

    // Get current status
    let status = get_execution_status(execution_state, app_state).await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Stop execution (cancels current tasks and pauses)
/// This transitions all agent-active tasks to Failed status via TransitionHandler.
/// The on_exit handlers will decrement the running count for each task.
#[tauri::command]
pub async fn stop_execution(
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // First pause to prevent new tasks from starting
    execution_state.pause();

    // Build transition service for proper state machine transitions
    let transition_service = TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.message_queue),
        Arc::clone(&app_state.running_agent_registry),
        Arc::clone(&execution_state),
        app_state.app_handle.clone(),
    );

    // Find all tasks in agent-active states across all projects
    let all_projects = app_state
        .project_repo
        .get_all()
        .await
        .map_err(|e| e.to_string())?;

    for project in all_projects {
        let tasks = app_state
            .task_repo
            .get_by_project(&project.id)
            .await
            .map_err(|e| e.to_string())?;

        for task in tasks {
            // Check if task is in an agent-active state
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                // Use TransitionHandler to transition to Failed
                // This triggers on_exit handlers which decrement running count
                if let Err(e) = transition_service
                    .transition_task(&task.id, InternalStatus::Failed)
                    .await
                {
                    tracing::warn!(
                        task_id = task.id.as_str(),
                        error = %e,
                        "Failed to transition task to Failed during stop"
                    );
                }
            }
        }
    }

    // Note: running_count is decremented by on_exit handlers in TransitionHandler
    // No manual reset needed here

    // Emit status_changed event for real-time UI update
    // This reflects the final state after all tasks have been stopped
    if let Some(ref handle) = app_state.app_handle {
        execution_state.emit_status_changed(handle, "stopped");
    }

    // Get current status
    let status = get_execution_status(execution_state, app_state).await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ========================================
    // ExecutionState Unit Tests
    // ========================================

    #[test]
    fn test_execution_state_new() {
        let state = ExecutionState::new();
        assert!(!state.is_paused());
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.max_concurrent(), 2);
    }

    #[test]
    fn test_execution_state_with_max_concurrent() {
        let state = ExecutionState::with_max_concurrent(5);
        assert_eq!(state.max_concurrent(), 5);
    }

    #[test]
    fn test_execution_state_pause_resume() {
        let state = ExecutionState::new();

        assert!(!state.is_paused());

        state.pause();
        assert!(state.is_paused());

        state.resume();
        assert!(!state.is_paused());
    }

    #[test]
    fn test_execution_state_running_count() {
        let state = ExecutionState::new();

        assert_eq!(state.running_count(), 0);

        let count = state.increment_running();
        assert_eq!(count, 1);
        assert_eq!(state.running_count(), 1);

        let count = state.increment_running();
        assert_eq!(count, 2);
        assert_eq!(state.running_count(), 2);

        let count = state.decrement_running();
        assert_eq!(count, 1);
        assert_eq!(state.running_count(), 1);
    }

    #[test]
    fn test_execution_state_decrement_no_underflow() {
        let state = ExecutionState::new();

        // Should not underflow
        let count = state.decrement_running();
        assert_eq!(count, 0);
        assert_eq!(state.running_count(), 0);
    }

    #[test]
    fn test_execution_state_set_max_concurrent() {
        let state = ExecutionState::new();

        state.set_max_concurrent(10);
        assert_eq!(state.max_concurrent(), 10);
    }

    #[test]
    fn test_execution_state_can_start_task() {
        let state = ExecutionState::with_max_concurrent(2);

        // Initially can start
        assert!(state.can_start_task());

        // After pausing, cannot start
        state.pause();
        assert!(!state.can_start_task());

        // After resuming, can start again
        state.resume();
        assert!(state.can_start_task());

        // Fill up to max concurrent
        state.increment_running();
        state.increment_running();
        assert!(!state.can_start_task());

        // After one completes, can start again
        state.decrement_running();
        assert!(state.can_start_task());
    }

    #[test]
    fn test_execution_state_thread_safe() {
        use std::thread;

        let state = Arc::new(ExecutionState::new());
        let mut handles = vec![];

        // Spawn threads that increment and decrement
        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                state_clone.increment_running();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(state.running_count(), 10);

        let mut handles = vec![];
        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                state_clone.decrement_running();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(state.running_count(), 0);
    }

    // ========================================
    // Response Serialization Tests
    // ========================================

    #[test]
    fn test_execution_status_response_serialization() {
        let response = ExecutionStatusResponse {
            is_paused: true,
            running_count: 1,
            max_concurrent: 2,
            queued_count: 5,
            can_start_task: false,
        };

        let json = serde_json::to_string(&response).unwrap();

        // Verify camelCase serialization
        assert!(json.contains("\"isPaused\":true"));
        assert!(json.contains("\"runningCount\":1"));
        assert!(json.contains("\"maxConcurrent\":2"));
        assert!(json.contains("\"queuedCount\":5"));
        assert!(json.contains("\"canStartTask\":false"));
    }

    #[test]
    fn test_execution_command_response_serialization() {
        let response = ExecutionCommandResponse {
            success: true,
            status: ExecutionStatusResponse {
                is_paused: false,
                running_count: 0,
                max_concurrent: 2,
                queued_count: 3,
                can_start_task: true,
            },
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"status\":"));
        assert!(json.contains("\"isPaused\":false"));
    }

    // ========================================
    // Integration Tests with AppState
    // ========================================

    use crate::domain::entities::{Project, Task};
    use crate::domain::repositories::{ProjectRepository, TaskRepository};
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Create a test project with tasks
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo
            .create(project.clone())
            .await
            .expect("Failed to create test project");

        // Create tasks in various statuses
        let mut task1 = Task::new(project.id.clone(), "Ready Task 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        task_repo
            .create(task1)
            .await
            .expect("Failed to create Ready task 1");

        let mut task2 = Task::new(project.id.clone(), "Ready Task 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        task_repo
            .create(task2)
            .await
            .expect("Failed to create Ready task 2");

        let mut task3 = Task::new(project.id.clone(), "Executing Task".to_string());
        task3.internal_status = InternalStatus::Executing;
        task_repo
            .create(task3)
            .await
            .expect("Failed to create Executing task");

        let mut task4 = Task::new(project.id.clone(), "Backlog Task".to_string());
        task4.internal_status = InternalStatus::Backlog;
        task_repo
            .create(task4)
            .await
            .expect("Failed to create Backlog task");

        let app_state = AppState::with_repos(task_repo, project_repo);

        (execution_state, app_state)
    }

    #[tokio::test]
    async fn test_get_execution_status_counts_ready_tasks() {
        let (execution_state, app_state) = setup_test_state().await;

        // Simulate the command by directly calling the logic
        let all_projects = app_state.project_repo.get_all().await.unwrap();

        let mut queued_count = 0u32;
        for project in all_projects {
            let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
            queued_count += tasks
                .iter()
                .filter(|t| t.internal_status == InternalStatus::Ready)
                .count() as u32;
        }

        // We created 2 ready tasks
        assert_eq!(queued_count, 2);
        assert!(!execution_state.is_paused());
        assert_eq!(execution_state.running_count(), 0);
    }

    #[tokio::test]
    async fn test_pause_sets_paused_flag() {
        let (execution_state, _app_state) = setup_test_state().await;

        assert!(!execution_state.is_paused());
        execution_state.pause();
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_resume_clears_paused_flag() {
        let (execution_state, _app_state) = setup_test_state().await;

        execution_state.pause();
        assert!(execution_state.is_paused());

        execution_state.resume();
        assert!(!execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_stop_cancels_executing_tasks() {
        let (_execution_state, app_state) = setup_test_state().await;

        // Get the project
        let projects = app_state.project_repo.get_all().await.unwrap();
        let project = &projects[0];

        // Find the executing task and cancel it
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for mut task in tasks {
            if task.internal_status == InternalStatus::Executing {
                task.internal_status = InternalStatus::Failed;
                task.touch();
                app_state.task_repo.update(&task).await.unwrap();
            }
        }

        // Verify the task is now failed
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        let executing_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Executing)
            .count();
        let failed_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Failed)
            .count();

        assert_eq!(executing_count, 0);
        assert_eq!(failed_count, 1);
    }

    #[tokio::test]
    async fn test_stop_cancels_multiple_agent_active_tasks() {
        // Setup: Create tasks in various agent-active states
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in all agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "QaRefining Task".to_string());
        task2.internal_status = InternalStatus::QaRefining;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Create a task NOT in agent-active state (should not be affected)
        let mut task4 = Task::new(project.id.clone(), "Ready Task".to_string());
        task4.internal_status = InternalStatus::Ready;
        app_state.task_repo.create(task4.clone()).await.unwrap();

        // Build transition service (same as stop_execution does)
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Pause execution (as stop_execution would)
        execution_state.pause();

        // Transition all agent-active tasks to Failed
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Failed)
                    .await;
            }
        }

        // Verify: All agent-active tasks should now be Failed
        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();

        let failed_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Failed)
            .count();

        let ready_count = tasks
            .iter()
            .filter(|t| t.internal_status == InternalStatus::Ready)
            .count();

        // 3 agent-active tasks should be Failed
        assert_eq!(failed_count, 3);
        // 1 Ready task should remain Ready
        assert_eq!(ready_count, 1);
        // Execution should be paused
        assert!(execution_state.is_paused());
    }

    #[test]
    fn test_agent_active_statuses_constant() {
        // Verify the constant includes all expected statuses
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Executing));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::QaRefining));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::QaTesting));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Reviewing));
        assert!(AGENT_ACTIVE_STATUSES.contains(&InternalStatus::ReExecuting));

        // Non-agent-active statuses should not be included
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Ready));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Backlog));
        assert!(!AGENT_ACTIVE_STATUSES.contains(&InternalStatus::Failed));
    }

    #[test]
    fn test_default_trait() {
        let state = ExecutionState::default();
        assert!(!state.is_paused());
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.max_concurrent(), 2);
    }

    // ========================================
    // Event Emission Tests
    // ========================================

    #[test]
    fn test_emit_status_changed_does_not_panic() {
        let state = ExecutionState::new();
        state.increment_running();

        let handle = crate::testing::create_mock_app_handle();
        // Should not panic even with mock runtime
        state.emit_status_changed(&handle, "task_started");
    }

    #[test]
    fn test_emit_status_changed_reflects_current_state() {
        let state = ExecutionState::with_max_concurrent(4);
        state.increment_running();
        state.increment_running();
        state.pause();

        let handle = crate::testing::create_mock_app_handle();
        // Verify the method reads current state correctly
        // (emit itself is fire-and-forget, but we can verify state is consistent)
        assert!(state.is_paused());
        assert_eq!(state.running_count(), 2);
        assert_eq!(state.max_concurrent(), 4);
        state.emit_status_changed(&handle, "paused");
    }

    #[test]
    fn test_emit_status_changed_with_various_reasons() {
        let state = ExecutionState::new();
        let handle = crate::testing::create_mock_app_handle();

        // All valid reason strings should work without panic
        let reasons = ["task_started", "task_completed", "paused", "resumed", "stopped"];
        for reason in &reasons {
            state.emit_status_changed(&handle, reason);
        }
    }

    // ========================================
    // Integration Tests - Stop Execution
    // ========================================

    #[tokio::test]
    async fn test_stop_resets_running_count() {
        // Setup: Create tasks in agent-active states and simulate running count
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in agent-active statuses
        let mut task1 = Task::new(project.id.clone(), "Executing Task 1".to_string());
        task1.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task1.clone()).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Executing Task 2".to_string());
        task2.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task2.clone()).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Reviewing Task".to_string());
        task3.internal_status = InternalStatus::Reviewing;
        app_state.task_repo.create(task3.clone()).await.unwrap();

        // Simulate that running count matches agent-active tasks
        // (In real usage, spawner increments this when starting each task)
        execution_state.increment_running(); // task1
        execution_state.increment_running(); // task2
        execution_state.increment_running(); // task3
        assert_eq!(execution_state.running_count(), 3);

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Execute stop: pause and transition all agent-active tasks to Failed
        execution_state.pause();

        let tasks = app_state.task_repo.get_by_project(&project.id).await.unwrap();
        for task in tasks {
            if AGENT_ACTIVE_STATUSES.contains(&task.internal_status) {
                let _ = transition_service
                    .transition_task(&task.id, InternalStatus::Failed)
                    .await;
            }
        }

        // Verify: Running count should be 0 after all tasks transitioned
        // (on_exit handlers decrement for each agent-active state exit)
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should be 0 after stop cancels all tasks"
        );

        // Verify execution is paused
        assert!(execution_state.is_paused());
    }

    #[tokio::test]
    async fn test_running_count_decrements_on_task_completion() {
        // Setup: Create a task in Executing state
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(5));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create a task in Executing status
        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        app_state.task_repo.create(task.clone()).await.unwrap();

        // Simulate that running count was incremented when task started
        execution_state.increment_running();
        assert_eq!(execution_state.running_count(), 1);

        // Build transition service with execution state
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Transition task from Executing to Failed (simulating task cancellation)
        // Note: In real usage, task might go through QaRefining -> QaTesting -> QaPassed,
        // but for testing the decrement behavior, any exit from Executing is sufficient.
        let _ = transition_service
            .transition_task(&task.id, InternalStatus::Failed)
            .await;

        // Verify: Running count should have decremented
        // (on_exit handler for Executing state decrements)
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should decrement when task exits Executing state"
        );
    }

    #[tokio::test]
    async fn test_running_count_decrements_for_all_agent_active_states() {
        // Test that decrement works for all agent-active states, not just Executing
        let execution_state = Arc::new(ExecutionState::with_max_concurrent(10));
        let app_state = AppState::new_test();

        // Create a test project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks in different agent-active states
        let test_cases = [
            (InternalStatus::Executing, "Executing Task"),
            (InternalStatus::QaRefining, "QaRefining Task"),
            (InternalStatus::QaTesting, "QaTesting Task"),
            (InternalStatus::Reviewing, "Reviewing Task"),
            (InternalStatus::ReExecuting, "ReExecuting Task"),
        ];

        // Create all tasks and increment running count for each
        let mut task_ids = Vec::new();
        for (status, title) in &test_cases {
            let mut task = Task::new(project.id.clone(), title.to_string());
            task.internal_status = *status;
            app_state.task_repo.create(task.clone()).await.unwrap();
            task_ids.push(task.id);
            execution_state.increment_running();
        }

        assert_eq!(execution_state.running_count(), 5);

        // Build transition service
        let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(&execution_state),
            None,
        );

        // Transition each task to Failed (all should decrement running count)
        for task_id in &task_ids {
            let _ = transition_service
                .transition_task(task_id, InternalStatus::Failed)
                .await;
        }

        // Verify: Running count should be 0 after all tasks transitioned
        assert_eq!(
            execution_state.running_count(),
            0,
            "Running count should be 0 after all agent-active tasks exit their states"
        );
    }

    // ========================================
    // Integration Tests - Pause Prevents Spawns
    // ========================================
    // Note: Detailed spawn blocking tests are in spawner.rs:
    // - test_spawn_blocked_when_paused
    // - test_spawn_blocked_at_max_concurrent
    // - test_spawn_increments_running_count
    // These tests verify the ExecutionState integration with the spawner.
}
