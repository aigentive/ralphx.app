// InternalStatus enum representing the 14 internal states of a task
// with transition validation for the state machine

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The 14 internal statuses that a task can be in.
/// These map to external Kanban columns via WorkflowSchema (Phase 11).
///
/// State machine transitions are validated - not all transitions are allowed.
/// Use `can_transition_to()` to check if a transition is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalStatus {
    /// Task is in the backlog, not yet ready for execution
    Backlog,
    /// Task is ready to be picked up for execution
    Ready,
    /// Task is blocked by dependencies or external factors
    Blocked,
    /// Task is currently being executed by an agent
    Executing,
    /// QA is refining test criteria based on implementation
    QaRefining,
    /// QA tests are being executed
    QaTesting,
    /// QA tests have passed
    QaPassed,
    /// QA tests have failed, needs revision
    QaFailed,
    /// Awaiting code review (AI or human)
    PendingReview,
    /// AI agent is actively reviewing
    Reviewing,
    /// AI approved, awaiting human confirmation
    ReviewPassed,
    /// AI couldn't decide, escalated to human for decision
    Escalated,
    /// Reviewer requested changes
    RevisionNeeded,
    /// Worker is revising based on review feedback
    ReExecuting,
    /// Task has been approved and is complete
    Approved,
    /// Task has permanently failed after max retries
    Failed,
    /// Task was cancelled by user
    Cancelled,
}

impl InternalStatus {
    /// Returns the list of valid statuses this status can transition to.
    /// This enforces the state machine rules from the master plan.
    pub fn valid_transitions(&self) -> &'static [InternalStatus] {
        use InternalStatus::*;
        match self {
            // Idle states
            Backlog => &[Ready, Cancelled],
            Ready => &[Executing, Blocked, Cancelled],
            Blocked => &[Ready, Cancelled],

            // Execution states
            Executing => &[QaRefining, PendingReview, Failed, Blocked],

            // QA states
            QaRefining => &[QaTesting],
            QaTesting => &[QaPassed, QaFailed],
            QaPassed => &[PendingReview],
            QaFailed => &[RevisionNeeded],

            // Review states
            PendingReview => &[Reviewing],
            Reviewing => &[ReviewPassed, RevisionNeeded, Escalated],
            ReviewPassed => &[Approved, RevisionNeeded],
            Escalated => &[Approved, RevisionNeeded],
            RevisionNeeded => &[ReExecuting, Cancelled],
            ReExecuting => &[PendingReview, Failed, Blocked],

            // Terminal states (can be re-opened)
            Approved => &[Ready],
            Failed => &[Ready],
            Cancelled => &[Ready],
        }
    }

    /// Checks if transitioning to the target status is valid.
    pub fn can_transition_to(&self, target: InternalStatus) -> bool {
        self.valid_transitions().contains(&target)
    }

    /// Returns all possible InternalStatus variants.
    /// Useful for iteration and testing.
    pub fn all_variants() -> &'static [InternalStatus] {
        use InternalStatus::*;
        &[
            Backlog,
            Ready,
            Blocked,
            Executing,
            QaRefining,
            QaTesting,
            QaPassed,
            QaFailed,
            PendingReview,
            Reviewing,
            ReviewPassed,
            Escalated,
            RevisionNeeded,
            ReExecuting,
            Approved,
            Failed,
            Cancelled,
        ]
    }

    /// Returns the snake_case string representation (matches serde serialization)
    pub fn as_str(&self) -> &'static str {
        match self {
            InternalStatus::Backlog => "backlog",
            InternalStatus::Ready => "ready",
            InternalStatus::Blocked => "blocked",
            InternalStatus::Executing => "executing",
            InternalStatus::QaRefining => "qa_refining",
            InternalStatus::QaTesting => "qa_testing",
            InternalStatus::QaPassed => "qa_passed",
            InternalStatus::QaFailed => "qa_failed",
            InternalStatus::PendingReview => "pending_review",
            InternalStatus::Reviewing => "reviewing",
            InternalStatus::ReviewPassed => "review_passed",
            InternalStatus::Escalated => "escalated",
            InternalStatus::RevisionNeeded => "revision_needed",
            InternalStatus::ReExecuting => "re_executing",
            InternalStatus::Approved => "approved",
            InternalStatus::Failed => "failed",
            InternalStatus::Cancelled => "cancelled",
        }
    }
}

