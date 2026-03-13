use super::*;
use crate::application::AppState;
use crate::domain::entities::{
    ExecutionFailureSource, ExecutionRecoveryEventKind, ExecutionRecoveryMetadata,
    ExecutionRecoveryReasonCode, ExecutionRecoverySource, ExecutionRecoveryState, InternalStatus,
    Project, Task,
};
use crate::domain::entities::task_metadata::GIT_ISOLATION_ERROR_PREFIX;
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

/// Stopped is terminal but does NOT satisfy dependencies — stopped tasks
/// have incomplete work, so dependents should remain blocked.
#[tokio::test]
async fn test_is_blocker_complete_with_stopped_state() {
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
        "Stopped blockers should still block dependents (incomplete work)"
    );
}

/// MergeIncomplete does NOT satisfy dependencies — merge failed, code not on target branch.
/// A task with a MergeIncomplete blocker should remain blocked.
#[tokio::test]
async fn test_is_blocker_complete_with_merge_incomplete_state() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "MergeIncomplete Blocker".to_string());
    blocker.internal_status = InternalStatus::MergeIncomplete;
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
        "MergeIncomplete blockers should NOT satisfy dependencies (merge failed)"
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
        Arc::clone(&app_state.chat_attachment_repo),
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

// ============================================================================
// Regression: merge unblocks dependent tasks
// ============================================================================

/// Regression test: when task A merges via the programmatic path (side_effects.rs),
/// task B which depends on A must be unblocked (Blocked → Ready).
///
/// Before the fix, complete_merge_internal bypassed TransitionHandler so on_enter(Merged)
/// never fired and unblock_dependents was never called. Blocked tasks stayed stuck forever.
#[tokio::test]
async fn test_merge_unblocks_dependent_task() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    // Task A: the dependency (blocker) — simulate it just merged
    let mut task_a = Task::new(project.id.clone(), "Task A (Blocker)".to_string());
    task_a.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(task_a.clone()).await.unwrap();

    // Task B: depends on A, currently blocked
    let mut task_b = Task::new(project.id.clone(), "Task B (Dependent)".to_string());
    task_b.internal_status = InternalStatus::Blocked;
    task_b.blocked_reason = Some(format!("Waiting for: {}", task_a.title));
    app_state.task_repo.create(task_b.clone()).await.unwrap();

    // Register dependency: B is blocked by A (B depends on A)
    app_state
        .task_dependency_repo
        .add_dependency(&task_b.id, &task_a.id)
        .await
        .unwrap();

    // Simulate what post_merge_cleanup now calls after complete_merge_internal succeeds
    manager.unblock_dependents(task_a.id.as_str()).await;

    // Assert B is now Ready
    let updated_b = app_state
        .task_repo
        .get_by_id(&task_b.id)
        .await
        .unwrap()
        .expect("Task B should still exist");

    assert_eq!(
        updated_b.internal_status,
        InternalStatus::Ready,
        "Task B should be unblocked to Ready after Task A merges"
    );
    assert!(
        updated_b.blocked_reason.is_none(),
        "Task B should have no blocked_reason after unblocking"
    );
}

/// Regression: unblock_dependents is idempotent — calling it twice does not cause errors
/// and a Ready task stays Ready (not double-transitioned).
#[tokio::test]
async fn test_merge_unblocks_dependent_task_idempotent() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut task_a = Task::new(project.id.clone(), "Task A (Blocker)".to_string());
    task_a.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(task_a.clone()).await.unwrap();

    let mut task_b = Task::new(project.id.clone(), "Task B (Dependent)".to_string());
    task_b.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(task_b.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&task_b.id, &task_a.id)
        .await
        .unwrap();

    // Call twice (defence-in-depth may call it from both post_merge_cleanup and chat_service_merge)
    manager.unblock_dependents(task_a.id.as_str()).await;
    manager.unblock_dependents(task_a.id.as_str()).await;

    let updated_b = app_state
        .task_repo
        .get_by_id(&task_b.id)
        .await
        .unwrap()
        .expect("Task B should still exist");

    assert_eq!(
        updated_b.internal_status,
        InternalStatus::Ready,
        "Task B should be Ready after idempotent unblock calls"
    );
}

