use super::*;
use crate::application::AppState;
use crate::domain::entities::{InternalStatus, Project, Task};
use crate::domain::services::{MemoryRunningAgentRegistry, MessageQueue};
use crate::domain::state_machine::transition_handler::metadata_builder::MetadataUpdate;
use serde_json::Value;

#[test]
fn test_tauri_event_emitter_creation() {
    let emitter: TauriEventEmitter<tauri::Wry> = TauriEventEmitter::new(None);
    assert!(emitter.app_handle.is_none());
}

#[test]
fn test_logging_notifier() {
    let _notifier = LoggingNotifier;
    // Just verify it can be created
}

#[test]
fn test_no_op_review_starter() {
    let _starter = NoOpReviewStarter;
    // Just verify it can be created
}

fn build_dependency_manager(app_state: &AppState) -> RepoBackedDependencyManager<tauri::Wry> {
    RepoBackedDependencyManager::new(
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.task_repo),
        None,
    )
}

#[tokio::test]
async fn test_dependency_manager_treats_paused_blocker_as_incomplete() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Paused Blocker".to_string());
    blocker.internal_status = InternalStatus::Paused;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut blocked = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&blocked.id, &blocker.id)
        .await
        .unwrap();

    let has_blockers = manager.has_unresolved_blockers(blocked.id.as_str()).await;
    assert!(
        has_blockers,
        "Paused blockers should be treated as unresolved"
    );
}

#[tokio::test]
async fn test_dependency_manager_treats_stopped_blocker_as_incomplete() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Stopped Blocker".to_string());
    blocker.internal_status = InternalStatus::Stopped;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut blocked = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&blocked.id, &blocker.id)
        .await
        .unwrap();

    let has_blockers = manager.has_unresolved_blockers(blocked.id.as_str()).await;
    assert!(
        has_blockers,
        "Stopped blockers should be treated as unresolved"
    );
}

// ============================================================================
// Wave 3: Metadata Merge Tests
// ============================================================================

fn build_test_service(app_state: &AppState) -> TaskTransitionService<tauri::Wry> {
    let execution_state = Arc::new(ExecutionState::new());
    let message_queue = Arc::new(MessageQueue::new());
    let running_registry = Arc::new(MemoryRunningAgentRegistry::new());

    TaskTransitionService::new(
        Arc::clone(&app_state.task_repo),
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.project_repo),
        Arc::clone(&app_state.chat_message_repo),
        Arc::clone(&app_state.chat_conversation_repo),
        Arc::clone(&app_state.agent_run_repo),
        Arc::clone(&app_state.ideation_session_repo),
        Arc::clone(&app_state.activity_event_repo),
        message_queue,
        running_registry,
        execution_state,
        None,
        Arc::clone(&app_state.memory_event_repo),
    )
}

#[tokio::test]
async fn test_transition_task_with_metadata_update_persists_atomically() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Test Task".to_string());
    task.internal_status = InternalStatus::Backlog;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let metadata_update = MetadataUpdate::new()
        .with_string("custom_key", "custom_value")
        .with_bool("is_test", true);

    let updated_task = service
        .transition_task_with_metadata(&task.id, InternalStatus::Ready, Some(metadata_update))
        .await
        .unwrap();

    assert_eq!(updated_task.internal_status, InternalStatus::Ready);

    let metadata_json = updated_task.metadata.expect("Metadata should be set");
    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("custom_key").unwrap(),
        &Value::String("custom_value".to_string())
    );
    assert_eq!(parsed.get("is_test").unwrap(), &Value::Bool(true));
}

#[tokio::test]
async fn test_transition_task_with_none_preserves_existing_metadata() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Test Task".to_string());
    task.internal_status = InternalStatus::Backlog;
    task.metadata = Some(r#"{"existing_key":"existing_value"}"#.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated_task = service
        .transition_task_with_metadata(&task.id, InternalStatus::Ready, None)
        .await
        .unwrap();

    assert_eq!(updated_task.internal_status, InternalStatus::Ready);

    let metadata_json = updated_task.metadata.expect("Metadata should be preserved");
    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("existing_key").unwrap(),
        &Value::String("existing_value".to_string())
    );
}

#[tokio::test]
async fn test_qa_refining_transition_auto_adds_trigger_origin() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Test Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated_task = service
        .transition_task_with_metadata(&task.id, InternalStatus::QaRefining, None)
        .await
        .unwrap();

    assert_eq!(updated_task.internal_status, InternalStatus::QaRefining);

    let metadata_json = updated_task
        .metadata
        .expect("Metadata should have trigger_origin");
    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap(),
        &Value::String("qa".to_string())
    );
}

#[tokio::test]
async fn test_qa_testing_transition_auto_adds_trigger_origin() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Test Task".to_string());
    task.internal_status = InternalStatus::QaRefining;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated_task = service
        .transition_task_with_metadata(&task.id, InternalStatus::QaTesting, None)
        .await
        .unwrap();

    assert_eq!(updated_task.internal_status, InternalStatus::QaTesting);

    let metadata_json = updated_task
        .metadata
        .expect("Metadata should have trigger_origin");
    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap(),
        &Value::String("qa".to_string())
    );
}

#[tokio::test]
async fn test_metadata_merge_preserves_existing_keys_not_in_update() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state
        .project_repo
        .create(project.clone())
        .await
        .unwrap();

    let mut task = Task::new(project.id.clone(), "Test Task".to_string());
    task.internal_status = InternalStatus::Backlog;
    task.metadata =
        Some(r#"{"existing_key":"existing_value","another_key":"another_value"}"#.to_string());
    app_state.task_repo.create(task.clone()).await.unwrap();

    let metadata_update = MetadataUpdate::new().with_string("new_key", "new_value");

    let updated_task = service
        .transition_task_with_metadata(&task.id, InternalStatus::Ready, Some(metadata_update))
        .await
        .unwrap();

    let metadata_json = updated_task.metadata.expect("Metadata should be merged");
    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("existing_key").unwrap(),
        &Value::String("existing_value".to_string())
    );
    assert_eq!(
        parsed.get("another_key").unwrap(),
        &Value::String("another_value".to_string())
    );
    assert_eq!(
        parsed.get("new_key").unwrap(),
        &Value::String("new_value".to_string())
    );
}