impl fmt::Display for InternalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for parsing InternalStatus from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseInternalStatusError {
    pub value: String,
}

impl fmt::Display for ParseInternalStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown internal status: '{}'", self.value)
    }
}

impl std::error::Error for ParseInternalStatusError {}

impl FromStr for InternalStatus {
    type Err = ParseInternalStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(InternalStatus::Backlog),
            "ready" => Ok(InternalStatus::Ready),
            "blocked" => Ok(InternalStatus::Blocked),
            "executing" => Ok(InternalStatus::Executing),
            "qa_refining" => Ok(InternalStatus::QaRefining),
            "qa_testing" => Ok(InternalStatus::QaTesting),
            "qa_passed" => Ok(InternalStatus::QaPassed),
            "qa_failed" => Ok(InternalStatus::QaFailed),
            "pending_review" => Ok(InternalStatus::PendingReview),
            "reviewing" => Ok(InternalStatus::Reviewing),
            "review_passed" => Ok(InternalStatus::ReviewPassed),
            "escalated" => Ok(InternalStatus::Escalated),
            "revision_needed" => Ok(InternalStatus::RevisionNeeded),
            "re_executing" => Ok(InternalStatus::ReExecuting),
            "approved" => Ok(InternalStatus::Approved),
            "failed" => Ok(InternalStatus::Failed),
            "cancelled" => Ok(InternalStatus::Cancelled),
            _ => Err(ParseInternalStatusError {
                value: s.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== All 14 Variants Exist Tests =====

    #[test]
    fn internal_status_has_17_variants() {
        assert_eq!(InternalStatus::all_variants().len(), 17);
    }

    #[test]
    fn all_variants_returns_correct_statuses() {
        use InternalStatus::*;
        let expected = vec![
            Backlog,
            Ready,
            Blocked,
            Executing,
            QaRefining,
            QaTesting,
            QaPassed,
            QaFailed,
            PendingReview,
            Reviewing,
            ReviewPassed,
            Escalated,
            RevisionNeeded,
            ReExecuting,
            Approved,
            Failed,
            Cancelled,
        ];
        assert_eq!(InternalStatus::all_variants(), expected.as_slice());
    }

    // ===== Serialization Tests (snake_case) =====

    #[test]
    fn serializes_to_snake_case_backlog() {
        let json = serde_json::to_string(&InternalStatus::Backlog).unwrap();
        assert_eq!(json, "\"backlog\"");
    }


    #[test]
    fn serializes_to_snake_case_qa_refining() {
        let json = serde_json::to_string(&InternalStatus::QaRefining).unwrap();
        assert_eq!(json, "\"qa_refining\"");
    }

    #[test]
    fn serializes_to_snake_case_pending_review() {
        let json = serde_json::to_string(&InternalStatus::PendingReview).unwrap();
        assert_eq!(json, "\"pending_review\"");
    }

    #[test]
    fn serializes_to_snake_case_revision_needed() {
        let json = serde_json::to_string(&InternalStatus::RevisionNeeded).unwrap();
        assert_eq!(json, "\"revision_needed\"");
    }

    #[test]
    fn serializes_to_snake_case_escalated() {
        let json = serde_json::to_string(&InternalStatus::Escalated).unwrap();
        assert_eq!(json, "\"escalated\"");
    }

    #[test]
    fn all_variants_serialize_correctly() {
        let expected_serializations = vec![
            ("backlog", InternalStatus::Backlog),
            ("ready", InternalStatus::Ready),
            ("blocked", InternalStatus::Blocked),
            ("executing", InternalStatus::Executing),
            ("qa_refining", InternalStatus::QaRefining),
            ("qa_testing", InternalStatus::QaTesting),
            ("qa_passed", InternalStatus::QaPassed),
            ("qa_failed", InternalStatus::QaFailed),
            ("pending_review", InternalStatus::PendingReview),
            ("reviewing", InternalStatus::Reviewing),
            ("review_passed", InternalStatus::ReviewPassed),
            ("escalated", InternalStatus::Escalated),
            ("revision_needed", InternalStatus::RevisionNeeded),
            ("re_executing", InternalStatus::ReExecuting),
            ("approved", InternalStatus::Approved),
            ("failed", InternalStatus::Failed),
            ("cancelled", InternalStatus::Cancelled),
        ];

        for (expected_str, status) in expected_serializations {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", expected_str), "Failed for {:?}", status);
        }
    }

    // ===== Deserialization Tests =====


    #[test]
    fn deserializes_all_variants() {
        for status in InternalStatus::all_variants() {
            let json = format!("\"{}\"", status.as_str());
            let parsed: InternalStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&parsed, status);
        }
    }

    #[test]
    fn deserialize_invalid_returns_error() {
        let result: Result<InternalStatus, _> = serde_json::from_str("\"invalid_status\"");
        assert!(result.is_err());
    }

    // ===== FromStr / Parse Tests =====

    #[test]
    fn from_str_parses_all_variants() {
        for status in InternalStatus::all_variants() {
            let parsed = InternalStatus::from_str(status.as_str()).unwrap();
            assert_eq!(&parsed, status);
        }
    }

    #[test]
    fn from_str_returns_error_for_invalid() {
        let result = InternalStatus::from_str("not_a_status");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.value, "not_a_status");
    }

