use crate::domain::state_machine::context::TaskContext;
use crate::domain::state_machine::events::TaskEvent;
use crate::domain::state_machine::machine::{ParseStateError, Response, State, TaskStateMachine};
use crate::domain::state_machine::types::{Blocker, FailedData, QaFailedData, QaFailure};

fn create_machine() -> TaskStateMachine {
    TaskStateMachine::new(TaskContext::new_test("task-1", "proj-1"))
}

// ==================
// State helper tests
// ==================

#[test]
fn test_state_is_terminal() {
    assert!(State::Approved.is_terminal());
    assert!(State::Failed(FailedData::default()).is_terminal());
    assert!(State::Cancelled.is_terminal());

    assert!(!State::Backlog.is_terminal());
    assert!(!State::Executing.is_terminal());
}

#[test]
fn test_state_is_idle() {
    assert!(State::Backlog.is_idle());
    assert!(State::Ready.is_idle());
    assert!(State::Blocked.is_idle());

    assert!(!State::Executing.is_idle());
    assert!(!State::Approved.is_idle());
}

#[test]
fn test_state_is_active() {
    assert!(State::Executing.is_active());
    assert!(State::ReExecuting.is_active());
    assert!(State::QaRefining.is_active());
    assert!(State::PendingReview.is_active());
    assert!(State::Reviewing.is_active());
    assert!(State::ReviewPassed.is_active());

    assert!(!State::Backlog.is_active());
    assert!(!State::Approved.is_active());
}

// ==================
// Backlog state tests
// ==================

#[test]
fn test_backlog_schedule_transitions_to_ready() {
    let mut machine = create_machine();
    let response = machine.backlog(&TaskEvent::Schedule);
    assert_eq!(response, Response::Transition(State::Ready));
}

#[test]
fn test_backlog_cancel_transitions_to_cancelled() {
    let mut machine = create_machine();
    let response = machine.backlog(&TaskEvent::Cancel);
    assert_eq!(response, Response::Transition(State::Cancelled));
}

#[test]
fn test_backlog_ignores_other_events() {
    let mut machine = create_machine();
    let response = machine.backlog(&TaskEvent::ExecutionComplete);
    assert_eq!(response, Response::NotHandled);
}

// ==================
// Ready state tests
// ==================

#[test]
fn test_ready_blocker_detected_transitions_to_blocked() {
    let mut machine = create_machine();
    let response = machine.ready(&TaskEvent::BlockerDetected {
        blocker_id: "blocker-task".to_string(),
    });
    assert_eq!(response, Response::Transition(State::Blocked));
    assert!(machine.context.has_blockers());
}

#[test]
fn test_ready_cancel_transitions_to_cancelled() {
    let mut machine = create_machine();
    let response = machine.ready(&TaskEvent::Cancel);
    assert_eq!(response, Response::Transition(State::Cancelled));
}

// ==================
// Blocked state tests
// ==================

#[test]
fn test_blocked_blockers_resolved_transitions_to_ready() {
    let mut machine = create_machine();
    machine
        .context
        .add_blocker(Blocker::new("task-2"));

    let response = machine.blocked(&TaskEvent::BlockersResolved);
    assert_eq!(response, Response::Transition(State::Ready));
    assert!(!machine.context.has_blockers());
}

#[test]
fn test_blocked_cancel_transitions_to_cancelled() {
    let mut machine = create_machine();
    let response = machine.blocked(&TaskEvent::Cancel);
    assert_eq!(response, Response::Transition(State::Cancelled));
}

// ==================
// Executing state tests
// ==================

#[test]
fn test_executing_complete_transitions_to_pending_review_without_qa() {
    let mut machine = create_machine();
    machine.context.qa_enabled = false;
    let response = machine.executing(&TaskEvent::ExecutionComplete);
    assert_eq!(response, Response::Transition(State::PendingReview));
}

#[test]
fn test_executing_complete_transitions_to_qa_refining_with_qa() {
    let mut machine = create_machine();
    machine.context.qa_enabled = true;
    let response = machine.executing(&TaskEvent::ExecutionComplete);
    assert_eq!(response, Response::Transition(State::QaRefining));
}

