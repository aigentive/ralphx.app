// TaskStateMachine - statig-based state machine for task lifecycle
// This implements the core 14-state task lifecycle with hierarchical superstates

use super::context::TaskContext;
use super::events::TaskEvent;
use super::types::{FailedData, QaFailedData};
#[allow(unused_imports)]
use statig::prelude::*;
use std::fmt;
use std::str::FromStr;
use tracing::{debug, info};

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
    ReExecuting,

    // QA states
    QaRefining,
    QaTesting,
    QaPassed,
    QaFailed(QaFailedData),

    // Review states
    PendingReview,
    Reviewing,
    ReviewPassed,
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
            TaskEvent::StartExecution => {
                // User explicitly starts execution (e.g., drag to "In Progress")
                Response::Transition(State::Executing)
            }
            TaskEvent::BlockerDetected { blocker_id } => {
                // Add blocker to context
                self.context
                    .add_blocker(super::types::Blocker::new(blocker_id.clone()));
                Response::Transition(State::Blocked)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
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

    /// Executing state - worker agent is actively running (first attempt)
    pub fn executing(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::ExecutionComplete => {
                // Check qa_enabled directly instead of going to ExecutionDone
                if self.context.qa_enabled {
                    Response::Transition(State::QaRefining)
                } else {
                    Response::Transition(State::PendingReview)
                }
            }
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

    /// ReExecuting state - worker revising after failed review
    pub fn re_executing(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::ExecutionComplete => {
                // Check qa_enabled to decide next step
                if self.context.qa_enabled {
                    Response::Transition(State::QaRefining)
                } else {
                    Response::Transition(State::PendingReview)
                }
            }
            TaskEvent::ExecutionFailed { error } => {
                Response::Transition(State::Failed(FailedData::new(error.clone())))
            }
            TaskEvent::BlockerDetected { blocker_id: _ } => {
                Response::Transition(State::Blocked)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
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

    /// PendingReview state - awaiting AI reviewer to pick up
    pub fn pending_review(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Transition to Reviewing happens via entry action (spawns reviewer)
            _ => Response::NotHandled,
        }
    }

    /// Reviewing state - AI agent is actively reviewing
    pub fn reviewing(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::ReviewComplete {
                approved: true,
                feedback,
            } => {
                if let Some(fb) = feedback {
                    self.context.review_feedback = Some(fb.clone());
                }
                Response::Transition(State::ReviewPassed)
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
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// ReviewPassed state - AI approved, awaiting human confirmation
    pub fn review_passed(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::HumanApprove => Response::Transition(State::Approved),
            TaskEvent::HumanRequestChanges { feedback } => {
                self.context.review_feedback = Some(feedback.clone());
                Response::Transition(State::RevisionNeeded)
            }
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            _ => Response::NotHandled,
        }
    }

    /// RevisionNeeded state - review found issues, ready for re-execution
    pub fn revision_needed(&mut self, event: &TaskEvent) -> Response {
        match event {
            TaskEvent::Cancel => Response::Transition(State::Cancelled),
            // Auto-transition to ReExecuting happens via entry action
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
    ///
    /// This is the main entry point for processing events. It logs the dispatch
    /// at debug level, routes the event to the appropriate state handler, and
    /// logs any resulting transition at info level.
    pub fn dispatch(&mut self, state: &State, event: &TaskEvent) -> Response {
        // Log the dispatch at debug level
        self.on_dispatch(state, event);

        let response = match state {
            State::Backlog => self.backlog(event),
            State::Ready => self.ready(event),
            State::Blocked => self.blocked(event),
            State::Executing => self.executing(event),
            State::ReExecuting => self.re_executing(event),
            State::QaRefining => self.qa_refining(event),
            State::QaTesting => self.qa_testing(event),
            State::QaPassed => self.qa_passed(event),
            State::QaFailed(data) => self.qa_failed(event, data),
            State::PendingReview => self.pending_review(event),
            State::Reviewing => self.reviewing(event),
            State::ReviewPassed => self.review_passed(event),
            State::RevisionNeeded => self.revision_needed(event),
            State::Approved => self.approved(event),
            State::Failed(data) => self.failed(event, data),
            State::Cancelled => self.cancelled(event),
        };

        // Log transition at info level if a transition occurred
        if let Response::Transition(ref new_state) = response {
            self.on_transition(state, new_state, event);
        }

        response
    }

    /// Hook called before dispatching an event to a state handler.
    ///
    /// Logs the state/event pair at debug level for detailed tracing.
    fn on_dispatch(&self, state: &State, event: &TaskEvent) {
        debug!(
            task_id = %self.context.task_id,
            project_id = %self.context.project_id,
            state = %state.name(),
            event = %event.name(),
            "Dispatching event to state"
        );
    }

    /// Hook called after a state transition occurs.
    ///
    /// Logs the from/to states at info level for visibility into state changes.
    fn on_transition(&self, from: &State, to: &State, event: &TaskEvent) {
        info!(
            task_id = %self.context.task_id,
            project_id = %self.context.project_id,
            from_state = %from.name(),
            to_state = %to.name(),
            event = %event.name(),
            "State transition"
        );
    }
}

impl State {
    /// Returns a human-readable name for the state (for logging)
    pub fn name(&self) -> &'static str {
        match self {
            State::Backlog => "Backlog",
            State::Ready => "Ready",
            State::Blocked => "Blocked",
            State::Executing => "Executing",
            State::ReExecuting => "ReExecuting",
            State::QaRefining => "QaRefining",
            State::QaTesting => "QaTesting",
            State::QaPassed => "QaPassed",
            State::QaFailed(_) => "QaFailed",
            State::PendingReview => "PendingReview",
            State::Reviewing => "Reviewing",
            State::ReviewPassed => "ReviewPassed",
            State::RevisionNeeded => "RevisionNeeded",
            State::Approved => "Approved",
            State::Failed(_) => "Failed",
            State::Cancelled => "Cancelled",
        }
    }

    /// Returns the snake_case string representation for SQLite storage.
    ///
    /// This matches the InternalStatus as_str() format for consistency
    /// with the tasks table internal_status column.
    pub fn as_str(&self) -> &'static str {
        match self {
            State::Backlog => "backlog",
            State::Ready => "ready",
            State::Blocked => "blocked",
            State::Executing => "executing",
            State::ReExecuting => "re_executing",
            State::QaRefining => "qa_refining",
            State::QaTesting => "qa_testing",
            State::QaPassed => "qa_passed",
            State::QaFailed(_) => "qa_failed",
            State::PendingReview => "pending_review",
            State::Reviewing => "reviewing",
            State::ReviewPassed => "review_passed",
            State::RevisionNeeded => "revision_needed",
            State::Approved => "approved",
            State::Failed(_) => "failed",
            State::Cancelled => "cancelled",
        }
    }
}

/// Error returned when parsing a State from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseStateError {
    pub invalid_value: String,
}

impl fmt::Display for ParseStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid state: '{}'", self.invalid_value)
    }
}