    #[test]
    fn from_str_error_displays_correctly() {
        let err = ParseInternalStatusError {
            value: "unknown".to_string(),
        };
        assert_eq!(err.to_string(), "unknown internal status: 'unknown'");
    }

    // ===== Display / as_str Tests =====

    #[test]
    fn display_matches_as_str() {
        for status in InternalStatus::all_variants() {
            assert_eq!(format!("{}", status), status.as_str());
        }
    }

    #[test]
    fn as_str_returns_snake_case() {
        assert_eq!(InternalStatus::QaRefining.as_str(), "qa_refining");
        assert_eq!(InternalStatus::PendingReview.as_str(), "pending_review");
    }

    // ===== Valid Transitions Tests =====

    #[test]
    fn backlog_transitions() {
        use InternalStatus::*;
        let transitions = Backlog.valid_transitions();
        assert_eq!(transitions, &[Ready, Cancelled]);
    }

    #[test]
    fn ready_transitions() {
        use InternalStatus::*;
        let transitions = Ready.valid_transitions();
        assert_eq!(transitions, &[Executing, Blocked, Cancelled]);
    }

    #[test]
    fn blocked_transitions() {
        use InternalStatus::*;
        let transitions = Blocked.valid_transitions();
        assert_eq!(transitions, &[Ready, Cancelled]);
    }

    #[test]
    fn executing_transitions() {
        use InternalStatus::*;
        let transitions = Executing.valid_transitions();
        assert_eq!(transitions, &[QaRefining, PendingReview, Failed, Blocked]);
    }


    #[test]
    fn qa_refining_transitions() {
        use InternalStatus::*;
        let transitions = QaRefining.valid_transitions();
        assert_eq!(transitions, &[QaTesting]);
    }

    #[test]
    fn qa_testing_transitions() {
        use InternalStatus::*;
        let transitions = QaTesting.valid_transitions();
        assert_eq!(transitions, &[QaPassed, QaFailed]);
    }

    #[test]
    fn qa_passed_transitions() {
        use InternalStatus::*;
        let transitions = QaPassed.valid_transitions();
        assert_eq!(transitions, &[PendingReview]);
    }

    #[test]
    fn qa_failed_transitions() {
        use InternalStatus::*;
        let transitions = QaFailed.valid_transitions();
        assert_eq!(transitions, &[RevisionNeeded]);
    }

    #[test]
    fn pending_review_transitions() {
        use InternalStatus::*;
        let transitions = PendingReview.valid_transitions();
        assert_eq!(transitions, &[Reviewing]);
    }

    #[test]
    fn revision_needed_transitions() {
        use InternalStatus::*;
        let transitions = RevisionNeeded.valid_transitions();
        assert_eq!(transitions, &[ReExecuting, Cancelled]);
    }

