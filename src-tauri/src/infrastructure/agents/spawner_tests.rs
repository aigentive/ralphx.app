use super::*;
use crate::commands::ExecutionState;
use crate::domain::entities::{GitMode, Project, ProjectId, Task, TaskId};
use crate::domain::repositories::{
    ProjectRepository, StatusTransition, StateHistoryMetadata, TaskRepository,
};
use crate::error::AppResult;
use crate::infrastructure::MockAgenticClient;

// ==================== Mock Repos for CWD Tests ====================

/// Minimal mock TaskRepository that returns a configurable task
struct MockTaskRepoForSpawner {
    task: Option<Task>,
}

#[async_trait]
impl TaskRepository for MockTaskRepoForSpawner {
    async fn create(&self, task: Task) -> AppResult<Task> { Ok(task) }
    async fn get_by_id(&self, _id: &TaskId) -> AppResult<Option<Task>> {
        Ok(self.task.clone())
    }
    async fn get_by_project(&self, _: &ProjectId) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn update(&self, _: &Task) -> AppResult<()> { Ok(()) }
    async fn delete(&self, _: &TaskId) -> AppResult<()> { Ok(()) }
    async fn get_by_status(&self, _: &ProjectId, _: crate::domain::entities::InternalStatus) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn persist_status_change(&self, _: &TaskId, _: crate::domain::entities::InternalStatus, _: crate::domain::entities::InternalStatus, _: &str) -> AppResult<()> { Ok(()) }
    async fn get_status_history(&self, _: &TaskId) -> AppResult<Vec<StatusTransition>> { Ok(vec![]) }
    async fn get_next_executable(&self, _: &ProjectId) -> AppResult<Option<Task>> { Ok(None) }
    async fn get_blockers(&self, _: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn get_dependents(&self, _: &TaskId) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn add_blocker(&self, _: &TaskId, _: &TaskId) -> AppResult<()> { Ok(()) }
    async fn resolve_blocker(&self, _: &TaskId, _: &TaskId) -> AppResult<()> { Ok(()) }
    async fn get_by_ideation_session(&self, _: &crate::domain::entities::IdeationSessionId) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn get_by_project_filtered(&self, _: &ProjectId, _: bool) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn archive(&self, _: &TaskId) -> AppResult<Task> { unimplemented!() }
    async fn restore(&self, _: &TaskId) -> AppResult<Task> { unimplemented!() }
    async fn get_archived_count(&self, _: &ProjectId) -> AppResult<u32> { Ok(0) }
    async fn list_paginated(&self, _: &ProjectId, _: Option<Vec<crate::domain::entities::InternalStatus>>, _: u32, _: u32, _: bool) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn count_tasks(&self, _: &ProjectId, _: bool) -> AppResult<u32> { Ok(0) }
    async fn search(&self, _: &ProjectId, _: &str, _: bool) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> { Ok(None) }
    async fn get_oldest_ready_tasks(&self, _: u32) -> AppResult<Vec<Task>> { Ok(vec![]) }
    async fn update_latest_state_history_metadata(&self, _: &TaskId, _: &StateHistoryMetadata) -> AppResult<()> { Ok(()) }
    async fn has_task_in_states(&self, _: &ProjectId, _: &[crate::domain::entities::InternalStatus]) -> AppResult<bool> { Ok(false) }
}

/// Minimal mock ProjectRepository that returns a configurable project
struct MockProjectRepoForSpawner {
    project: Option<Project>,
}

#[async_trait]
impl ProjectRepository for MockProjectRepoForSpawner {
    async fn create(&self, project: Project) -> AppResult<Project> { Ok(project) }
    async fn get_by_id(&self, _: &ProjectId) -> AppResult<Option<Project>> {
        Ok(self.project.clone())
    }
    async fn get_all(&self) -> AppResult<Vec<Project>> { Ok(vec![]) }
    async fn update(&self, _: &Project) -> AppResult<()> { Ok(()) }
    async fn delete(&self, _: &ProjectId) -> AppResult<()> { Ok(()) }
    async fn get_by_working_directory(&self, _: &str) -> AppResult<Option<Project>> { Ok(None) }
}

#[test]
fn test_role_from_string() {
    // Standard roles
    assert_eq!(AgenticClientSpawner::role_from_string("worker"), AgentRole::Worker);
    assert_eq!(AgenticClientSpawner::role_from_string("qa-prep"), AgentRole::QaPrep);
    assert_eq!(AgenticClientSpawner::role_from_string("qa-refiner"), AgentRole::QaRefiner);
    assert_eq!(AgenticClientSpawner::role_from_string("qa-tester"), AgentRole::QaTester);
    assert_eq!(AgenticClientSpawner::role_from_string("reviewer"), AgentRole::Reviewer);
    assert_eq!(AgenticClientSpawner::role_from_string("supervisor"), AgentRole::Supervisor);
    // Custom role
    assert_eq!(AgenticClientSpawner::role_from_string("my-custom-agent"), AgentRole::Custom("my-custom-agent".to_string()));
}

#[tokio::test]
async fn test_spawn_calls_client() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    spawner.spawn("worker", "task-123").await;

    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn test_spawn_uses_correct_role() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    spawner.spawn("reviewer", "task-456").await;

    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
    if let crate::infrastructure::MockCallType::Spawn { role, prompt } = &calls[0].call_type {
        assert_eq!(*role, AgentRole::Reviewer);
        assert!(prompt.contains("task-456"));
    } else {
        panic!("Expected Spawn call");
    }
}

