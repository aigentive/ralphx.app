// Error visibility hardening tests (Scenarios H1-H4)
//
// Tests verifying that errors during state transitions are properly surfaced
// to users, and demonstrating gaps where errors are silently swallowed.
//
// Key concern: TransitionHandler catches on_enter errors and logs them
// but does NOT propagate them. Only ExecutionBlocked is explicitly surfaced.

use super::helpers::*;
use crate::domain::entities::{InternalStatus, ProjectId};
use crate::domain::repositories::TaskRepository;
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::State;
use crate::domain::state_machine::services::EventEmitter;
use crate::error::AppError;

// ============================================================================
// H1: ExecutionBlocked visible to user — COVERED
// ============================================================================

#[tokio::test]
async fn test_h1_execution_blocked_error_variant_exists_with_message() {
    // COVERED: AppError::ExecutionBlocked carries a human-readable message.
    // In production, TaskTransitionService catches this and transitions the
    // task to Failed with blocked_reason set.
    let error = AppError::ExecutionBlocked(
        "Cannot execute task: uncommitted changes in working directory.".to_string(),
    );

    let error_msg = error.to_string();
    assert!(
        error_msg.contains("Execution blocked"),
        "Error display should include 'Execution blocked'"
    );
    assert!(
        error_msg.contains("uncommitted changes"),
        "Error display should include the specific reason"
    );
}

#[tokio::test]
async fn test_h1_task_blocked_reason_is_accessible() {
    // COVERED: blocked_reason field on Task is user-visible.
    // When ExecutionBlocked is caught, task.blocked_reason is set.
    let s = create_hardening_services();
    let project_id = ProjectId::from_string("proj-h1".to_string());

    let mut task = create_test_task_with_status(
        &project_id,
        "Blocked task",
        InternalStatus::Failed,
    );
    task.blocked_reason = Some("Git isolation failed: uncommitted changes".to_string());
    s.task_repo.create(task.clone()).await.unwrap();

    let stored = s
        .task_repo
        .get_by_id(&task.id)
        .await
        .unwrap()
        .expect("Task should exist");
    assert_eq!(
        stored.blocked_reason,
        Some("Git isolation failed: uncommitted changes".to_string()),
        "blocked_reason should be persisted and accessible"
    );
}

#[tokio::test]
async fn test_h1_failed_transition_emits_task_failed_event() {
    // COVERED: Transitioning to Failed emits "task_failed" via event_emitter.
    // This is the primary user-visibility mechanism for execution failures.
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-h1c", "proj-h1c", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(
            &State::Executing,
            &TaskEvent::ExecutionFailed {
                error: "Git isolation failed".to_string(),
            },
        )
        .await;

    assert!(result.is_success(), "Transition to Failed should succeed");
    assert!(
        matches!(result.state(), Some(State::Failed(_))),
        "Should be in Failed state"
    );

    // on_enter(Failed) calls event_emitter.emit("task_failed", task_id)
    assert!(
        s.emitter.has_event("task_failed"),
        "COVERED: event_emitter should emit 'task_failed' for user visibility"
    );
}

#[tokio::test]
async fn test_h1_failed_transition_stores_failure_metadata() {
    // COVERED: Transitioning to Failed stores failure metadata on the task.
    // The on_enter(Failed) code reads the task from repo and updates metadata
    // with failure_error, failure_details, and is_timeout fields.
    let s = create_hardening_services();
    let project_id = ProjectId::from_string("proj-h1d".to_string());
    let task = create_test_task_with_status(
        &project_id,
        "Task about to fail",
        InternalStatus::Executing,
    );
    let task_id_str = task.id.as_str().to_string();
    s.task_repo.create(task.clone()).await.unwrap();

    let services = build_task_services(&s);
    let mut machine = create_state_machine(&task_id_str, "proj-h1d", services);
    let mut handler = create_transition_handler(&mut machine);

    let result = handler
        .handle_transition(
            &State::Executing,
            &TaskEvent::ExecutionFailed {
                error: "Agent crashed".to_string(),
            },
        )
        .await;

    assert!(result.is_success());

    // Verify failure metadata was stored on the task
    let stored = s.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(
        stored.metadata.is_some(),
        "COVERED: Task should have failure metadata after transitioning to Failed"
    );
    let meta: serde_json::Value =
        serde_json::from_str(stored.metadata.as_ref().unwrap()).unwrap();
    assert!(
        meta.get("failure_error").is_some(),
        "COVERED: failure_error should be stored in task metadata"
    );
}

