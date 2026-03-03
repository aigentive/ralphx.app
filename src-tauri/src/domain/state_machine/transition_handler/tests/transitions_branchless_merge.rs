// Tests for branchless task auto-transition: Approved → Merged (skipping PendingMerge).
//
// When a task has no task_branch (e.g., external repo work), the merge pipeline
// is skipped because there's no branch to merge into the main repo.

use super::helpers::*;
use crate::domain::entities::{ProjectId, Task};
use crate::domain::state_machine::{
    State, TaskEvent, TaskStateMachine, TransitionHandler, TransitionResult,
};

#[tokio::test]
async fn test_branchless_task_approved_skips_merge_pipeline() {
    // Create a task with NO branch (simulates external repo work)
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let mut task = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        "Create shared agentive/reefbot-strategy repo".to_string(),
    );
    task.task_branch = None;
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>);
    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Should auto-transition directly to Merged (NOT PendingMerge)
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(
            *state,
            State::Merged,
            "Branchless task should skip merge pipeline and go to Merged"
        );
    } else {
        panic!(
            "Expected AutoTransition to Merged, got {:?}",
            result
        );
    }
}

#[tokio::test]
async fn test_branched_task_approved_enters_merge_pipeline() {
    // Create a task WITH a branch (normal workflow)
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let mut task = Task::new(
        ProjectId::from_string("proj-1".to_string()),
        "Normal implementation task".to_string(),
    );
    task.task_branch = Some("ralphx/test-project/task-123".to_string());
    let task_id = task.id.clone();
    task_repo.create(task).await.unwrap();

    let services = TaskServices::new_mock()
        .with_task_repo(Arc::clone(&task_repo) as Arc<dyn TaskRepository>);
    let context = TaskContext::new(task_id.as_str(), "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Should auto-transition to PendingMerge (normal merge pipeline)
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(
            *state,
            State::PendingMerge,
            "Branched task should enter merge pipeline"
        );
    } else {
        panic!(
            "Expected AutoTransition to PendingMerge, got {:?}",
            result
        );
    }
}

#[tokio::test]
async fn test_no_task_repo_defaults_to_merge_pipeline() {
    // When task_repo is not available (e.g., old test setup), default to PendingMerge
    let (_spawner, _emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    // Should default to PendingMerge when task_repo is not set
    if let TransitionResult::AutoTransition(state) = &result {
        assert_eq!(
            *state,
            State::PendingMerge,
            "Without task_repo, should default to merge pipeline"
        );
    } else {
        panic!(
            "Expected AutoTransition to PendingMerge, got {:?}",
            result
        );
    }
}
