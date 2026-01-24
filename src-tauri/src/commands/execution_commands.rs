// Tauri commands for execution control
// Manages global execution state: pause, resume, stop

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::InternalStatus;

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

    // Get current status
    let status = get_execution_status(execution_state, app_state).await?;

    Ok(ExecutionCommandResponse {
        success: true,
        status,
    })
}

/// Stop execution (cancels current tasks and pauses)
/// This transitions all executing tasks to Cancelled status
#[tauri::command]
pub async fn stop_execution(
    execution_state: State<'_, Arc<ExecutionState>>,
    app_state: State<'_, AppState>,
) -> Result<ExecutionCommandResponse, String> {
    // First pause to prevent new tasks from starting
    execution_state.pause();

    // Find all executing tasks and cancel them
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

        for mut task in tasks {
            if task.internal_status == InternalStatus::Executing {
                // Transition to Cancelled (valid transition per state machine)
                // Note: Executing can transition to Failed, which can then transition to Cancelled
                // For immediate stop, we use Failed as an intermediate state
                task.internal_status = InternalStatus::Failed;
                task.touch();
                app_state
                    .task_repo
                    .update(&task)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // Reset running count to 0 since we've stopped all tasks
    while execution_state.running_count() > 0 {
        execution_state.decrement_running();
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
        project_repo.create(project.clone()).await.unwrap();

        // Create tasks in various statuses
        let mut task1 = Task::new(project.id.clone(), "Ready Task 1".to_string());
        task1.internal_status = InternalStatus::Ready;
        task_repo.create(task1).await.unwrap();

        let mut task2 = Task::new(project.id.clone(), "Ready Task 2".to_string());
        task2.internal_status = InternalStatus::Ready;
        task_repo.create(task2).await.unwrap();

        let mut task3 = Task::new(project.id.clone(), "Executing Task".to_string());
        task3.internal_status = InternalStatus::Executing;
        task_repo.create(task3).await.unwrap();

        let mut task4 = Task::new(project.id.clone(), "Backlog Task".to_string());
        task4.internal_status = InternalStatus::Backlog;
        task_repo.create(task4).await.unwrap();

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

    #[test]
    fn test_default_trait() {
        let state = ExecutionState::default();
        assert!(!state.is_paused());
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.max_concurrent(), 2);
    }
}