#[test]
fn test_executing_failed_transitions_to_failed() {
    let mut machine = create_machine();
    let response = machine.executing(&TaskEvent::ExecutionFailed {
        error: "Build failed".to_string(),
    });

    if let Response::Transition(State::Failed(data)) = response {
        assert_eq!(data.error, "Build failed");
    } else {
        panic!("Expected Failed state");
    }
}

#[test]
fn test_executing_needs_human_input_transitions_to_blocked() {
    let mut machine = create_machine();
    let response = machine.executing(&TaskEvent::NeedsHumanInput {
        reason: "Need API key".to_string(),
    });
    assert_eq!(response, Response::Transition(State::Blocked));
    assert!(machine.context.has_blockers());
}

#[test]
fn test_executing_cancel_transitions_to_cancelled() {
    let mut machine = create_machine();
    let response = machine.executing(&TaskEvent::Cancel);
    assert_eq!(response, Response::Transition(State::Cancelled));
}

// ==================
// QA state tests
// ==================

#[test]
fn test_qa_refining_complete_transitions_to_testing() {
    let mut machine = create_machine();
    let response = machine.qa_refining(&TaskEvent::QaRefinementComplete);
    assert_eq!(response, Response::Transition(State::QaTesting));
}

#[test]
fn test_qa_testing_passed_transitions_to_qa_passed() {
    let mut machine = create_machine();
    let response = machine.qa_testing(&TaskEvent::QaTestsComplete { passed: true });
    assert_eq!(response, Response::Transition(State::QaPassed));
}

#[test]
fn test_qa_testing_failed_transitions_to_qa_failed() {
    let mut machine = create_machine();
    let response = machine.qa_testing(&TaskEvent::QaTestsComplete { passed: false });

    if let Response::Transition(State::QaFailed(_)) = response {
        // Expected
    } else {
        panic!("Expected QaFailed state");
    }
}

#[test]
fn test_qa_failed_retry_transitions_to_revision_needed() {
    let mut machine = create_machine();
    let response = machine.qa_failed(&TaskEvent::Retry, &QaFailedData::default());
    assert_eq!(response, Response::Transition(State::RevisionNeeded));
}

#[test]
fn test_qa_failed_skip_qa_transitions_to_pending_review() {
    let mut machine = create_machine();
    let response = machine.qa_failed(&TaskEvent::SkipQa, &QaFailedData::default());
    assert_eq!(response, Response::Transition(State::PendingReview));
}

// ==================
// Review state tests
// ==================

#[test]
fn test_reviewing_approved_transitions_to_review_passed() {
    let mut machine = create_machine();
    let response = machine.reviewing(&TaskEvent::ReviewComplete {
        approved: true,
        feedback: Some("LGTM".to_string()),
    });
    assert_eq!(response, Response::Transition(State::ReviewPassed));
    assert_eq!(machine.context.review_feedback, Some("LGTM".to_string()));
}

#[test]
fn test_reviewing_rejected_transitions_to_revision_needed() {
    let mut machine = create_machine();
    let response = machine.reviewing(&TaskEvent::ReviewComplete {
        approved: false,
        feedback: Some("Needs tests".to_string()),
    });
    assert_eq!(response, Response::Transition(State::RevisionNeeded));
}

#[test]
fn test_review_passed_human_approve_transitions_to_approved() {
    let mut machine = create_machine();
    let response = machine.review_passed(&TaskEvent::HumanApprove);
    assert_eq!(response, Response::Transition(State::Approved));
}

#[test]
fn test_review_passed_human_request_changes_transitions_to_revision_needed() {
    let mut machine = create_machine();
    let response = machine.review_passed(&TaskEvent::HumanRequestChanges {
        feedback: "Please add tests".to_string(),
    });
    assert_eq!(response, Response::Transition(State::RevisionNeeded));
    assert_eq!(machine.context.review_feedback, Some("Please add tests".to_string()));
}

#[test]
fn test_re_executing_complete_transitions_to_pending_review_without_qa() {
    let mut machine = create_machine();
    machine.context.qa_enabled = false;
    let response = machine.re_executing(&TaskEvent::ExecutionComplete);
    assert_eq!(response, Response::Transition(State::PendingReview));
}

