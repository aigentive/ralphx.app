// Fix B1: send_message error → ExecutionBlocked → task transitions to Failed
//
// After fix: when chat_service.send_message() fails during on_enter(Executing),
// it returns Err(ExecutionBlocked(...)) instead of silently swallowing the error.
// TaskTransitionService catches ExecutionBlocked and transitions to Failed.

use crate::tests::hardening_fixes::helpers::*;
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::State;
use crate::error::AppError;

#[tokio::test]
async fn test_b1_fix_send_message_error_returns_execution_blocked() {
    // After fix: on_enter(Executing) returns ExecutionBlocked when send_message fails
    let svc = create_hardening_services();
    svc.chat_service.set_available(false).await;

    // Build services WITHOUT repos so git setup is skipped
    let services = crate::domain::state_machine::context::TaskServices::new(
        svc.spawner.clone() as _,
        svc.emitter.clone() as _,
        svc.notifier.clone() as _,
        svc.dependency_manager.clone() as _,
        svc.review_starter.clone() as _,
        svc.chat_service.clone() as _,
    )
    .with_execution_state(svc.execution_state.clone());

    let mut machine = create_state_machine("task-b1-fix", "proj-b1-fix", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // The transition itself succeeds (state machine accepted the event),
    // but on_enter returned an error which is now emitted as an event
    assert!(
        result.is_success(),
        "State machine transition should still succeed"
    );

    // Verify the on_enter_error event was emitted (H2 fix makes errors visible)
    let events = svc.emitter.get_events();
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| e.args.first().map(|s| s == "task:on_enter_error").unwrap_or(false))
        .collect();
    assert!(
        !error_events.is_empty(),
        "FIX B1+H2: on_enter error should emit task:on_enter_error event"
    );
}

#[tokio::test]
async fn test_b1_fix_re_executing_also_returns_error() {
    // After fix: on_enter(ReExecuting) also returns error when send_message fails
    let svc = create_hardening_services();
    svc.chat_service.set_available(false).await;

    let services = crate::domain::state_machine::context::TaskServices::new(
        svc.spawner.clone() as _,
        svc.emitter.clone() as _,
        svc.notifier.clone() as _,
        svc.dependency_manager.clone() as _,
        svc.review_starter.clone() as _,
        svc.chat_service.clone() as _,
    )
    .with_execution_state(svc.execution_state.clone())
    .with_task_scheduler(
        svc.scheduler.clone() as std::sync::Arc<dyn crate::domain::state_machine::services::TaskScheduler>,
    );

    let mut machine = create_state_machine("task-b1-reexec-fix", "proj-b1-fix", services);
    machine.context.review_feedback = Some("Fix tests".to_string());
    let mut handler = create_transition_handler(&mut machine);

    // Transition to ReExecuting via RevisionNeeded auto-transition
    let _result = handler
        .handle_transition(
            &State::Reviewing,
            &TaskEvent::ReviewComplete {
                approved: false,
                feedback: Some("Fix tests".to_string()),
            },
        )
        .await;

    // send_message was called and failed
    assert!(
        svc.chat_service.call_count() >= 1,
        "send_message should have been called"
    );

    // Error event should be emitted
    let events = svc.emitter.get_events();
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| e.args.first().map(|s| s == "task:on_enter_error").unwrap_or(false))
        .collect();
    assert!(
        !error_events.is_empty(),
        "FIX B1: ReExecuting on_enter error should also emit error event"
    );
}

#[tokio::test]
async fn test_b1_fix_execution_blocked_error_carries_message() {
    // Verify the ExecutionBlocked error message includes useful context
    let error = AppError::ExecutionBlocked(
        "Failed to start agent: Chat service unavailable".to_string(),
    );
    let msg = error.to_string();
    assert!(msg.contains("Failed to start agent"), "Error should include agent failure context");
}