/// When task A merges but task B has another blocker still incomplete,
/// task B should remain Blocked.
#[tokio::test]
async fn test_merge_does_not_unblock_task_with_remaining_blocker() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    // Task A merges
    let mut task_a = Task::new(project.id.clone(), "Task A".to_string());
    task_a.internal_status = InternalStatus::Merged;
    app_state.task_repo.create(task_a.clone()).await.unwrap();

    // Task C is still executing (incomplete blocker)
    let mut task_c = Task::new(project.id.clone(), "Task C (Still Running)".to_string());
    task_c.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task_c.clone()).await.unwrap();

    // Task B depends on both A and C
    let mut task_b = Task::new(project.id.clone(), "Task B (Dependent)".to_string());
    task_b.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(task_b.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&task_b.id, &task_a.id)
        .await
        .unwrap();
    app_state
        .task_dependency_repo
        .add_dependency(&task_b.id, &task_c.id)
        .await
        .unwrap();

    // A merges — but C is still running, so B should stay Blocked
    manager.unblock_dependents(task_a.id.as_str()).await;

    let updated_b = app_state
        .task_repo
        .get_by_id(&task_b.id)
        .await
        .unwrap()
        .expect("Task B should still exist");

    assert_eq!(
        updated_b.internal_status,
        InternalStatus::Blocked,
        "Task B should remain Blocked since Task C is still executing"
    );
}

// ============================================================================
// Team Mode Priority Tests
// ============================================================================

#[test]
fn test_team_mode_defaults_to_none() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);
    assert_eq!(
        service.team_mode, None,
        "Default team_mode should be None (unset)"
    );
}

#[test]
fn test_with_team_mode_true_sets_some_true() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state).with_team_mode(true);
    assert_eq!(
        service.team_mode,
        Some(true),
        "with_team_mode(true) should set Some(true)"
    );
}

#[test]
fn test_with_team_mode_false_sets_some_false() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state).with_team_mode(false);
    assert_eq!(
        service.team_mode,
        Some(false),
        "with_team_mode(false) should set Some(false), not None — explicit solo must skip metadata fallback"
    );
}

#[test]
fn test_with_team_mode_overrides_previous_value() {
    let app_state = AppState::new_test();
    // Start with team, then switch to solo
    let service = build_test_service(&app_state)
        .with_team_mode(true)
        .with_team_mode(false);
    assert_eq!(
        service.team_mode,
        Some(false),
        "Second with_team_mode call should override the first"
    );
}

// ============================================================================
// Hard-block dependents when a blocker fails
// ============================================================================

/// When a blocker fails, dependents must stay Blocked — not unblocked to Ready.
/// This prevents cascade execution against broken output.
#[tokio::test]
async fn test_failed_blocker_keeps_dependent_blocked() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    // Blocker fails during execution
    let mut blocker = Task::new(project.id.clone(), "Setup DB".to_string());
    blocker.internal_status = InternalStatus::Failed;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    // Dependent was blocked waiting for the blocker
    let mut dependent = Task::new(project.id.clone(), "Run Migrations".to_string());
    dependent.internal_status = InternalStatus::Blocked;
    dependent.blocked_reason = Some(format!("Waiting for: {}", blocker.title));
    app_state.task_repo.create(dependent.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&dependent.id, &blocker.id)
        .await
        .unwrap();

    // Simulate on_enter(Failed) calling unblock_dependents
    manager.unblock_dependents(blocker.id.as_str()).await;

    let updated = app_state
        .task_repo
        .get_by_id(&dependent.id)
        .await
        .unwrap()
        .expect("Dependent should still exist");

    assert_eq!(
        updated.internal_status,
        InternalStatus::Blocked,
        "Dependent should remain Blocked when blocker fails"
    );
}

/// When a blocker fails, the dependent's blocked_reason must mention the failed dependency.
#[tokio::test]
async fn test_failed_blocker_sets_blocked_reason_with_failure_message() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Setup DB".to_string());
    blocker.internal_status = InternalStatus::Failed;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut dependent = Task::new(project.id.clone(), "Run Migrations".to_string());
    dependent.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(dependent.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&dependent.id, &blocker.id)
        .await
        .unwrap();

    manager.unblock_dependents(blocker.id.as_str()).await;

    let updated = app_state
        .task_repo
        .get_by_id(&dependent.id)
        .await
        .unwrap()
        .expect("Dependent should still exist");

    let reason = updated
        .blocked_reason
        .expect("blocked_reason should be set when blocker fails");
    assert!(
        reason.contains("Setup DB") && reason.to_lowercase().contains("fail"),
        "blocked_reason should mention the failed dependency name and failure, got: {reason}"
    );
}