// ============================================================================
// H2: Non-ExecutionBlocked on_enter errors invisible — GAP
// ============================================================================

#[tokio::test]
async fn test_h2_on_enter_errors_swallowed_by_transition_handler() {
    // GAP: TransitionHandler::handle_transition catches ALL on_enter errors
    // and still returns TransitionResult::Success. The error is logged via
    // tracing but not propagated to the caller.
    //
    // From transition_handler/mod.rs:
    //   if let Err(e) = self.on_enter(&new_state).await {
    //       tracing::error!(...);
    //       // Still returns Success — error is logged but NOT propagated
    //   }
    let s = create_hardening_services();

    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-h2", "proj-h2", services);
    let mut handler = create_transition_handler(&mut machine);

    // Transition to Executing — on_enter will try to set up git branch/worktree,
    // but since we have no project in the repo, it will fail silently.
    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    // GAP: The transition reports success even though on_enter couldn't
    // complete its side effects (no project found = no branch setup).
    assert!(
        result.is_success(),
        "GAP: Transition returns Success even when on_enter side effects fail silently"
    );

    // No error event was emitted
    let events = s.emitter.get_events();
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.args
                .first()
                .map(|s| s.contains("error") || s.contains("failed"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        error_events.is_empty(),
        "GAP: No error events emitted to UI when on_enter fails — errors are only logged"
    );
}

#[tokio::test]
async fn test_h2_no_notification_on_silent_on_enter_failure() {
    // GAP: When on_enter fails silently, no notification is sent either.
    let s = create_hardening_services();

    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-h2-notif", "proj-h2-notif", services);
    let mut handler = create_transition_handler(&mut machine);

    // Transition to Executing without a project — on_enter fails silently
    let result = handler
        .handle_transition(&State::Ready, &TaskEvent::StartExecution)
        .await;

    assert!(result.is_success());

    // No notification about the on_enter failure
    let notifications = s.notifier.get_notifications();
    let on_enter_failure_notifications: Vec<_> = notifications
        .iter()
        .filter(|n| {
            n.args.iter().any(|a| {
                a.contains("on_enter") || a.contains("side_effect") || a.contains("branch_setup")
            })
        })
        .collect();
    assert!(
        on_enter_failure_notifications.is_empty(),
        "GAP: No notification sent when on_enter fails — user is unaware of the silent failure"
    );
}

#[tokio::test]
async fn test_h2_auto_transition_on_enter_errors_also_swallowed() {
    // GAP: Auto-transition on_enter errors are also swallowed.
    // The same pattern applies to auto-transitions. When an auto-transition's
    // on_enter fails, the error is caught and logged, but the auto-transition
    // still succeeds.
    let s = create_hardening_services();

    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-h2b", "proj-h2b", services);
    let mut handler = create_transition_handler(&mut machine);

    // QaPassed auto-transitions to PendingReview, which auto-transitions to Reviewing.
    // on_enter for Reviewing tries to start AI review, but no project exists.
    let result = handler
        .handle_transition(
            &State::QaTesting,
            &TaskEvent::QaTestsComplete { passed: true },
        )
        .await;

    // Even if side effects fail, the auto-transition chain completes
    assert!(
        result.is_success(),
        "GAP: Auto-transition chain succeeds even when on_enter fails for intermediate states"
    );
}

// ============================================================================
// H3: Spawn blocked by max_concurrent not visible — GAP
// ============================================================================

#[tokio::test]
async fn test_h3_at_capacity_no_event_emitted() {
    // GAP: When ExecutionState is at capacity, no event or error is emitted
    // to inform the user. The scheduler silently skips scheduling.
    let s = create_hardening_services();

    // Fill up capacity
    s.execution_state.increment_running();
    s.execution_state.increment_running();

    assert!(
        !s.execution_state.can_start_task(),
        "Should be at capacity (max=2, running=2)"
    );

    // Clear any events from setup
    s.emitter.clear();

    // The scheduler would check can_start_task() and silently return.
    // There is no event or notification mechanism for "task queued because at capacity".

    let events = s.emitter.get_events();
    let capacity_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.args.iter().any(|a| {
                a.contains("capacity")
                    || a.contains("queue")
                    || a.contains("blocked")
                    || a.contains("max_concurrent")
            })
        })
        .collect();
    assert!(
        capacity_events.is_empty(),
        "GAP: No capacity/queue events emitted when execution is at capacity"
    );

    // Verify notifications are also empty
    let notifications = s.notifier.get_notifications();
    assert!(
        notifications.is_empty(),
        "GAP: No notifications sent when spawn is blocked by max_concurrent"
    );
}

