// Tests for spawn failure recovery: on_enter(Merging) with failing chat service.
//
// Covers Fix 2 (commit 189e8eaf): spawn failures now record AttemptFailed events.
// After merging_max_retries failures the retry budget is exhausted so the reconciler
// can transition the task to MergeIncomplete on the next cycle (≤30 s).
//
// Per CLAUDE.md rule 1.5: MemoryTaskRepository + MockChatService (set unavailable).
// No real git repo needed — these tests exercise the spawn failure path, not git.

use super::helpers::*;
use crate::domain::entities::{
    InternalStatus, MergeRecoveryEventKind, MergeRecoveryMetadata, Project, ProjectId, Task,
};
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::{State, TransitionHandler};
use crate::infrastructure::agents::claude::reconciliation_config;

// ──────────────────────────────────────────────────────────────────────────────
// Shared helper
// ──────────────────────────────────────────────────────────────────────────────

/// Build TaskServices wired with a FAILING MockChatService (is_available = false).
///
/// chat_service.send_message() returns AgentNotAvailable, triggering
/// record_merger_spawn_failure() and recording an AttemptFailed event.
async fn make_services_with_failing_chat(
    task_repo: Arc<MemoryTaskRepository>,
    project_repo: Arc<MemoryProjectRepository>,
) -> (Arc<MockChatService>, TaskServices) {
    let chat_service = Arc::new(MockChatService::new());
    chat_service.set_available(false).await;
    let services = TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(&chat_service) as Arc<dyn ChatService>,
    )
    .with_task_scheduler(Arc::new(MockTaskScheduler::new()) as Arc<dyn TaskScheduler>)
    .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>)
    .with_project_repo(Arc::clone(&project_repo) as Arc<dyn ProjectRepository>);
    (chat_service, services)
}

/// Seed repos with a Merging task and a project pointing at a nonexistent directory.
async fn setup_merging_task() -> (
    Arc<MemoryTaskRepository>,
    Arc<MemoryProjectRepository>,
    crate::domain::entities::TaskId,
) {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "Spawn failure test".to_string());
    task.internal_status = InternalStatus::Merging;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new(
        "test-project".to_string(),
        "/tmp/nonexistent-spawn-test".to_string(),
    );
    project.id = project_id;
    project_repo.create(project).await.unwrap();

    (task_repo, project_repo, task_id)
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 4: single spawn failure → AttemptFailed event recorded
// ──────────────────────────────────────────────────────────────────────────────

/// Test 4: A single merger agent spawn failure records an AttemptFailed event.
///
/// Orchestration chain:
///   on_enter(&State::Merging)
///   → on_enter_dispatch(Merging)
///   → chat_service.send_message(ChatContextType::Merge, ...) → AgentNotAvailable error
///   → record_merger_spawn_failure(task_repo, task_id, error_msg)
///   → MergeRecoveryMetadata.append_event(AttemptFailed, message="Merger agent failed to spawn: ...")
///   → task.metadata updated in repo
///
/// Verified: task metadata contains exactly 1 AttemptFailed event with "failed to spawn" message.
#[tokio::test]
async fn spawn_failure_records_attempt_failed_event() {
    let (task_repo, project_repo, task_id) = setup_merging_task().await;

    let (_chat_service, services) =
        make_services_with_failing_chat(Arc::clone(&task_repo), Arc::clone(&project_repo)).await;
    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let _ = handler.on_enter(&State::Merging).await;

    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    let recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
        .expect("metadata parse must not fail")
        .expect("merge_recovery key must be present after a spawn failure");

    let attempt_failed_count = recovery
        .events
        .iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AttemptFailed))
        .count();

    assert_eq!(
        attempt_failed_count, 1,
        "Exactly 1 AttemptFailed event must be recorded on spawn failure. \
         Got {} events. Metadata: {:?}",
        attempt_failed_count, updated.metadata,
    );

    let spawn_failure_event = recovery.events.iter().find(|e| {
        matches!(e.kind, MergeRecoveryEventKind::AttemptFailed)
            && e.message.contains("failed to spawn")
    });
    assert!(
        spawn_failure_event.is_some(),
        "AttemptFailed event message must contain 'failed to spawn'. \
         Events: {:?}",
        recovery.events,
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Test 5: N spawn failures → retry budget exhausted
// ──────────────────────────────────────────────────────────────────────────────

/// Test 5: After merging_max_retries spawn failures the AttemptFailed count
/// reaches the reconciler's retry limit.
///
/// This confirms that N consecutive spawn failures accumulate N AttemptFailed
/// events in task metadata. Once the count reaches merging_max_retries the
/// reconciler will transition the task to MergeIncomplete on the next cycle.
///
/// Verified: after N on_enter(Merging) calls with a failing chat service:
///   - AttemptFailed count == merging_max_retries
///   - Count >= merging_max_retries (retry budget is exhausted)
#[tokio::test]
async fn n_spawn_failures_exhaust_retry_budget() {
    let (task_repo, project_repo, task_id) = setup_merging_task().await;

    let max_retries = reconciliation_config().merging_max_retries as u32;

    // Simulate max_retries consecutive spawn failures.
    // Each on_enter(Merging) with a failing chat service appends one AttemptFailed event.
    for _ in 0..max_retries {
        let (_cs, services) =
            make_services_with_failing_chat(Arc::clone(&task_repo), Arc::clone(&project_repo))
                .await;
        let context = TaskContext::new(task_id.as_str(), "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        let _ = handler.on_enter(&State::Merging).await;
    }

    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    let recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
        .expect("metadata parse must not fail")
        .expect("merge_recovery key must be present after spawn failures");

    let attempt_failed_count = recovery
        .events
        .iter()
        .filter(|e| matches!(e.kind, MergeRecoveryEventKind::AttemptFailed))
        .count() as u32;

    assert_eq!(
        attempt_failed_count, max_retries,
        "After {} spawn failures AttemptFailed count must be {}. Got {}. Metadata: {:?}",
        max_retries, max_retries, attempt_failed_count, updated.metadata,
    );
    assert!(
        attempt_failed_count >= max_retries,
        "Retry budget must be exhausted: count {} >= max_retries {}",
        attempt_failed_count, max_retries,
    );
}
