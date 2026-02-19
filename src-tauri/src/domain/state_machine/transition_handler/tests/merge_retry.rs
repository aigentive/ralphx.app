// Merge retry tests: reload continuation, event emission, deferred merge regression, main merge retry

use super::helpers::*;
use crate::domain::state_machine::{
    State, TaskEvent, TaskStateMachine, TransitionHandler, TransitionResult,
};
use std::sync::Arc;

// ==================
// Reload continuation tests
// ==================

/// Callback drop during merge workflow handled gracefully (no panic without app_handle).
#[tokio::test]
async fn test_reload_continuation_callback_drop() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let services = TaskServices::new_mock().with_execution_state(Arc::clone(&execution_state));
    assert!(services.app_handle.is_none(), "Mock services should not have app_handle");

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    // None should panic (graceful handling even without app_handle)
    handler.on_exit(&State::PendingMerge, &State::Merged).await;
    handler.on_exit(&State::PendingMerge, &State::MergeIncomplete).await;

    assert_eq!(
        execution_state.running_count(), 1,
        "Running count should be unchanged (PendingMerge is not agent-active)"
    );
}

/// on_enter for merge states without app_handle (reload scenario).
#[tokio::test]
async fn test_reload_continuation_enter_states_without_app_handle() {
    let services = TaskServices::new_mock();
    assert!(services.app_handle.is_none());

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);

    let result = handler.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "on_enter(PendingMerge) should succeed without app_handle");

    let result = handler.on_enter(&State::Merged).await;
    assert!(result.is_ok(), "on_enter(Merged) should succeed without app_handle");

    let result = handler.on_enter(&State::Merging).await;
    assert!(result.is_ok(), "on_enter(Merging) should succeed without app_handle");
}

/// State recovery after simulated reload mid-merge.
#[tokio::test]
async fn test_reload_continuation_state_recovery() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    // Phase 1: Task enters PendingMerge
    let scheduler1 = Arc::new(MockTaskScheduler::new());
    let services1 = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler1)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);
    let context1 = create_context_with_services("task-1", "proj-1", services1);
    let mut machine1 = TaskStateMachine::new(context1);
    let handler1 = TransitionHandler::new(&mut machine1);
    let _ = handler1.on_enter(&State::PendingMerge).await;

    // Phase 2: Simulate "reload" — create fresh context for same task
    let scheduler2 = Arc::new(MockTaskScheduler::new());
    let services2 = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler2)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);
    let context2 = create_context_with_services("task-1", "proj-1", services2);
    let mut machine2 = TaskStateMachine::new(context2);
    let handler2 = TransitionHandler::new(&mut machine2);

    let result = handler2.on_enter(&State::PendingMerge).await;
    assert!(result.is_ok(), "Re-entering PendingMerge after reload should succeed");

    handler2.on_exit(&State::PendingMerge, &State::Merged).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;
    let calls = scheduler2.get_calls();
    assert!(
        calls.iter().any(|c| c.method == "try_retry_deferred_merges"),
        "Deferred merge retry should work after reload"
    );
}

// ==================
// Event emission tests
// ==================

/// Approved entry emits task_completed event.
#[tokio::test]
async fn test_event_emission_approved_emits_task_completed() {
    let (_spawner, emitter, _notifier, _dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;

    assert_eq!(result.state(), Some(&State::PendingMerge));
    assert!(
        emitter.has_event("task_completed"),
        "Should emit task_completed when entering Approved"
    );
}

/// Merged entry triggers unblock_dependents side effect.
#[tokio::test]
async fn test_event_emission_merged_entry_side_effects() {
    let dep_manager = Arc::new(MockDependencyManager::new());
    let emitter = Arc::new(MockEventEmitter::new());

    let mut services = TaskServices::new_mock();
    services.dependency_manager = Arc::clone(&dep_manager) as Arc<dyn DependencyManager>;
    services.event_emitter = Arc::clone(&emitter) as Arc<dyn EventEmitter>;

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    let _ = handler.on_enter(&State::Merged).await;

    let dep_calls = dep_manager.get_calls();
    assert!(
        dep_calls.iter().any(|c| c.method == "unblock_dependents"),
        "on_enter(Merged) should unblock dependents"
    );
}

/// Exiting PendingMerge does NOT decrement execution running count.
#[tokio::test]
async fn test_event_emission_pending_merge_exit_preserves_execution_state() {
    use crate::commands::ExecutionState;

    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let emitter = Arc::new(MockEventEmitter::new());
    let mut services = TaskServices::new_mock();
    services.event_emitter = Arc::clone(&emitter) as Arc<dyn EventEmitter>;
    let services = services.with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    assert_eq!(
        execution_state.running_count(), 1,
        "PendingMerge exit should NOT decrement running count (not agent-active)"
    );
}

/// Full merge event sequence from ReviewPassed through PendingMerge.
#[tokio::test]
async fn test_event_emission_full_merge_event_sequence() {
    let (_spawner, emitter, _notifier, dep_manager, _review_starter, services) =
        create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let mut handler = TransitionHandler::new(&mut machine);

    let result = handler
        .handle_transition(&State::ReviewPassed, &TaskEvent::HumanApprove)
        .await;
    assert_eq!(result.state(), Some(&State::PendingMerge));

    let events = emitter.get_events();
    assert!(
        events.iter().any(|e| e.args.first().map(|s| s.as_str()) == Some("task_completed")),
        "Event sequence should include task_completed"
    );

    let dep_calls = dep_manager.get_calls();
    assert!(
        !dep_calls.iter().any(|c| c.method == "unblock_dependents"),
        "Dependents should NOT be unblocked at Approved"
    );
}

// ==================
// Deferred merge compatibility regression tests
// ==================

/// Regression: deferred merge retry preserved on all PendingMerge exits.
#[tokio::test]
async fn test_deferred_merge_retry_on_all_pending_merge_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let target_states = [State::Merged, State::MergeIncomplete, State::Merging];

    for target in &target_states {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock().with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        handler.on_exit(&State::PendingMerge, target).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(), 1,
            "Expected deferred merge retry when exiting PendingMerge to {:?}", target
        );
        assert_eq!(
            retry_calls[0].args, vec!["proj-1"],
            "Deferred retry should use correct project_id for target {:?}", target
        );
    }
}