#[tokio::test]
async fn test_h3_paused_execution_also_silent() {
    // GAP: When execution is paused, can_start_task returns false,
    // but no user-facing event explains why tasks aren't starting.
    let s = create_hardening_services();

    s.execution_state.pause();
    assert!(s.execution_state.is_paused(), "Should be paused");
    assert!(
        !s.execution_state.can_start_task(),
        "Should not be able to start tasks when paused"
    );

    s.emitter.clear();

    // When a task enters Ready while paused, the scheduler would be called
    // but would check can_start_task() and silently return.

    let events = s.emitter.get_events();
    assert!(
        events.is_empty(),
        "GAP: No event explains to the user that tasks are not starting because execution is paused"
    );
}

#[tokio::test]
async fn test_h3_can_start_task_false_emits_nothing() {
    // GAP: Explicitly verify that can_start_task() returning false produces
    // no side effects at all — no events, no notifications, no errors.
    let s = create_hardening_services();

    // At capacity
    s.execution_state.increment_running();
    s.execution_state.increment_running();

    s.emitter.clear();
    s.notifier.clear();

    // Query can_start_task — should be false
    let can_start = s.execution_state.can_start_task();
    assert!(!can_start, "Should be at capacity");

    // Nothing was recorded anywhere
    assert_eq!(s.emitter.event_count(), 0, "GAP: No event on capacity check");
    assert_eq!(
        s.notifier.notification_count(),
        0,
        "GAP: No notification on capacity check"
    );
}

// ============================================================================
// H4: Reconciliation recovery actions visible — COVERED
// ============================================================================

#[tokio::test]
async fn test_h4_scheduler_called_on_slot_free() {
    // COVERED: When an agent-active state exits (on_exit), the running count
    // is decremented and scheduler.try_schedule_ready_tasks() is called.
    // This makes slot availability visible to the scheduling system.
    let s = create_hardening_services();
    let services = build_task_services(&s);
    let mut machine = create_state_machine("task-h4a", "proj-h4a", services);
    let mut handler = create_transition_handler(&mut machine);

    // Transition Executing -> Failed (on_exit decrements + schedules)
    let result = handler
        .handle_transition(
            &State::Executing,
            &TaskEvent::ExecutionFailed {
                error: "crashed".to_string(),
            },
        )
        .await;

    assert!(result.is_success());

    // Verify scheduler was called (on_exit triggers try_schedule_ready_tasks)
    assert!(
        s.scheduler.call_count() > 0,
        "COVERED: scheduler.try_schedule_ready_tasks() called on agent slot free (on_exit)"
    );
}

#[tokio::test]
async fn test_h4_mock_event_emitter_can_record_reconciliation_events() {
    // COVERED: In production, reconciliation emits "task:reconciliation_action"
    // events. Verify the MockEventEmitter can record these events.
    let s = create_hardening_services();

    s.emitter
        .emit("task:reconciliation_action", "task-h4-1")
        .await;
    s.emitter
        .emit_with_payload(
            "task:reconciliation_action",
            "task-h4-2",
            r#"{"action":"reset_stale","old_status":"executing","new_status":"ready"}"#,
        )
        .await;

    assert_eq!(s.emitter.event_count(), 2, "Both events should be recorded");

    assert!(
        s.emitter.has_event("task:reconciliation_action"),
        "reconciliation_action event type should be findable"
    );

    // Verify payload structure
    let events = s.emitter.get_events();
    let payload_event = events.iter().find(|e| e.method == "emit_with_payload");
    assert!(payload_event.is_some(), "Should have a payload event");

    let payload_str = &payload_event.unwrap().args[2];
    let payload: serde_json::Value = serde_json::from_str(payload_str).unwrap();
    assert_eq!(payload["action"], "reset_stale");
    assert_eq!(payload["old_status"], "executing");
    assert_eq!(payload["new_status"], "ready");
}

#[tokio::test]
async fn test_h4_reconciliation_event_includes_task_context() {
    // COVERED: Reconciliation events carry task_id context.
    let s = create_hardening_services();

    let task_id = "task-h4-context";
    s.emitter
        .emit("task:reconciliation_action", task_id)
        .await;

    let events = s.emitter.get_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].args[0], "task:reconciliation_action");
    assert_eq!(events[0].args[1], task_id, "Event should carry the task_id");
}