/// Mixed scenario: one blocker failed, another is still running — dependent stays Blocked.
#[tokio::test]
async fn test_mixed_failed_and_running_blockers_keeps_dependent_blocked() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut failed_blocker = Task::new(project.id.clone(), "Setup DB".to_string());
    failed_blocker.internal_status = InternalStatus::Failed;
    app_state
        .task_repo
        .create(failed_blocker.clone())
        .await
        .unwrap();

    let mut running_blocker = Task::new(project.id.clone(), "Build Assets".to_string());
    running_blocker.internal_status = InternalStatus::Executing;
    app_state
        .task_repo
        .create(running_blocker.clone())
        .await
        .unwrap();

    let mut dependent = Task::new(project.id.clone(), "Deploy".to_string());
    dependent.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(dependent.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&dependent.id, &failed_blocker.id)
        .await
        .unwrap();
    app_state
        .task_dependency_repo
        .add_dependency(&dependent.id, &running_blocker.id)
        .await
        .unwrap();

    manager.unblock_dependents(failed_blocker.id.as_str()).await;

    let updated = app_state
        .task_repo
        .get_by_id(&dependent.id)
        .await
        .unwrap()
        .expect("Dependent should still exist");

    assert_eq!(
        updated.internal_status,
        InternalStatus::Blocked,
        "Dependent should remain Blocked when both failed and running blockers exist"
    );
}

/// A Failed blocker treated as incomplete — has_unresolved_blockers returns true.
#[tokio::test]
async fn test_has_unresolved_blockers_treats_failed_as_unresolved() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Build Step".to_string());
    blocker.internal_status = InternalStatus::Failed;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut dependent = Task::new(project.id.clone(), "Deploy Step".to_string());
    dependent.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(dependent.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&dependent.id, &blocker.id)
        .await
        .unwrap();

    let has_blockers = manager.has_unresolved_blockers(dependent.id.as_str()).await;
    assert!(
        has_blockers,
        "Failed blockers must be treated as unresolved (hard-block)"
    );
}

// ============================================================================
// RC5: Event-driven transition logging
// Verify that the three primary event-driven transitions succeed and return the
// correct status. The INFO log added to transition_task_with_metadata fires on
// the success path, so a passing test confirms the log line is reachable.
// ============================================================================

/// RC5 guard: Executing → PendingReview (ExecutionComplete event path).
#[tokio::test]
async fn test_executing_to_pending_review_transition_succeeds() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "RC5 Executing Task".to_string());
    task.internal_status = InternalStatus::Executing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated = service
        .transition_task_with_metadata(&task.id, InternalStatus::PendingReview, None)
        .await
        .unwrap();

    assert_eq!(
        updated.internal_status,
        InternalStatus::PendingReview,
        "RC5: Executing → PendingReview must succeed and persist"
    );
}

/// RC5 guard: Reviewing → ReviewPassed (ReviewComplete event path).
#[tokio::test]
async fn test_reviewing_to_review_passed_transition_succeeds() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "RC5 Reviewing Task".to_string());
    task.internal_status = InternalStatus::Reviewing;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated = service
        .transition_task_with_metadata(&task.id, InternalStatus::ReviewPassed, None)
        .await
        .unwrap();

    assert_eq!(
        updated.internal_status,
        InternalStatus::ReviewPassed,
        "RC5: Reviewing → ReviewPassed must succeed and persist"
    );
}