#[tokio::test]
async fn test_spawn_background_calls_client() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    spawner.spawn_background("qa-prep", "task-789").await;

    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn test_with_working_dir() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_working_dir("/custom/work/dir");

    assert_eq!(spawner.working_directory, PathBuf::from("/custom/work/dir"));
}

#[tokio::test]
async fn test_wait_for_is_noop() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    // Should not panic or error
    spawner.wait_for("worker", "task-123").await;
}

// ==================== Event Bus Tests ====================

#[tokio::test]
async fn test_with_event_bus() {
    let mock = Arc::new(MockAgenticClient::new());
    let event_bus = Arc::new(EventBus::new());
    let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus.clone());

    assert!(spawner.event_bus().is_some());
}

#[tokio::test]
async fn test_spawn_emits_task_start_event() {
    let mock = Arc::new(MockAgenticClient::new());
    let event_bus = Arc::new(EventBus::new());
    let mut subscriber = event_bus.subscribe();

    let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

    spawner.spawn("worker", "task-123").await;

    // Check that TaskStart event was emitted
    let event = subscriber.try_recv().unwrap();
    if let SupervisorEvent::TaskStart {
        task_id,
        agent_role,
        ..
    } = event
    {
        assert_eq!(task_id, "task-123");
        assert_eq!(agent_role, "worker");
    } else {
        panic!("Expected TaskStart event, got {:?}", event);
    }
}

#[tokio::test]
async fn test_emit_tool_call() {
    let mock = Arc::new(MockAgenticClient::new());
    let event_bus = Arc::new(EventBus::new());
    let mut subscriber = event_bus.subscribe();

    let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

    let info = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
    spawner.emit_tool_call("task-123", info);

    let event = subscriber.try_recv().unwrap();
    if let SupervisorEvent::ToolCall { task_id, info } = event {
        assert_eq!(task_id, "task-123");
        assert_eq!(info.tool_name, "Write");
    } else {
        panic!("Expected ToolCall event, got {:?}", event);
    }
}

#[tokio::test]
async fn test_emit_error() {
    let mock = Arc::new(MockAgenticClient::new());
    let event_bus = Arc::new(EventBus::new());
    let mut subscriber = event_bus.subscribe();

    let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

    let info = ErrorInfo::new("Something went wrong", "test_source");
    spawner.emit_error("task-123", info);

    let event = subscriber.try_recv().unwrap();
    if let SupervisorEvent::Error { task_id, info } = event {
        assert_eq!(task_id, "task-123");
        assert_eq!(info.message, "Something went wrong");
        assert_eq!(info.source, "test_source");
    } else {
        panic!("Expected Error event, got {:?}", event);
    }
}

#[tokio::test]
async fn test_spawn_without_event_bus_works() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    // Should not panic even without event bus
    spawner.spawn("worker", "task-123").await;

    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn test_emit_without_event_bus_is_noop() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    // Should not panic
    let info = ToolCallInfo::new("Read", "{}");
    spawner.emit_tool_call("task-123", info);

    let error_info = ErrorInfo::new("Test error", "test");
    spawner.emit_error("task-123", error_info);
}

