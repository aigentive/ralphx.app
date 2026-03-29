use super::*;
use crate::entities::{ProjectId, ReviewerType, TaskId};

#[test]
fn test_apply_review_outcome_updates_review_and_returns_history_outcome() {
    let mut review = Review::new(ProjectId::new(), TaskId::new(), ReviewerType::Ai);
    let outcome = apply_review_outcome(
        &mut review,
        ReviewToolOutcome::NeedsChanges,
        Some("Fix this".to_string()),
    );

    assert_eq!(outcome, ReviewOutcome::ChangesRequested);
    assert_eq!(review.status.to_string(), "changes_requested");
    assert_eq!(review.notes.as_deref(), Some("Fix this"));
}

#[test]
fn test_review_note_content_prefers_escalation_reason_for_escalate() {
    let content = review_note_content(
        ReviewToolOutcome::Escalate,
        Some("generic feedback"),
        Some("specific reason"),
    );
    assert_eq!(content.as_deref(), Some("specific reason"));
}

#[test]
fn test_review_note_content_uses_feedback_for_non_escalation() {
    let content = review_note_content(
        ReviewToolOutcome::Approved,
        Some("looks good"),
        Some("ignored"),
    );
    assert_eq!(content.as_deref(), Some("looks good"));
}

#[test]
fn test_complete_review_response_message_mentions_followup() {
    assert_eq!(
        complete_review_response_message(Some("session-123")),
        "Review submitted successfully. Follow-up ideation session created: session-123"
    );
}

#[test]
fn test_target_status_helpers_map_expected_states() {
    assert_eq!(approved_target_status(true), InternalStatus::ReviewPassed);
    assert_eq!(approved_target_status(false), InternalStatus::Approved);
    assert_eq!(
        approved_no_changes_target_status(true),
        InternalStatus::ReviewPassed
    );
    assert_eq!(
        approved_no_changes_target_status(false),
        InternalStatus::Merged
    );
}