/// RC5 guard: ReviewPassed → Approved (HumanApprove event path).
#[tokio::test]
async fn test_review_passed_to_approved_transition_succeeds() {
    let app_state = AppState::new_test();
    let service = build_test_service(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    app_state.project_repo.create(project.clone()).await.unwrap();

    let mut task = Task::new(project.id.clone(), "RC5 ReviewPassed Task".to_string());
    task.internal_status = InternalStatus::ReviewPassed;
    app_state.task_repo.create(task.clone()).await.unwrap();

    let updated = service
        .transition_task_with_metadata(&task.id, InternalStatus::Approved, None)
        .await
        .unwrap();

    assert_eq!(
        updated.internal_status,
        InternalStatus::Approved,
        "RC5: ReviewPassed → Approved must succeed and persist"
    );
}

// ============================================================================
// Wave 3: Git Isolation ExecutionRecoveryMetadata Tests
// ============================================================================

#[test]
fn test_git_isolation_metadata_created_for_git_isolation_reason() {
    let reason = format!("{}: could not create worktree at '/tmp/test'", GIT_ISOLATION_ERROR_PREFIX);
    let result = create_git_isolation_recovery_metadata_json(&reason, None);
    assert!(result.is_some(), "Expected metadata JSON for git isolation reason");
}

#[test]
fn test_git_isolation_metadata_not_created_for_non_git_reason() {
    let result = create_git_isolation_recovery_metadata_json("Agent error: something failed", None);
    assert!(result.is_none(), "Expected no metadata for non-git ExecutionBlocked reason");
}

#[test]
fn test_git_isolation_metadata_not_created_for_empty_reason() {
    let result = create_git_isolation_recovery_metadata_json("", None);
    assert!(result.is_none(), "Expected no metadata for empty reason");
}

#[test]
fn test_git_isolation_metadata_last_state_is_retrying() {
    let reason = format!("{}: could not create worktree", GIT_ISOLATION_ERROR_PREFIX);
    let json = create_git_isolation_recovery_metadata_json(&reason, None).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let recovery: ExecutionRecoveryMetadata =
        serde_json::from_value(parsed["execution_recovery"].clone()).unwrap();

    assert_eq!(
        recovery.last_state,
        ExecutionRecoveryState::Retrying,
        "last_state must be Retrying for reconciler eligibility"
    );
    assert!(!recovery.stop_retrying, "stop_retrying must be false on initial failure");
}

#[test]
fn test_git_isolation_metadata_event_has_correct_fields() {
    let reason = format!("{}: stale index.lock detected", GIT_ISOLATION_ERROR_PREFIX);
    let json = create_git_isolation_recovery_metadata_json(&reason, None).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let recovery: ExecutionRecoveryMetadata =
        serde_json::from_value(parsed["execution_recovery"].clone()).unwrap();

    assert_eq!(recovery.events.len(), 1);
    let event = &recovery.events[0];
    assert_eq!(event.kind, ExecutionRecoveryEventKind::Failed);
    assert_eq!(event.source, ExecutionRecoverySource::Auto);
    assert_eq!(event.reason_code, ExecutionRecoveryReasonCode::GitIsolationFailed);
    assert_eq!(
        event.failure_source,
        Some(ExecutionFailureSource::GitIsolation)
    );
    assert_eq!(event.message, reason);
}

#[test]
fn test_git_isolation_metadata_deserialization_round_trip() {
    let reason = format!("{}: leftover worktree directory exists", GIT_ISOLATION_ERROR_PREFIX);
    let json = create_git_isolation_recovery_metadata_json(&reason, None).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let recovery: ExecutionRecoveryMetadata =
        serde_json::from_value(parsed["execution_recovery"].clone()).unwrap();

    // Round-trip: re-serialize and re-deserialize must produce identical struct
    let re_json = serde_json::to_string(&recovery).unwrap();
    let re_recovery: ExecutionRecoveryMetadata = serde_json::from_str(&re_json).unwrap();
    assert_eq!(recovery, re_recovery, "Round-trip deserialization must be lossless");
}

#[test]
fn test_git_isolation_metadata_preserves_existing_metadata_keys() {
    let existing = r#"{"branch_freshness_conflict": false, "trigger_origin": "manual"}"#;
    let reason = format!("{}: stale lock file", GIT_ISOLATION_ERROR_PREFIX);
    let json =
        create_git_isolation_recovery_metadata_json(&reason, Some(existing)).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    // Existing metadata keys must be preserved
    assert_eq!(
        parsed["branch_freshness_conflict"],
        serde_json::Value::Bool(false),
        "Existing key branch_freshness_conflict must be preserved"
    );
    assert_eq!(
        parsed["trigger_origin"],
        serde_json::Value::String("manual".to_string()),
        "Existing key trigger_origin must be preserved"
    );
    // execution_recovery key must be present
    assert!(
        parsed["execution_recovery"].is_object(),
        "execution_recovery key must be added to existing metadata"
    );
}
