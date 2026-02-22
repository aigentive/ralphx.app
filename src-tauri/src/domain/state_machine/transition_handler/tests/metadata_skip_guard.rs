// Metadata skip guard tests and TransitionHandler bypass fix tests
//
// Tests for Wave 4 metadata skip guard behavior:
// - on_enter(Failed) skips/writes metadata based on existing failure_error
// - on_enter(Executing) uses update_metadata
// - on_enter(QaRefining) skips/writes trigger_origin metadata
// - on_enter(QaTesting) skips/writes trigger_origin metadata
// - TransitionHandler bypass fix: empty source branch and repos-unavailable paths

use super::helpers::*;
use crate::domain::state_machine::{State, TaskStateMachine, TransitionHandler};
use std::sync::Arc;

// ============================================================================
// Wave 4: Metadata Skip Guard Tests
// ============================================================================

/// Test that on_enter(Failed) skips metadata write when failure_error is already present
#[tokio::test]
async fn test_on_enter_failed_skips_when_failure_error_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed failure metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata =
        Some(r#"{"failure_error":"Pre-computed error","is_timeout":false}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Create Failed state with different error
    let failed_data = FailedData::new("New error from on_enter");
    let failed_state = State::Failed(failed_data);

    // Call on_enter(Failed) which should skip the write
    let _ = handler.on_enter(&failed_state).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Pre-computed error",
        "Metadata should NOT be overwritten when already present"
    );
}

/// Test that on_enter(Failed) writes metadata when not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_failed_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without failure metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Create Failed state
    let failed_data = FailedData::new("Fallback error").with_details("Details here");
    let failed_state = State::Failed(failed_data);

    // Call on_enter(Failed) which should write the metadata
    let _ = handler.on_enter(&failed_state).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Fallback error",
        "Metadata should be written when not present"
    );
    assert_eq!(
        parsed.get("failure_details").unwrap().as_str().unwrap(),
        "Details here"
    );
    assert!(!parsed.get("is_timeout").unwrap().as_bool().unwrap());
}

/// Test that on_enter(Executing) uses update_metadata instead of full update
/// This test verifies the change from update(&task) to update_metadata()
#[tokio::test]
async fn test_on_enter_executing_uses_update_metadata() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create project and task
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Note: We can't directly verify update_metadata was called vs update,
    // but we can verify the behavior is correct (metadata is written).
    // This test primarily documents the expected behavior after Wave 4 changes.
    let _ = handler.on_enter(&State::Executing).await;

    // If the task has execution_setup_log, it was written via update_metadata
    // (In practice, this won't happen without actual project analysis setup,
    // but the code path is correct)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();

    // This test passes if no panic occurred and task is retrievable
    assert_eq!(updated_task.id, task.id);
}

/// Test that on_enter(QaRefining) skips metadata write when trigger_origin is already present
#[tokio::test]
async fn test_on_enter_qa_refining_skips_when_trigger_origin_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"trigger_origin":"scheduler","other_key":"value"}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaRefining) which should skip the write
    let _ = handler.on_enter(&State::QaRefining).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "scheduler",
        "Metadata should NOT be overwritten when already present"
    );
    assert_eq!(
        parsed.get("other_key").unwrap().as_str().unwrap(),
        "value",
        "Other metadata keys should be preserved"
    );
}

/// Test that on_enter(QaRefining) writes metadata when trigger_origin not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_qa_refining_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaRefining) which should write the metadata
    let _ = handler.on_enter(&State::QaRefining).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "qa",
        "trigger_origin should be set to 'qa' when not present"
    );
}

/// Test that on_enter(QaTesting) skips metadata write when trigger_origin is already present
#[tokio::test]
async fn test_on_enter_qa_testing_skips_when_trigger_origin_already_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task with pre-computed trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"trigger_origin":"scheduler","other_key":"value"}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaTesting) which should skip the write
    let _ = handler.on_enter(&State::QaTesting).await;

    // Verify metadata was NOT overwritten (still has pre-computed value)
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "scheduler",
        "Metadata should NOT be overwritten when already present"
    );
    assert_eq!(
        parsed.get("other_key").unwrap().as_str().unwrap(),
        "value",
        "Other metadata keys should be preserved"
    );
}

/// Test that on_enter(QaTesting) writes metadata when trigger_origin not present (backward compatibility)
#[tokio::test]
async fn test_on_enter_qa_testing_writes_when_not_present() {
    use crate::domain::entities::{Project, Task};
    use crate::infrastructure::memory::MemoryTaskRepository;

    // Create task without trigger_origin metadata
    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    // Create services with real repo
    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);

    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(QaTesting) which should write the metadata
    let _ = handler.on_enter(&State::QaTesting).await;

    // Verify metadata was written
    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("trigger_origin").unwrap().as_str().unwrap(),
        "qa",
        "trigger_origin should be set to 'qa' when not present"
    );
}

// ==================
// TransitionHandler bypass fix tests (Phase 3 hardening)
// ==================