/// Regression: deferred merge retry preserved on all Merging exits.
#[tokio::test]
async fn test_deferred_merge_retry_on_all_merging_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let target_states = [State::Merged, State::MergeIncomplete, State::MergeConflict];

    for target in &target_states {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock().with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        handler.on_exit(&State::Merging, target).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(), 1,
            "Expected deferred merge retry when exiting Merging to {:?}", target
        );
    }
}

/// Regression: single on_exit produces exactly one deferred retry (no duplicates).
#[tokio::test]
async fn test_deferred_merge_no_duplicate_retries() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let services = TaskServices::new_mock()
        .with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let calls = scheduler.get_calls();
    let retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_deferred_merges")
        .collect();

    assert_eq!(
        retry_calls.len(), 1,
        "Single on_exit should produce exactly one deferred retry call, got {}", retry_calls.len()
    );
}

/// Regression: non-merge state exits do NOT trigger deferred merge retry.
#[tokio::test]
async fn test_deferred_merge_not_triggered_by_non_merge_exits() {
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let non_merge_transitions = [
        (State::Executing, State::PendingReview),
        (State::Reviewing, State::ReviewPassed),
        (State::ReExecuting, State::PendingReview),
        (State::Ready, State::Executing),
        (State::QaTesting, State::QaPassed),
        (State::QaRefining, State::QaTesting),
    ];

    for (from, to) in &non_merge_transitions {
        let scheduler = Arc::new(MockTaskScheduler::new());
        let services = TaskServices::new_mock().with_task_scheduler(Arc::clone(&scheduler)
            as Arc<dyn crate::domain::state_machine::services::TaskScheduler>);

        let context = create_context_with_services("task-1", "proj-1", services);
        let mut machine = TaskStateMachine::new(context);
        let handler = TransitionHandler::new(&mut machine);
        handler.on_exit(from, to).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

        let calls = scheduler.get_calls();
        let retry_calls: Vec<_> = calls
            .iter()
            .filter(|c| c.method == "try_retry_deferred_merges")
            .collect();

        assert_eq!(
            retry_calls.len(), 0,
            "Non-merge exit {:?} -> {:?} should NOT trigger deferred retry", from, to
        );
    }
}

// ==================
// Merge-exit main merge retry tests
// ==================

/// Exiting PendingMerge calls try_retry_main_merges when running_count == 0.
#[tokio::test]
async fn test_merge_exit_triggers_main_merge_retry_when_all_idle() {
    use crate::commands::ExecutionState;
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let execution_state = Arc::new(ExecutionState::new());

    let services = TaskServices::new_mock()
        .with_task_scheduler(
            Arc::clone(&scheduler) as Arc<dyn crate::domain::state_machine::services::TaskScheduler>,
        )
        .with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    let calls = scheduler.get_calls();
    let main_retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_main_merges")
        .collect();

    assert_eq!(
        main_retry_calls.len(), 1,
        "Expected try_retry_main_merges when all agents idle (running_count == 0)"
    );
}

/// Exiting PendingMerge skips try_retry_main_merges when agents are still running.
#[tokio::test]
async fn test_merge_exit_skips_main_merge_retry_when_agents_running() {
    use crate::commands::ExecutionState;
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let execution_state = Arc::new(ExecutionState::new());
    execution_state.increment_running();

    let services = TaskServices::new_mock()
        .with_task_scheduler(
            Arc::clone(&scheduler) as Arc<dyn crate::domain::state_machine::services::TaskScheduler>,
        )
        .with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::PendingMerge, &State::Merged).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    let calls = scheduler.get_calls();
    let main_retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_main_merges")
        .collect();

    assert_eq!(
        main_retry_calls.len(), 0,
        "Expected NO try_retry_main_merges when agents are still running (running_count > 0)"
    );
}

/// Cascading merge unblocking — Merging exit also triggers main merge retry when idle.
#[tokio::test]
async fn test_merging_exit_triggers_main_merge_retry_when_all_idle() {
    use crate::commands::ExecutionState;
    use crate::domain::state_machine::mocks::MockTaskScheduler;

    let scheduler = Arc::new(MockTaskScheduler::new());
    let execution_state = Arc::new(ExecutionState::new());

    let services = TaskServices::new_mock()
        .with_task_scheduler(
            Arc::clone(&scheduler) as Arc<dyn crate::domain::state_machine::services::TaskScheduler>,
        )
        .with_execution_state(Arc::clone(&execution_state));

    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);
    let handler = TransitionHandler::new(&mut machine);
    handler.on_exit(&State::Merging, &State::Merged).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

    let calls = scheduler.get_calls();
    let main_retry_calls: Vec<_> = calls
        .iter()
        .filter(|c| c.method == "try_retry_main_merges")
        .collect();

    // Merging is agent-active, so there may be 1 or 2 calls. Assert at least 1.
    assert!(
        !main_retry_calls.is_empty(),
        "Merging exit should trigger try_retry_main_merges when all agents idle"
    );
}