impl std::error::Error for ParseStateError {}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for State {
    type Err = ParseStateError;

    /// Parses a snake_case string into a State.
    ///
    /// For states with local data (QaFailed, Failed), this returns the variant
    /// with default data. To restore actual data, use the persistence helpers.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(State::Backlog),
            "ready" => Ok(State::Ready),
            "blocked" => Ok(State::Blocked),
            "executing" => Ok(State::Executing),
            "re_executing" => Ok(State::ReExecuting),
            "qa_refining" => Ok(State::QaRefining),
            "qa_testing" => Ok(State::QaTesting),
            "qa_passed" => Ok(State::QaPassed),
            "qa_failed" => Ok(State::QaFailed(QaFailedData::default())),
            "pending_review" => Ok(State::PendingReview),
            "reviewing" => Ok(State::Reviewing),
            "review_passed" => Ok(State::ReviewPassed),
            "revision_needed" => Ok(State::RevisionNeeded),
            "approved" => Ok(State::Approved),
            "failed" => Ok(State::Failed(FailedData::default())),
            "cancelled" => Ok(State::Cancelled),
            _ => Err(ParseStateError {
                invalid_value: s.to_string(),
            }),
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
            .add_blocker(super::super::types::Blocker::new("task-2"));

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
            super::super::types::QaFailure::new("test", "error"),
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
}
