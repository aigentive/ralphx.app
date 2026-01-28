use super::*;
use crate::commands::ExecutionState;
use crate::infrastructure::MockAgenticClient;

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
