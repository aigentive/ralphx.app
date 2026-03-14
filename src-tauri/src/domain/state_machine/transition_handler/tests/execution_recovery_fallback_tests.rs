// Unit tests for on_enter(Failed) execution_recovery fallback safety net.
//
// Covers three scenarios:
// 1. No pre-write → fallback writes execution_recovery with Retrying + stop_retrying=false
// 2. Pre-written execution_recovery present → fallback does NOT overwrite
// 3. Chat service pre-writes failure_error + execution_recovery → both preserved unchanged

use super::helpers::*;
use crate::domain::state_machine::{State, TaskStateMachine, TransitionHandler};

/// Test: transition to Failed without any pre-write → fallback writes execution_recovery
/// with last_state=Retrying, stop_retrying=false, and one Failed event.
/// Also verifies failed_at is written.
/// This covers path #5 (empty output) and any future unknown failure paths.
#[tokio::test]
async fn test_on_enter_failed_without_prewrite_writes_fallback_recovery() {
    use crate::domain::entities::{
        ExecutionRecoveryMetadata, ExecutionRecoveryState, Project, Task,
    };
    use crate::domain::state_machine::types::FailedData;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    // Task with no metadata at all — simulates empty-output path (#5)
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let failed_data = FailedData::new("Agent completed with no output");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");

    // Parse execution_recovery
    let recovery = ExecutionRecoveryMetadata::from_task_metadata(Some(&metadata_json))
        .expect("Parsing should succeed")
        .expect("execution_recovery should be present after fallback");

    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "Fallback should write Retrying state so reconciler can pick up the task"
    );
    assert!(
        !recovery.stop_retrying,
        "stop_retrying must be false — task should be eligible for auto-recovery"
    );
    assert_eq!(
        recovery.events.len(),
        1,
        "Exactly one Failed event should be appended by the fallback"
    );

    // Verify failed_at was also written
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();
    assert!(
        parsed.contains_key("failed_at"),
        "failed_at should be written when absent"
    );
}

/// Test: transition to Failed WITH pre-written execution_recovery → fallback does NOT overwrite.
/// Simulates a terminal path (E7 retry-limit) that pre-writes stop_retrying=true.
/// The fallback is_none() check must skip because execution_recovery already exists.
#[tokio::test]
async fn test_on_enter_failed_with_prewritten_recovery_does_not_overwrite() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoveryReasonCode,
        ExecutionRecoverySource, ExecutionRecoveryState, Project, Task,
    };
    use crate::domain::state_machine::types::FailedData;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());

    // Simulate E7 terminal pre-write: stop_retrying=true, last_state=Failed
    let mut terminal_recovery = ExecutionRecoveryMetadata::new();
    terminal_recovery.stop_retrying = true;
    terminal_recovery.append_event_with_state(
        crate::domain::entities::ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::StopRetrying,
            ExecutionRecoverySource::System,
            ExecutionRecoveryReasonCode::MaxRetriesExceeded,
            "Executing retry limit exceeded (3/3)",
        ),
        ExecutionRecoveryState::Failed,
    );

    let recovery_json = serde_json::to_string(&terminal_recovery).unwrap();
    task.metadata = Some(format!(
        r#"{{"failure_error":"Max retries exceeded","execution_recovery":{}}}"#,
        recovery_json
    ));

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let failed_data = FailedData::new("Max retries exceeded");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should exist");

    let recovery = ExecutionRecoveryMetadata::from_task_metadata(Some(&metadata_json))
        .expect("Parsing should succeed")
        .expect("execution_recovery should still be present");

    assert!(
        recovery.stop_retrying,
        "stop_retrying must remain true — fallback must NOT overwrite terminal pre-write"
    );
    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Failed,
        "last_state must remain Failed — fallback must NOT change to Retrying"
    );
    assert_eq!(
        recovery.events.len(),
        1,
        "Event count must not change — fallback must NOT append a new event"
    );
}

/// Test: chat service pre-writes both failure_error AND execution_recovery (streaming path).
/// on_enter(Failed) fires afterward → both are preserved unchanged.
/// Verifies the skip guard (failure_error present) and is_none() check together.
#[tokio::test]
async fn test_on_enter_failed_chat_service_prewrite_preserved() {
    use crate::domain::entities::{
        ExecutionRecoveryEventKind, ExecutionRecoveryMetadata, ExecutionRecoveryReasonCode,
        ExecutionRecoverySource, ExecutionRecoveryState, Project, Task,
    };
    use crate::domain::state_machine::types::FailedData;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());

    // Simulate chat service pre-write: failure_error present + Retrying recovery with 1 event
    let mut chat_recovery = ExecutionRecoveryMetadata::new();
    chat_recovery.append_event_with_state(
        crate::domain::entities::ExecutionRecoveryEvent::new(
            ExecutionRecoveryEventKind::Failed,
            ExecutionRecoverySource::System,
            ExecutionRecoveryReasonCode::Timeout,
            "Agent stream timed out (chat service pre-write)",
        ),
        ExecutionRecoveryState::Retrying,
    );

    let recovery_json = serde_json::to_string(&chat_recovery).unwrap();
    task.metadata = Some(format!(
        r#"{{"failure_error":"Transient timeout","is_timeout":true,"failed_at":"2026-03-14T12:00:00Z","execution_recovery":{}}}"#,
        recovery_json
    ));

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // on_enter fires as it would after ExecutionFailed event
    let failed_data = FailedData::new("Transient timeout");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should exist");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    // failure_error from chat service must be preserved
    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Transient timeout",
        "failure_error must not be overwritten by on_enter"
    );

    // failed_at from chat service must be preserved (not replaced with newer timestamp)
    assert_eq!(
        parsed.get("failed_at").unwrap().as_str().unwrap(),
        "2026-03-14T12:00:00Z",
        "failed_at must not be overwritten when already present"
    );

    let recovery = ExecutionRecoveryMetadata::from_task_metadata(Some(&metadata_json))
        .expect("Parsing should succeed")
        .expect("execution_recovery should still be present");

    assert_eq!(
        recovery.events.len(),
        1,
        "No extra events must be appended — fallback must skip when recovery already exists"
    );
    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "last_state must remain as set by chat service"
    );
    assert!(
        !recovery.stop_retrying,
        "stop_retrying must remain false as set by chat service"
    );
}
