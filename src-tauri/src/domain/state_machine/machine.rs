// TaskStateMachine - statig-based state machine for task lifecycle
// This implements the core 14-state task lifecycle with hierarchical superstates

use super::context::TaskContext;
use super::events::TaskEvent;
use super::types::{FailedData, QaFailedData};
use statig::prelude::*;

/// The task state machine shared data (context)
#[derive(Debug)]
pub struct TaskStateMachine {
    pub context: TaskContext,
}

/// All possible states for a task
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    // Idle states
    Backlog,
    Ready,
    Blocked,

    // Execution states
    Executing,
    ExecutionDone,

    // QA states
    QaRefining,
    QaTesting,
    QaPassed,
    QaFailed(QaFailedData),

    // Review states
    PendingReview,
    RevisionNeeded,

    // Terminal states
    Approved,
    Failed(FailedData),
    Cancelled,
}

impl State {
    /// Returns true if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, State::Approved | State::Failed(_) | State::Cancelled)
    }

    /// Returns true if this is an idle state
    pub fn is_idle(&self) -> bool {
        matches!(self, State::Backlog | State::Ready | State::Blocked)
    }

    /// Returns true if this is an active (non-idle, non-terminal) state
    pub fn is_active(&self) -> bool {
        !self.is_idle() && !self.is_terminal()
    }
}

/// Response type for state machine transitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Transition was handled
    Handled,
    /// Event was not applicable in current state
    NotHandled,
    /// Transition to a new state
    Transition(State),
}

impl TaskStateMachine {
    pub fn new(context: TaskContext) -> Self {
        Self { context }
    }

    // ==================
    // Idle States
    // ==================