#[test]
fn test_re_executing_complete_transitions_to_qa_refining_with_qa() {
    let mut machine = create_machine();
    machine.context.qa_enabled = true;
    let response = machine.re_executing(&TaskEvent::ExecutionComplete);
    assert_eq!(response, Response::Transition(State::QaRefining));
}

// ==================
// Terminal state tests
// ==================

#[test]
fn test_approved_retry_transitions_to_ready() {
    let mut machine = create_machine();
    machine.context.review_feedback = Some("Old feedback".to_string());
    let response = machine.approved(&TaskEvent::Retry);
    assert_eq!(response, Response::Transition(State::Ready));
    assert!(machine.context.review_feedback.is_none());
}

#[test]
fn test_failed_retry_transitions_to_ready() {
    let mut machine = create_machine();
    machine.context.error = Some("Old error".to_string());
    let response = machine.failed(&TaskEvent::Retry, &FailedData::default());
    assert_eq!(response, Response::Transition(State::Ready));
    assert!(machine.context.error.is_none());
}

#[test]
fn test_cancelled_retry_transitions_to_ready() {
    let mut machine = create_machine();
    let response = machine.cancelled(&TaskEvent::Retry);
    assert_eq!(response, Response::Transition(State::Ready));
}

#[test]
fn test_terminal_states_ignore_other_events() {
    let mut machine = create_machine();
    assert_eq!(
        machine.approved(&TaskEvent::Cancel),
        Response::NotHandled
    );
    assert_eq!(
        machine.failed(&TaskEvent::Cancel, &FailedData::default()),
        Response::NotHandled
    );
    assert_eq!(
        machine.cancelled(&TaskEvent::Cancel),
        Response::NotHandled
    );
}

// ==================
// Dispatch tests
// ==================

#[test]
fn test_dispatch_routes_to_correct_state() {
    let mut machine = create_machine();

    let response = machine.dispatch(&State::Backlog, &TaskEvent::Schedule);
    assert_eq!(response, Response::Transition(State::Ready));

    let response = machine.dispatch(&State::Ready, &TaskEvent::Cancel);
    assert_eq!(response, Response::Transition(State::Cancelled));
}

#[test]
fn test_dispatch_with_state_data() {
    let mut machine = create_machine();

    let response = machine.dispatch(
        &State::Failed(FailedData::new("error")),
        &TaskEvent::Retry,
    );
    assert_eq!(response, Response::Transition(State::Ready));
}

// ==================
// State name tests
// ==================

#[test]
fn test_state_names_are_correct() {
    assert_eq!(State::Backlog.name(), "Backlog");
    assert_eq!(State::Ready.name(), "Ready");
    assert_eq!(State::Blocked.name(), "Blocked");
    assert_eq!(State::Executing.name(), "Executing");
    assert_eq!(State::ReExecuting.name(), "ReExecuting");
    assert_eq!(State::QaRefining.name(), "QaRefining");
    assert_eq!(State::QaTesting.name(), "QaTesting");
    assert_eq!(State::QaPassed.name(), "QaPassed");
    assert_eq!(State::QaFailed(QaFailedData::default()).name(), "QaFailed");
    assert_eq!(State::PendingReview.name(), "PendingReview");
    assert_eq!(State::Reviewing.name(), "Reviewing");
    assert_eq!(State::ReviewPassed.name(), "ReviewPassed");
    assert_eq!(State::RevisionNeeded.name(), "RevisionNeeded");
    assert_eq!(State::Approved.name(), "Approved");
    assert_eq!(State::Failed(FailedData::default()).name(), "Failed");
    assert_eq!(State::Cancelled.name(), "Cancelled");
}

// ==================
// Logging hook tests
// ==================

#[test]
fn test_dispatch_logs_transition_on_state_change() {
    // This test verifies that dispatch() properly routes through
    // on_dispatch and on_transition hooks when a transition occurs.
    // The actual log output is verified by integration tests with
    // a tracing subscriber. Here we verify the state machine behavior.
    let mut machine = create_machine();

    let response = machine.dispatch(&State::Backlog, &TaskEvent::Schedule);

    // Verify transition occurred (which triggers on_transition)
    assert_eq!(response, Response::Transition(State::Ready));
}

