// State machine transition validation tests for BranchFreshnessConflict event.
//
// Tests all 3 freshness → Merging paths at the state machine dispatch level:
//   Executing + BranchFreshnessConflict → Merging
//   ReExecuting + BranchFreshnessConflict → Merging
//   Reviewing + BranchFreshnessConflict → Merging
//
// Also validates that BranchFreshnessConflict is NOT handled in other states.

use super::helpers::{create_context_with_services, create_test_services};
use crate::domain::state_machine::machine::types::Response;
use crate::domain::state_machine::{State, TaskEvent, TaskStateMachine};

// ==================
// Executing → Merging
// ==================

#[test]
fn test_executing_branch_freshness_conflict_transitions_to_merging() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::Executing, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::Transition(State::Merging),
        "Executing + BranchFreshnessConflict must transition to Merging"
    );
}

// ==================
// ReExecuting → Merging
// ==================

#[test]
fn test_re_executing_branch_freshness_conflict_transitions_to_merging() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::ReExecuting, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::Transition(State::Merging),
        "ReExecuting + BranchFreshnessConflict must transition to Merging"
    );
}

// ==================
// Reviewing → Merging
// ==================

#[test]
fn test_reviewing_branch_freshness_conflict_transitions_to_merging() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::Reviewing, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::Transition(State::Merging),
        "Reviewing + BranchFreshnessConflict must transition to Merging"
    );
}

// ==================
// States that must NOT handle BranchFreshnessConflict
// ==================

#[test]
fn test_ready_does_not_handle_branch_freshness_conflict() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::Ready, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::NotHandled,
        "Ready must NOT handle BranchFreshnessConflict"
    );
}

#[test]
fn test_merging_does_not_handle_branch_freshness_conflict() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::Merging, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::NotHandled,
        "Merging must NOT handle BranchFreshnessConflict"
    );
}

#[test]
fn test_backlog_does_not_handle_branch_freshness_conflict() {
    let (_, _, _, _, _, services) = create_test_services();
    let context = create_context_with_services("task-1", "proj-1", services);
    let mut machine = TaskStateMachine::new(context);

    let response = machine.dispatch(&State::Backlog, &TaskEvent::BranchFreshnessConflict);

    assert_eq!(
        response,
        Response::NotHandled,
        "Backlog must NOT handle BranchFreshnessConflict"
    );
}

// ==================
// Event classification
// ==================

#[test]
fn test_branch_freshness_conflict_is_system_signal() {
    assert!(
        TaskEvent::BranchFreshnessConflict.is_system_signal(),
        "BranchFreshnessConflict must be classified as a system signal"
    );
    assert!(
        !TaskEvent::BranchFreshnessConflict.is_user_action(),
        "BranchFreshnessConflict must NOT be a user action"
    );
    assert!(
        !TaskEvent::BranchFreshnessConflict.is_agent_signal(),
        "BranchFreshnessConflict must NOT be an agent signal"
    );
}

#[test]
fn test_branch_freshness_conflict_name() {
    assert_eq!(
        TaskEvent::BranchFreshnessConflict.name(),
        "BranchFreshnessConflict"
    );
}
