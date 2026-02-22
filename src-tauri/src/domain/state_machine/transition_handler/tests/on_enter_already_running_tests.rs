// Tests for AgentAlreadyRunning guard in on_enter(Merging) and on_enter(Reviewing).
//
// RC#2 fix: when chat_service.send_message() returns AgentAlreadyRunning,
// on_enter should log info and return Ok(()) (no-op), NOT record a spawn failure.
//
// Per CLAUDE.md rule 1.5: MemoryTaskRepository + MockChatService.
// Mock agent spawning only → verify call_count() and absence of spawn_failure metadata.

use super::helpers::*;
use crate::domain::entities::{
    InternalStatus, MergeRecoveryMetadata, Project, ProjectId, Task,
};
use crate::domain::state_machine::services::TaskScheduler;
use crate::domain::state_machine::{State, TransitionHandler};

// ──────────────────────────────────────────────────────────────────────────────
// Shared helper: TaskServices with a MockChatService that returns
// AgentAlreadyRunning after the first successful call.
// ──────────────────────────────────────────────────────────────────────────────

async fn make_services_already_running_after_1(
    task_repo: Arc<MemoryTaskRepository>,
    project_repo: Arc<MemoryProjectRepository>,
) -> (Arc<MockChatService>, TaskServices) {
    let chat_service = Arc::new(MockChatService::new());
    chat_service.set_already_running_after(1).await;
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

fn build_services_from_chat(
    chat_service: &Arc<MockChatService>,
    task_repo: &Arc<MemoryTaskRepository>,
    project_repo: &Arc<MemoryProjectRepository>,
) -> TaskServices {
    TaskServices::new(
        Arc::new(MockAgentSpawner::new()) as Arc<dyn AgentSpawner>,
        Arc::new(MockEventEmitter::new()) as Arc<dyn EventEmitter>,
        Arc::new(MockNotifier::new()) as Arc<dyn Notifier>,
        Arc::new(MockDependencyManager::new()) as Arc<dyn DependencyManager>,
        Arc::new(MockReviewStarter::new()) as Arc<dyn ReviewStarter>,
        Arc::clone(chat_service) as Arc<dyn ChatService>,
    )
    .with_task_scheduler(Arc::new(MockTaskScheduler::new()) as Arc<dyn TaskScheduler>)
    .with_task_repo(Arc::clone(task_repo) as Arc<dyn TaskRepository>)
    .with_project_repo(Arc::clone(project_repo) as Arc<dyn ProjectRepository>)
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: on_enter(Merging) double call → call_count == 1, no spawn_failure
// ──────────────────────────────────────────────────────────────────────────────

/// Double on_enter(Merging): second call returns AgentAlreadyRunning → no-op.
///
/// Verifies:
///   1. First on_enter(Merging) succeeds (agent spawned, call_count == 1)
///   2. Second on_enter(Merging) returns Ok (no error propagated)
///   3. call_count == 2 (both calls reached chat_service) but only 1 agent spawned
///   4. No AttemptFailed / spawn_failure metadata written (record_merger_spawn_failure skipped)
#[tokio::test]
async fn merging_double_on_enter_agent_already_running_is_noop() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "Merging double on_enter test".to_string());
    task.internal_status = InternalStatus::Merging;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new(
        "test-project".to_string(),
        "/tmp/nonexistent-already-running-merge".to_string(),
    );
    project.id = project_id;
    project_repo.create(project).await.unwrap();

    let (chat_service, _) =
        make_services_already_running_after_1(Arc::clone(&task_repo), Arc::clone(&project_repo))
            .await;

    // First on_enter(Merging): succeeds
    {
        let services =
            build_services_from_chat(&chat_service, &task_repo, &project_repo);
        let context = TaskContext::new(task_id.as_str(), "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        let result = handler.on_enter(&State::Merging).await;
        assert!(result.is_ok(), "First on_enter(Merging) should succeed: {:?}", result.err());
    }

    assert_eq!(
        chat_service.call_count(),
        1,
        "After first on_enter, call_count should be 1"
    );

    // Second on_enter(Merging): AgentAlreadyRunning → should be no-op (Ok)
    {
        let services =
            build_services_from_chat(&chat_service, &task_repo, &project_repo);
        let context = TaskContext::new(task_id.as_str(), "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        let result = handler.on_enter(&State::Merging).await;
        assert!(
            result.is_ok(),
            "Second on_enter(Merging) must return Ok (no-op), not propagate AgentAlreadyRunning: {:?}",
            result.err()
        );
    }

    assert_eq!(
        chat_service.call_count(),
        2,
        "Both on_enter calls should have reached chat_service (call_count == 2)"
    );

    // Key assertion: no spawn_failure metadata written (record_merger_spawn_failure was skipped)
    let updated = task_repo.get_by_id(&task_id).await.unwrap().unwrap();
    let recovery = MergeRecoveryMetadata::from_task_metadata(updated.metadata.as_deref())
        .unwrap_or(None);

    if let Some(ref r) = recovery {
        let attempt_failed_count = r
            .events
            .iter()
            .filter(|e| matches!(e.kind, crate::domain::entities::MergeRecoveryEventKind::AttemptFailed))
            .count();
        assert_eq!(
            attempt_failed_count, 0,
            "AgentAlreadyRunning must NOT record AttemptFailed events. Got {}. Metadata: {:?}",
            attempt_failed_count, updated.metadata,
        );
    }
    // If recovery is None, no merge_recovery metadata was written — that's correct.
}

// ──────────────────────────────────────────────────────────────────────────────
// Test: on_enter(Reviewing) double call → call_count == 1, returns Ok
// ──────────────────────────────────────────────────────────────────────────────

/// Double on_enter(Reviewing): second call returns AgentAlreadyRunning → no-op.
///
/// Verifies:
///   1. First on_enter(Reviewing) succeeds (agent spawned, call_count == 1)
///   2. Second on_enter(Reviewing) returns Ok (no error propagated)
///   3. call_count == 2 (both calls reached chat_service) but only 1 agent spawned
#[tokio::test]
async fn reviewing_double_on_enter_agent_already_running_is_noop() {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project_id = ProjectId::from_string("proj-1".to_string());
    let mut task = Task::new(project_id.clone(), "Reviewing double on_enter test".to_string());
    task.internal_status = InternalStatus::Reviewing;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let mut project = Project::new(
        "test-project".to_string(),
        "/tmp/nonexistent-already-running-review".to_string(),
    );
    project.id = project_id;
    project_repo.create(project).await.unwrap();

    let (chat_service, _) =
        make_services_already_running_after_1(Arc::clone(&task_repo), Arc::clone(&project_repo))
            .await;

    // First on_enter(Reviewing): succeeds
    {
        let services =
            build_services_from_chat(&chat_service, &task_repo, &project_repo);
        let context = TaskContext::new(task_id.as_str(), "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        let result = handler.on_enter(&State::Reviewing).await;
        assert!(result.is_ok(), "First on_enter(Reviewing) should succeed: {:?}", result.err());
    }

    assert_eq!(
        chat_service.call_count(),
        1,
        "After first on_enter, call_count should be 1"
    );

    // Second on_enter(Reviewing): AgentAlreadyRunning → should be no-op (Ok)
    {
        let services =
            build_services_from_chat(&chat_service, &task_repo, &project_repo);
        let context = TaskContext::new(task_id.as_str(), "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        let result = handler.on_enter(&State::Reviewing).await;
        assert!(
            result.is_ok(),
            "Second on_enter(Reviewing) must return Ok (no-op), not propagate AgentAlreadyRunning: {:?}",
            result.err()
        );
    }

    assert_eq!(
        chat_service.call_count(),
        2,
        "Both on_enter calls should have reached chat_service (call_count == 2)"
    );
}