    #[test]
    fn approved_transitions() {
        use InternalStatus::*;
        let transitions = Approved.valid_transitions();
        assert_eq!(transitions, &[Ready]); // Re-open
    }

    #[test]
    fn failed_transitions() {
        use InternalStatus::*;
        let transitions = Failed.valid_transitions();
        assert_eq!(transitions, &[Ready]); // Re-open
    }

    #[test]
    fn cancelled_transitions() {
        use InternalStatus::*;
        let transitions = Cancelled.valid_transitions();
        assert_eq!(transitions, &[Ready]); // Re-open
    }

    // ===== can_transition_to Tests =====

    #[test]
    fn can_transition_to_valid_returns_true() {
        use InternalStatus::*;

        // Backlog -> Ready is valid
        assert!(Backlog.can_transition_to(Ready));

        // Ready -> Executing is valid
        assert!(Ready.can_transition_to(Executing));

        // Executing -> QaRefining is valid
        assert!(Executing.can_transition_to(QaRefining));

        // Executing -> PendingReview is valid
        assert!(Executing.can_transition_to(PendingReview));

        // QaTesting -> QaPassed is valid
        assert!(QaTesting.can_transition_to(QaPassed));

        // PendingReview -> Reviewing is valid
        assert!(PendingReview.can_transition_to(Reviewing));

        // ReviewPassed -> Approved is valid
        assert!(ReviewPassed.can_transition_to(Approved));
    }

    #[test]
    fn can_transition_to_invalid_returns_false() {
        use InternalStatus::*;

        // Backlog -> Executing is NOT valid (must go through Ready first)
        assert!(!Backlog.can_transition_to(Executing));

        // Backlog -> Approved is NOT valid
        assert!(!Backlog.can_transition_to(Approved));

        // Executing -> Approved is NOT valid (must go through review)
        assert!(!Executing.can_transition_to(Approved));

        // QaPassed -> Approved is NOT valid (must go through review)
        assert!(!QaPassed.can_transition_to(Approved));

        // Approved -> Executing is NOT valid (must go through Ready)
        assert!(!Approved.can_transition_to(Executing));
    }

    #[test]
    fn self_transition_is_not_valid() {
        for status in InternalStatus::all_variants() {
            assert!(
                !status.can_transition_to(*status),
                "Self-transition should not be valid for {:?}",
                status
            );
        }
    }

    #[test]
    fn terminal_states_can_only_reopen() {
        use InternalStatus::*;

        // Terminal states can only go to Ready (re-open)
        for terminal in &[Approved, Failed, Cancelled] {
            assert!(terminal.can_transition_to(Ready));

            // But can't go anywhere else
            for target in InternalStatus::all_variants() {
                if *target != Ready {
                    assert!(
                        !terminal.can_transition_to(*target),
                        "{:?} should not transition to {:?}",
                        terminal,
                        target
                    );
                }
            }
        }
    }

    // ===== Happy Path Flow Tests =====

    #[test]
    fn happy_path_without_qa() {
        use InternalStatus::*;

        // Backlog -> Ready -> Executing -> PendingReview -> Reviewing -> ReviewPassed -> Approved
        assert!(Backlog.can_transition_to(Ready));
        assert!(Ready.can_transition_to(Executing));
        assert!(Executing.can_transition_to(PendingReview));
        assert!(PendingReview.can_transition_to(Reviewing));
        assert!(Reviewing.can_transition_to(ReviewPassed));
        assert!(ReviewPassed.can_transition_to(Approved));
    }

    #[test]
    fn happy_path_with_qa() {
        use InternalStatus::*;

        // Backlog -> Ready -> Executing -> QaRefining ->
        // QaTesting -> QaPassed -> PendingReview -> Reviewing -> ReviewPassed -> Approved
        assert!(Backlog.can_transition_to(Ready));
        assert!(Ready.can_transition_to(Executing));
        assert!(Executing.can_transition_to(QaRefining));
        assert!(QaRefining.can_transition_to(QaTesting));
        assert!(QaTesting.can_transition_to(QaPassed));
        assert!(QaPassed.can_transition_to(PendingReview));
        assert!(PendingReview.can_transition_to(Reviewing));
        assert!(Reviewing.can_transition_to(ReviewPassed));
        assert!(ReviewPassed.can_transition_to(Approved));
    }