#[tokio::test]
async fn test_multiple_spawns_emit_multiple_events() {
    let mock = Arc::new(MockAgenticClient::new());
    let event_bus = Arc::new(EventBus::new());
    let mut subscriber = event_bus.subscribe();

    let spawner = AgenticClientSpawner::new(mock.clone()).with_event_bus(event_bus);

    spawner.spawn("worker", "task-1").await;
    spawner.spawn("reviewer", "task-2").await;
    spawner.spawn("supervisor", "task-3").await;

    // Check all three events
    let event1 = subscriber.try_recv().unwrap();
    let event2 = subscriber.try_recv().unwrap();
    let event3 = subscriber.try_recv().unwrap();

    assert_eq!(event1.task_id(), "task-1");
    assert_eq!(event2.task_id(), "task-2");
    assert_eq!(event3.task_id(), "task-3");
}

// ==================== Execution State Tests ====================

#[tokio::test]
async fn test_with_execution_state() {
    let mock = Arc::new(MockAgenticClient::new());
    let exec_state = Arc::new(ExecutionState::new());
    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

    assert!(spawner.execution_state.is_some());
}

#[tokio::test]
async fn test_spawn_blocked_when_paused() {
    let mock = Arc::new(MockAgenticClient::new());
    let exec_state = Arc::new(ExecutionState::new());

    // Pause execution
    exec_state.pause();

    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

    // Try to spawn while paused
    spawner.spawn("worker", "task-123").await;

    // Verify no spawn occurred
    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 0, "Should not spawn when paused");

    // Running count should not have incremented
    assert_eq!(exec_state.running_count(), 0);
}

#[tokio::test]
async fn test_spawn_blocked_at_max_concurrent() {
    let mock = Arc::new(MockAgenticClient::new());
    let exec_state = Arc::new(ExecutionState::with_max_concurrent(2));

    // Fill up to max concurrent
    exec_state.increment_running();
    exec_state.increment_running();

    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

    // Try to spawn at max concurrent
    spawner.spawn("worker", "task-123").await;

    // Verify no spawn occurred
    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 0, "Should not spawn when at max concurrent");

    // Running count should still be 2 (not incremented)
    assert_eq!(exec_state.running_count(), 2);
}

#[tokio::test]
async fn test_spawn_increments_running_count() {
    let mock = Arc::new(MockAgenticClient::new());
    let exec_state = Arc::new(ExecutionState::with_max_concurrent(5));

    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

    // Verify initial state
    assert_eq!(exec_state.running_count(), 0);

    // Spawn a task
    spawner.spawn("worker", "task-1").await;

    // Verify running count incremented
    assert_eq!(exec_state.running_count(), 1);

    // Spawn another task
    spawner.spawn("reviewer", "task-2").await;

    // Verify running count incremented again
    assert_eq!(exec_state.running_count(), 2);

    // Verify spawns actually occurred
    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 2);
}

#[tokio::test]
async fn test_spawn_without_execution_state_still_works() {
    let mock = Arc::new(MockAgenticClient::new());
    // No execution state attached
    let spawner = AgenticClientSpawner::new(mock.clone());

    // Should still spawn normally
    spawner.spawn("worker", "task-123").await;

    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
}

// ==================== App Handle Tests ====================

#[test]
fn test_app_handle_defaults_to_none() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    // By default, app_handle should be None
    assert!(spawner.app_handle.is_none());
}

#[test]
fn test_app_handle_field_accessible() {
    let mock = Arc::new(MockAgenticClient::new());
    let spawner = AgenticClientSpawner::new(mock.clone());

    // Verify app_handle can be accessed (compile-time check + runtime assertion)
    // Note: with_app_handle() requires a real AppHandle<Wry> which is not available in tests,
    // but we verify the field exists and defaults correctly.
    let _handle: &Option<AppHandle<Wry>> = &spawner.app_handle;
    assert!(spawner.app_handle.is_none());
}

#[tokio::test]
async fn test_spawn_with_execution_state_no_app_handle_does_not_panic() {
    // Verifies that spawn() handles the case where execution_state is Some
    // but app_handle is None (the emit_status_changed call is skipped gracefully).
    // Note: Actual event emission with app_handle requires a real Wry runtime,
    // which is tested via integration tests and execution_commands.rs emit tests.
    let mock = Arc::new(MockAgenticClient::new());
    let exec_state = Arc::new(ExecutionState::with_max_concurrent(5));

    // No app_handle attached, but execution_state is present
    let spawner =
        AgenticClientSpawner::new(mock.clone()).with_execution_state(exec_state.clone());

    // Should spawn without panicking (emit_status_changed is skipped when app_handle is None)
    spawner.spawn("worker", "task-123").await;

    // Verify spawn occurred and running count incremented
    let calls = mock.get_spawn_calls().await;
    assert_eq!(calls.len(), 1);
    assert_eq!(exec_state.running_count(), 1);
}