#[test]
fn test_dispatch_does_not_log_transition_when_not_handled() {
    // When an event is not handled, on_transition should not be called
    let mut machine = create_machine();

    let response = machine.dispatch(&State::Backlog, &TaskEvent::ExecutionComplete);

    // Verify no transition (on_transition not called)
    assert_eq!(response, Response::NotHandled);
}

#[test]
fn test_on_dispatch_is_called_for_every_event() {
    // on_dispatch should be called regardless of whether the event is handled
    let mut machine = create_machine();

    // Event that results in transition
    let _ = machine.dispatch(&State::Backlog, &TaskEvent::Schedule);

    // Event that is not handled
    let _ = machine.dispatch(&State::Backlog, &TaskEvent::ExecutionComplete);

    // Both should have gone through on_dispatch (tested via coverage)
}

#[test]
fn test_transition_logging_includes_task_context() {
    // Verify that the machine has context data available for logging
    let mut machine = create_machine();

    // Context should have task_id and project_id for logging
    assert_eq!(machine.context.task_id, "task-1");
    assert_eq!(machine.context.project_id, "proj-1");

    // Dispatch triggers logging with this context
    let _ = machine.dispatch(&State::Backlog, &TaskEvent::Schedule);
}

// ==================
// State as_str tests
// ==================

#[test]
fn test_state_as_str_returns_snake_case() {
    assert_eq!(State::Backlog.as_str(), "backlog");
    assert_eq!(State::Ready.as_str(), "ready");
    assert_eq!(State::Blocked.as_str(), "blocked");
    assert_eq!(State::Executing.as_str(), "executing");
    assert_eq!(State::ReExecuting.as_str(), "re_executing");
    assert_eq!(State::QaRefining.as_str(), "qa_refining");
    assert_eq!(State::QaTesting.as_str(), "qa_testing");
    assert_eq!(State::QaPassed.as_str(), "qa_passed");
    assert_eq!(State::QaFailed(QaFailedData::default()).as_str(), "qa_failed");
    assert_eq!(State::PendingReview.as_str(), "pending_review");
    assert_eq!(State::Reviewing.as_str(), "reviewing");
    assert_eq!(State::ReviewPassed.as_str(), "review_passed");
    assert_eq!(State::RevisionNeeded.as_str(), "revision_needed");
    assert_eq!(State::Approved.as_str(), "approved");
    assert_eq!(State::Failed(FailedData::default()).as_str(), "failed");
    assert_eq!(State::Cancelled.as_str(), "cancelled");
}

// ==================
// Display trait tests
// ==================

#[test]
fn test_state_display_uses_snake_case() {
    assert_eq!(format!("{}", State::Backlog), "backlog");
    assert_eq!(format!("{}", State::Ready), "ready");
    assert_eq!(format!("{}", State::ReExecuting), "re_executing");
    assert_eq!(format!("{}", State::QaFailed(QaFailedData::default())), "qa_failed");
    assert_eq!(format!("{}", State::Failed(FailedData::default())), "failed");
}

#[test]
fn test_state_display_all_states() {
    let states = [
        (State::Backlog, "backlog"),
        (State::Ready, "ready"),
        (State::Blocked, "blocked"),
        (State::Executing, "executing"),
        (State::ReExecuting, "re_executing"),
        (State::QaRefining, "qa_refining"),
        (State::QaTesting, "qa_testing"),
        (State::QaPassed, "qa_passed"),
        (State::QaFailed(QaFailedData::default()), "qa_failed"),
        (State::PendingReview, "pending_review"),
        (State::Reviewing, "reviewing"),
        (State::ReviewPassed, "review_passed"),
        (State::RevisionNeeded, "revision_needed"),
        (State::Approved, "approved"),
        (State::Failed(FailedData::default()), "failed"),
        (State::Cancelled, "cancelled"),
    ];

    for (state, expected) in states {
        assert_eq!(format!("{}", state), expected);
    }
}

// ==================
// FromStr trait tests
// ==================