    #[test]
    fn qa_failure_retry_path() {
        use InternalStatus::*;

        // QaTesting -> QaFailed -> RevisionNeeded -> ReExecuting
        assert!(QaTesting.can_transition_to(QaFailed));
        assert!(QaFailed.can_transition_to(RevisionNeeded));
        assert!(RevisionNeeded.can_transition_to(ReExecuting));
    }

    #[test]
    fn review_rejection_path() {
        use InternalStatus::*;

        // PendingReview -> Reviewing -> RevisionNeeded -> ReExecuting
        assert!(PendingReview.can_transition_to(Reviewing));
        assert!(Reviewing.can_transition_to(RevisionNeeded));
        assert!(RevisionNeeded.can_transition_to(ReExecuting));
    }

    // ===== Review State Transition Tests =====

    #[test]
    fn pending_review_to_reviewing() {
        use InternalStatus::*;
        assert!(PendingReview.can_transition_to(Reviewing));
    }

    #[test]
    fn reviewing_to_review_passed() {
        use InternalStatus::*;
        assert!(Reviewing.can_transition_to(ReviewPassed));
    }

    #[test]
    fn reviewing_to_revision_needed() {
        use InternalStatus::*;
        assert!(Reviewing.can_transition_to(RevisionNeeded));
    }

    #[test]
    fn review_passed_to_approved() {
        use InternalStatus::*;
        assert!(ReviewPassed.can_transition_to(Approved));
    }

    #[test]
    fn review_passed_to_revision_needed() {
        use InternalStatus::*;
        assert!(ReviewPassed.can_transition_to(RevisionNeeded));
    }

    // ===== Escalated State Transition Tests =====

    #[test]
    fn reviewing_to_escalated() {
        use InternalStatus::*;
        assert!(Reviewing.can_transition_to(Escalated));
    }

    #[test]
    fn escalated_transitions() {
        use InternalStatus::*;
        let transitions = Escalated.valid_transitions();
        assert_eq!(transitions, &[Approved, RevisionNeeded]);
    }

    #[test]
    fn escalated_to_approved() {
        use InternalStatus::*;
        assert!(Escalated.can_transition_to(Approved));
    }

    #[test]
    fn escalated_to_revision_needed() {
        use InternalStatus::*;
        assert!(Escalated.can_transition_to(RevisionNeeded));
    }

    #[test]
    fn revision_needed_to_re_executing() {
        use InternalStatus::*;
        assert!(RevisionNeeded.can_transition_to(ReExecuting));
    }

    #[test]
    fn re_executing_to_pending_review() {
        use InternalStatus::*;
        assert!(ReExecuting.can_transition_to(PendingReview));
    }

    #[test]
    fn blocking_and_unblocking_path() {
        use InternalStatus::*;

        // Ready -> Blocked -> Ready
        assert!(Ready.can_transition_to(Blocked));
        assert!(Blocked.can_transition_to(Ready));

        // Can also block during execution
        assert!(Executing.can_transition_to(Blocked));
    }

    // ===== Clone, Copy, Eq, Hash Tests =====

    #[test]
    fn clone_works() {
        let status = InternalStatus::Executing;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn copy_works() {
        let status = InternalStatus::Approved;
        let copied = status;
        assert_eq!(status, copied);
    }

    #[test]
    fn hash_works_in_hashset() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(InternalStatus::Ready);
        set.insert(InternalStatus::Executing);
        set.insert(InternalStatus::Ready); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&InternalStatus::Ready));
        assert!(set.contains(&InternalStatus::Executing));
        assert!(!set.contains(&InternalStatus::Approved));
    }

    #[test]
    fn debug_format_works() {
        let debug = format!("{:?}", InternalStatus::Executing);
        assert_eq!(debug, "Executing");
    }

    #[test]
    fn equality_works() {
        assert_eq!(InternalStatus::Ready, InternalStatus::Ready);
        assert_ne!(InternalStatus::Ready, InternalStatus::Executing);
    }
}