// ==================== Per-Task CWD Resolution Tests ====================

#[tokio::test]
async fn test_resolve_working_directory_worktree_mode() {
    let mock = Arc::new(MockAgenticClient::new());

    let project_id = ProjectId("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "Test task".to_string());
    task.id = TaskId("task-worktree".to_string());
    task.worktree_path = Some("/worktrees/task-worktree".to_string());

    let mut project = Project::new("Test Project".to_string(), "/project/root".to_string());
    project.id = project_id;
    project.git_mode = GitMode::Worktree;

    let task_repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepoForSpawner {
        task: Some(task),
    });
    let project_repo: Arc<dyn ProjectRepository> = Arc::new(MockProjectRepoForSpawner {
        project: Some(project),
    });

    let spawner = AgenticClientSpawner::new(mock)
        .with_working_dir("/fallback")
        .with_repos(task_repo, project_repo);

    let resolved = spawner.resolve_working_directory("task-worktree").await;
    assert_eq!(resolved, PathBuf::from("/worktrees/task-worktree"));
}

#[tokio::test]
async fn test_resolve_working_directory_worktree_mode_no_worktree_path() {
    let mock = Arc::new(MockAgenticClient::new());

    let project_id = ProjectId("proj-2".to_string());
    let mut task = Task::new(project_id.clone(), "Test task".to_string());
    task.id = TaskId("task-no-wt".to_string());
    // worktree_path is None

    let mut project = Project::new("Test Project".to_string(), "/project/root".to_string());
    project.id = project_id;
    project.git_mode = GitMode::Worktree;

    let task_repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepoForSpawner {
        task: Some(task),
    });
    let project_repo: Arc<dyn ProjectRepository> = Arc::new(MockProjectRepoForSpawner {
        project: Some(project),
    });

    let spawner = AgenticClientSpawner::new(mock)
        .with_working_dir("/fallback")
        .with_repos(task_repo, project_repo);

    // Falls back to project working_directory when worktree_path is None
    let resolved = spawner.resolve_working_directory("task-no-wt").await;
    assert_eq!(resolved, PathBuf::from("/project/root"));
}

#[tokio::test]
async fn test_resolve_working_directory_local_mode() {
    let mock = Arc::new(MockAgenticClient::new());

    let project_id = ProjectId("proj-3".to_string());
    let mut task = Task::new(project_id.clone(), "Test task".to_string());
    task.id = TaskId("task-local".to_string());
    task.worktree_path = Some("/worktrees/task-local".to_string()); // should be ignored

    let mut project = Project::new("Test Project".to_string(), "/project/root".to_string());
    project.id = project_id;
    project.git_mode = GitMode::Local;

    let task_repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepoForSpawner {
        task: Some(task),
    });
    let project_repo: Arc<dyn ProjectRepository> = Arc::new(MockProjectRepoForSpawner {
        project: Some(project),
    });

    let spawner = AgenticClientSpawner::new(mock)
        .with_working_dir("/fallback")
        .with_repos(task_repo, project_repo);

    // Local mode always uses project working_directory
    let resolved = spawner.resolve_working_directory("task-local").await;
    assert_eq!(resolved, PathBuf::from("/project/root"));
}

#[tokio::test]
async fn test_resolve_working_directory_fallback_no_repos() {
    let mock = Arc::new(MockAgenticClient::new());

    // No repos attached — should fall back to self.working_directory
    let spawner = AgenticClientSpawner::new(mock).with_working_dir("/fallback");

    let resolved = spawner.resolve_working_directory("any-task").await;
    assert_eq!(resolved, PathBuf::from("/fallback"));
}

#[tokio::test]
async fn test_resolve_working_directory_fallback_task_not_found() {
    let mock = Arc::new(MockAgenticClient::new());

    // Repos attached but task not found
    let task_repo: Arc<dyn TaskRepository> = Arc::new(MockTaskRepoForSpawner {
        task: None,
    });
    let project_repo: Arc<dyn ProjectRepository> = Arc::new(MockProjectRepoForSpawner {
        project: None,
    });

    let spawner = AgenticClientSpawner::new(mock)
        .with_working_dir("/fallback")
        .with_repos(task_repo, project_repo);

    let resolved = spawner.resolve_working_directory("nonexistent-task").await;
    assert_eq!(resolved, PathBuf::from("/fallback"));
}