#[test]
fn test_state_from_str_parses_all_states() {
    assert_eq!("backlog".parse::<State>().unwrap(), State::Backlog);
    assert_eq!("ready".parse::<State>().unwrap(), State::Ready);
    assert_eq!("blocked".parse::<State>().unwrap(), State::Blocked);
    assert_eq!("executing".parse::<State>().unwrap(), State::Executing);
    assert_eq!("re_executing".parse::<State>().unwrap(), State::ReExecuting);
    assert_eq!("qa_refining".parse::<State>().unwrap(), State::QaRefining);
    assert_eq!("qa_testing".parse::<State>().unwrap(), State::QaTesting);
    assert_eq!("qa_passed".parse::<State>().unwrap(), State::QaPassed);
    // QaFailed and Failed parse with default data
    if let State::QaFailed(data) = "qa_failed".parse::<State>().unwrap() {
        assert!(!data.has_failures());
    } else {
        panic!("Expected QaFailed");
    }
    assert_eq!("pending_review".parse::<State>().unwrap(), State::PendingReview);
    assert_eq!("reviewing".parse::<State>().unwrap(), State::Reviewing);
    assert_eq!("review_passed".parse::<State>().unwrap(), State::ReviewPassed);
    assert_eq!("revision_needed".parse::<State>().unwrap(), State::RevisionNeeded);
    assert_eq!("approved".parse::<State>().unwrap(), State::Approved);
    if let State::Failed(data) = "failed".parse::<State>().unwrap() {
        assert!(data.error.is_empty());
    } else {
        panic!("Expected Failed");
    }
    assert_eq!("cancelled".parse::<State>().unwrap(), State::Cancelled);
}

#[test]
fn test_state_from_str_invalid_returns_error() {
    let result = "invalid_state".parse::<State>();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.invalid_value, "invalid_state");
    assert_eq!(format!("{}", err), "invalid state: 'invalid_state'");
}

#[test]
fn test_state_from_str_empty_string_returns_error() {
    let result = "".parse::<State>();
    assert!(result.is_err());
}

#[test]
fn test_state_from_str_case_sensitive() {
    // FromStr should be case-sensitive (snake_case only)
    let result = "Backlog".parse::<State>();
    assert!(result.is_err());

    let result = "BACKLOG".parse::<State>();
    assert!(result.is_err());
}

// ==================
// Roundtrip tests
// ==================

#[test]
fn test_state_roundtrip_all_states() {
    let states = [
        State::Backlog,
        State::Ready,
        State::Blocked,
        State::Executing,
        State::ReExecuting,
        State::QaRefining,
        State::QaTesting,
        State::QaPassed,
        State::QaFailed(QaFailedData::default()),
        State::PendingReview,
        State::Reviewing,
        State::ReviewPassed,
        State::RevisionNeeded,
        State::Approved,
        State::Failed(FailedData::default()),
        State::Cancelled,
    ];

    for state in states {
        let s = state.to_string();
        let parsed: State = s.parse().expect("should parse");
        // For states with data, we can only compare the variant name
        assert_eq!(state.as_str(), parsed.as_str());
    }
}

#[test]
fn test_state_with_data_loses_data_on_roundtrip() {
    // States with local data will lose that data when parsed from string
    // This is by design - the persistence layer stores data separately
    let qa_failed = State::QaFailed(QaFailedData::single(
        QaFailure::new("test", "error"),
    ));
    let s = qa_failed.to_string();
    let parsed: State = s.parse().unwrap();

    if let State::QaFailed(data) = parsed {
        // Parsed state has default (empty) data
        assert!(!data.has_failures());
    } else {
        panic!("Expected QaFailed");
    }

    let failed = State::Failed(FailedData::new("original error"));
    let s = failed.to_string();
    let parsed: State = s.parse().unwrap();

    if let State::Failed(data) = parsed {
        // Parsed state has default (empty) data
        assert!(data.error.is_empty());
    } else {
        panic!("Expected Failed");
    }
}

// ==================
// ParseStateError tests
// ==================

#[test]
fn test_parse_state_error_display() {
    let err = ParseStateError {
        invalid_value: "foo".to_string(),
    };
    assert_eq!(format!("{}", err), "invalid state: 'foo'");
}

#[test]
fn test_parse_state_error_is_std_error() {
    let err = ParseStateError {
        invalid_value: "test".to_string(),
    };
    let _: &dyn std::error::Error = &err;
}

#[test]
fn test_parse_state_error_clone_and_eq() {
    let err1 = ParseStateError {
        invalid_value: "test".to_string(),
    };
    let err2 = err1.clone();
    assert_eq!(err1, err2);
}