    /// Backlog state - task is parked, not ready for work
    pub fn backlog(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Schedule => Response::Transition(State::Ready),
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// Ready state - task is ready to be picked up
    pub fn ready(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::BlockerDetected { blocker_id } => {
                // Add blocker to context
                self.context
                    .add_blocker(super::types::Blocker::new(blocker_id.clone()));
                Response::Transition(State::Blocked)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Auto-transition to Executing happens via entry action check
            _ => Response::NotHandled,
        }
    }

    /// Blocked state - waiting on dependencies or human input
    pub fn blocked(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::BlockersResolved => {
                self.context.resolve_all_blockers();
                Response::Transition(State::Ready)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    // ==================
    // Execution States
    // ==================

    /// Executing state - worker agent is actively running
    pub fn executing(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::ExecutionComplete => Response::Transition(State::ExecutionDone),
            TaskEvent::ExecutionFailed { error } => {
                Response::Transition(State::Failed(FailedData::new(error.clone())))
            }
            TaskEvent::NeedsHumanInput { reason } => {
                self.context
                    .add_blocker(super::types::Blocker::human_input(reason.clone()));
                Response::Transition(State::Blocked)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// ExecutionDone state - worker finished, deciding next step
    pub fn execution_done(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Auto-transitions based on qa_enabled are handled separately
            _ => Response::NotHandled,
        }
    }

    // ==================
    // QA States
    // ==================

    /// QaRefining state - QA agent refining test plan based on actual changes
    pub fn qa_refining(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::QaRefinementComplete => Response::Transition(State::QaTesting),
            TaskEvent::ExecutionFailed { error } => {
                Response::Transition(State::Failed(FailedData::new(error.clone())))
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// QaTesting state - QA tests are executing
    pub fn qa_testing(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::QaTestsComplete { passed: true } => Response::Transition(State::QaPassed),
            TaskEvent::QaTestsComplete { passed: false } => {
                Response::Transition(State::QaFailed(QaFailedData::default()))
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// QaPassed state - all QA tests passed
    pub fn qa_passed(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Auto-transition to PendingReview happens via entry action
            _ => Response::NotHandled,
        }
    }

    /// QaFailed state - QA tests failed
    pub fn qa_failed(&mut self, event: &TaskEvent, _data: &QaFailedData) -> Response {
        match event {
            TaskEvent::Retry => Response::Transition(State::RevisionNeeded),
            TaskEvent::SkipQa => Response::Transition(State::PendingReview),
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    // ==================
    // Review States
    // ==================

    /// PendingReview state - awaiting AI reviewer
    pub fn pending_review(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::ReviewComplete {
                approved: true,
                feedback,
            } => {
                if let Some(fb) = feedback {
                    self.context.review_feedback = Some(fb.clone());
                }
                Response::Transition(State::Approved)
            }
            TaskEvent::ReviewComplete {
                approved: false,
                feedback,
            } => {
                if let Some(fb) = feedback {
                    self.context.review_feedback = Some(fb.clone());
                }
                Response::Transition(State::RevisionNeeded)
            }
            TaskEvent::ForceApprove => Response::Transition(State::Approved),
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// RevisionNeeded state - review found issues
    pub fn revision_needed(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Auto-transition to Executing happens via entry action
            _ => Response::NotHandled,
        }
    }

    // ==================
    // Terminal States
    // ==================

    /// Approved state - complete and verified
    pub fn approved(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Retry => {
                self.context.clear_review_feedback();
                Response::Transition(State::Ready)
            }
            _ => Response::NotHandled,
        }
    }

    /// Failed state - requires manual intervention
    pub fn failed(&mut self, event: &TaskEvent, _data: &FailedData) -> Response {
        match event {
            TaskEvent::Retry => {
                self.context.clear_error();
                Response::Transition(State::Ready)
            }
            _ => Response::NotHandled,
        }
    }

    /// Cancelled state - intentionally abandoned
    pub fn cancelled(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Retry => Response::Transition(State::Ready),
            _ => Response::NotHandled,
        }
    }

    /// Dispatch an event to the current state
    pub fn dispatch(&mut self, state: &State, event: &TaskEvent) -> Response {
        match state {
            State::Backlog => self.backlog(event),
            State::Ready => self.ready(event),
            State::Blocked => self.blocked(event),
            State::Executing => self.executing(event),
            State::ExecutionDone => self.execution_done(event),
            State::QaRefining => self.qa_refining(event),
            State::QaTesting => self.qa_testing(event),
            State::QaPassed => self.qa_passed(event),
            State::QaFailed(data) => self.qa_failed(event, data),
            State::PendingReview => self.pending_review(event),
            State::RevisionNeeded => self.revision_needed(event),
            State::Approved => self.approved(event),
            State::Failed(data) => self.failed(event, data),
            State::Cancelled => self.cancelled(event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::state_machine::context::TaskContext;

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
        assert!(State::ExecutionDone.is_active());
        assert!(State::QaRefining.is_active());
        assert!(State::PendingReview.is_active());

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
        assert!(machine.context.has_unresolved_blockers());
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
            .add_blocker(super::super::types::Blocker::new("task-2"));

        let response = machine.blocked(&TaskEvent::BlockersResolved);
        assert_eq!(response, Response::Transition(State::Ready));
        assert!(!machine.context.has_unresolved_blockers());
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
    fn test_executing_complete_transitions_to_execution_done() {
        let mut machine = create_machine();
        let response = machine.executing(&TaskEvent::ExecutionComplete);
        assert_eq!(response, Response::Transition(State::ExecutionDone));
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
        assert!(machine.context.has_unresolved_blockers());
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
    fn test_pending_review_approved_transitions_to_approved() {
        let mut machine = create_machine();
        let response = machine.pending_review(&TaskEvent::ReviewComplete {
            approved: true,
            feedback: Some("LGTM".to_string()),
        });
        assert_eq!(response, Response::Transition(State::Approved));
        assert_eq!(machine.context.review_feedback, Some("LGTM".to_string()));
    }

    #[test]
    fn test_pending_review_rejected_transitions_to_revision_needed() {
        let mut machine = create_machine();
        let response = machine.pending_review(&TaskEvent::ReviewComplete {
            approved: false,
            feedback: Some("Needs tests".to_string()),
        });
        assert_eq!(response, Response::Transition(State::RevisionNeeded));
    }

    #[test]
    fn test_pending_review_force_approve_transitions_to_approved() {
        let mut machine = create_machine();
        let response = machine.pending_review(&TaskEvent::ForceApprove);
        assert_eq!(response, Response::Transition(State::Approved));
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
}