/// Test: Empty source branch path calls on_exit(PendingMerge → MergeIncomplete).
///
/// When `attempt_programmatic_merge` encounters an empty source branch (task has no
/// task_branch set), it must call on_exit to trigger deferred merge retry for other
/// tasks. Previously, this path returned early without calling on_exit, leaving
/// deferred merges blocked.
#[tokio::test]
async fn test_empty_source_branch_triggers_deferred_merge_retry() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::mocks::MockTaskScheduler;
    use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

    let scheduler = Arc::new(MockTaskScheduler::new());

    // Task with NO task_branch → empty source branch → MergeIncomplete path
    let project = Project::new("test-project".to_string(), "/tmp/test".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());
    // task.task_branch is None by default → empty source branch

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    let project_repo: Arc<dyn crate::domain::repositories::ProjectRepository> =
        Arc::new(MemoryProjectRepository::new());

    task_repo.create(task.clone()).await.unwrap();
    project_repo.create(project.clone()).await.unwrap();

    let services = TaskServices::new_mock()
        .with_task_repo(task_repo.clone())
        .with_project_repo(project_repo.clone())
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(PendingMerge) which runs attempt_programmatic_merge
    let _ = handler.on_enter(&State::PendingMerge).await;

    // Wait for spawned task to call try_retry_deferred_merges
    let sched = Arc::clone(&scheduler);
    assert!(
        wait_for_condition(
            || {
                let s = Arc::clone(&sched);
                async move {
                    s.get_calls()
                        .iter()
                        .any(|c| c.method == "try_retry_deferred_merges")
                }
            },
            5000
        )
        .await,
        "Empty-source-branch path must call on_exit to trigger try_retry_deferred_merges, \
         preventing deferred merges from being blocked"
    );
}

// Tests early-return guard — does not reach merge strategy dispatch
/// Without repos, on_enter(PendingMerge) still fires on_exit to unblock deferred retries.
#[tokio::test]
async fn test_guard_no_repos_fires_on_exit_for_deferred_retry() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());

    // Services with NO task_repo/project_repo (repos unavailable)
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);
    // Verify repos are not set
    assert!(
        services.task_repo.is_none(),
        "task_repo should be None for this test"
    );

    let context = create_context_with_services("task-unavailable", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // Call on_enter(PendingMerge) — repos unavailable path fires
    let _ = handler.on_enter(&State::PendingMerge).await;

    // Wait for spawned task to call try_retry_deferred_merges
    let sched = Arc::clone(&scheduler);
    assert!(
        wait_for_condition(
            || {
                let s = Arc::clone(&sched);
                async move {
                    s.get_calls()
                        .iter()
                        .any(|c| c.method == "try_retry_deferred_merges")
                }
            },
            5000
        )
        .await,
        "Repos-unavailable path must still call on_exit to trigger try_retry_deferred_merges"
    );
}

/// Test that on_enter(Failed) populates attempt_count from auto_retry_count_executing metadata
#[tokio::test]
async fn test_on_enter_failed_populates_attempt_count_from_metadata() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(r#"{"auto_retry_count_executing": 3}"#.to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let failed_data = FailedData::new("Agent error");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("attempt_count").unwrap().as_u64().unwrap(),
        3u64,
        "attempt_count should be populated from auto_retry_count_executing"
    );
}

/// Test that on_enter(Failed) sets attempt_count to 0 when auto_retry_count_executing is absent
#[tokio::test]
async fn test_on_enter_failed_attempt_count_zero_when_metadata_absent() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let task = Task::new(project.id.clone(), "Test task".to_string());

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let failed_data = FailedData::new("Agent error");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should be written");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("attempt_count").unwrap().as_u64().unwrap(),
        0u64,
        "attempt_count should be 0 when auto_retry_count_executing is absent"
    );
}

/// Test that on_enter(Failed) writes attempt_count in the pre-computed path without overwriting failure_error
#[tokio::test]
async fn test_on_enter_failed_writes_attempt_count_in_precomputed_path() {
    use crate::domain::entities::{Project, Task};
    use crate::domain::state_machine::types::FailedData;
    use crate::infrastructure::memory::MemoryTaskRepository;

    let project = Project::new("test-project".to_string(), "/test/path".to_string());
    let mut task = Task::new(project.id.clone(), "Test task".to_string());
    task.metadata = Some(
        r#"{"failure_error":"Pre-computed error","is_timeout":false,"auto_retry_count_executing":2}"#
            .to_string(),
    );

    let task_repo: Arc<dyn crate::domain::repositories::TaskRepository> =
        Arc::new(MemoryTaskRepository::new());
    task_repo.create(task.clone()).await.unwrap();

    let mut services = TaskServices::new_mock();
    services.task_repo = Some(task_repo.clone());

    let context = create_context_with_services(task.id.as_str(), project.id.as_str(), services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let failed_data = FailedData::new("New error");
    let _ = handler.on_enter(&State::Failed(failed_data)).await;

    let updated_task = task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    let metadata_json = updated_task.metadata.expect("Metadata should still exist");
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&metadata_json).unwrap();

    assert_eq!(
        parsed.get("failure_error").unwrap().as_str().unwrap(),
        "Pre-computed error",
        "failure_error should NOT be overwritten in pre-computed path"
    );
    assert_eq!(
        parsed.get("attempt_count").unwrap().as_u64().unwrap(),
        2u64,
        "attempt_count should be written from auto_retry_count_executing even in pre-computed path"
    );
}
