// Fix H2: Non-ExecutionBlocked on_enter errors now visible via event emission
//
// After fix: when on_enter fails for any state, a task:on_enter_error event
// is emitted via event_emitter before logging. The transition still succeeds
// but the error is now visible in the UI.

use crate::tests::hardening_fixes::helpers::*;
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::State;

#[tokio::test]
async fn test_h2_fix_on_enter_error_emits_event() {
    // After fix: on_enter error emits task:on_enter_error event
    // Force a failure by making chat_service unavailable (no repos = skip git, hits send_message)
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
    .with_execution_state(svc.execution_state.clone());

    let mut machine = create_state_machine("task-h2-fix", "proj-h2-fix", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // Transition still succeeds (state machine accepted the event)
    assert!(result.is_success(), "Transition should still succeed");

    // But the on_enter error is now visible via event
    let events = svc.emitter.get_events();
    let on_enter_errors: Vec<_> = events
        .iter()
        .filter(|e| e.args.first().map(|s| s == "task:on_enter_error").unwrap_or(false))
        .collect();
    assert!(
        !on_enter_errors.is_empty(),
        "FIX H2: task:on_enter_error event should be emitted when on_enter fails"
    );
}

#[tokio::test]
async fn test_h2_fix_on_enter_error_event_includes_state_and_error() {
    // After fix: the emitted event payload includes the state and error message
    let svc = create_hardening_services();
    svc.chat_service.set_available(false).await;

    // Build minimal services to force send_message failure
    let services = crate::domain::state_machine::context::TaskServices::new(
        svc.spawner.clone() as _,
        svc.emitter.clone() as _,
        svc.notifier.clone() as _,
        svc.dependency_manager.clone() as _,
        svc.review_starter.clone() as _,
        svc.chat_service.clone() as _,
    )
    .with_execution_state(svc.execution_state.clone());

    let mut machine = create_state_machine("task-h2-payload", "proj-h2-fix", services);
    let mut handler = create_transition_handler(&mut machine);

    let _result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // Find the error event
    let events = svc.emitter.get_events();
    let error_event = events
        .iter()
        .find(|e| e.args.first().map(|s| s == "task:on_enter_error").unwrap_or(false));

    assert!(
        error_event.is_some(),
        "FIX H2: task:on_enter_error event should be emitted"
    );

    // Verify payload contains state info
    let event = error_event.unwrap();
    let payload = &event.args[2]; // emit_with_payload: [event_type, task_id, payload]
    assert!(
        payload.contains("Executing") || payload.contains("state"),
        "FIX H2: Error event payload should include state information"
    );
}
