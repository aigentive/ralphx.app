//! Unit tests for verification_handoff synthesis helpers and injection logic.

use std::sync::Arc;

use crate::application::reconciliation::verification_handoff::{
    derive_recommended_action, format_verification_result_xml, maybe_inject_verification_result_message,
    summarize_gaps, top_3_blockers, ReconcileChildCompleteResult, ESCALATED_TO_PARENT,
};
use crate::domain::entities::{IdeationSessionId, VerificationGap, VerificationMetadata, VerificationStatus};
use crate::domain::repositories::{ChatConversationRepository, ChatMessageRepository};
use crate::domain::services::MessageQueue;
use crate::infrastructure::memory::{MemoryChatConversationRepository, MemoryChatMessageRepository};

fn make_gap(severity: &str, description: &str) -> VerificationGap {
    VerificationGap {
        severity: severity.to_string(),
        category: "test".to_string(),
        description: description.to_string(),
        why_it_matters: None,
        source: None,
    }
}

fn make_meta(convergence_reason: Option<&str>, gaps: Vec<VerificationGap>) -> VerificationMetadata {
    VerificationMetadata {
        v: 1,
        current_round: 3,
        max_rounds: 5,
        rounds: vec![],
        current_gaps: gaps,
        convergence_reason: convergence_reason.map(str::to_string),
        best_round_index: None,
        parse_failures: vec![],
    }
}

// ---------------------------------------------------------------------------
// maybe_inject_verification_result_message — happy path (synthesizes message)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn needs_revision_max_rounds_synthesizes_message() {
    let parent_id = IdeationSessionId::new();
    let meta = make_meta(
        Some("max_rounds"),
        vec![
            make_gap("critical", "Auth bypass possible"),
            make_gap("high", "Missing input validation"),
        ],
    );
    let result = ReconcileChildCompleteResult {
        terminal_status: VerificationStatus::NeedsRevision,
        parsed_meta: Some(meta),
    };

    let conv_repo: Arc<dyn ChatConversationRepository> = Arc::new(MemoryChatConversationRepository::new());
    let msg_repo: Arc<dyn ChatMessageRepository> = Arc::new(MemoryChatMessageRepository::new());
    let queue = Arc::new(MessageQueue::new());

    maybe_inject_verification_result_message(&parent_id, &result, &conv_repo, &msg_repo, &queue)
        .await;

    // A message should have been stored
    let messages = msg_repo
        .get_by_session(&parent_id)
        .await
        .expect("repo should not error");
    assert_eq!(messages.len(), 1, "expected exactly one injected message");

    let content = &messages[0].content;
    assert!(content.contains("<verification-result>"), "should contain XML root tag");
    assert!(content.contains("<status>needs_revision</status>"), "should contain status");
    assert!(content.contains("<convergence_reason>max_rounds</convergence_reason>"), "should contain reason");
    assert!(content.contains("<recommended_next_action>revise_plan</recommended_next_action>"), "max_rounds → revise_plan");
    assert!(content.contains("critical"), "should include blocker severity");
}

// ---------------------------------------------------------------------------
// Dedup guard — escalated_to_parent skips synthesis
// ---------------------------------------------------------------------------

#[tokio::test]
async fn needs_revision_escalated_to_parent_skips_synthesis() {
    let parent_id = IdeationSessionId::new();
    let meta = make_meta(
        Some(ESCALATED_TO_PARENT),
        vec![make_gap("critical", "Already escalated gap")],
    );
    let result = ReconcileChildCompleteResult {
        terminal_status: VerificationStatus::NeedsRevision,
        parsed_meta: Some(meta),
    };

    let conv_repo: Arc<dyn ChatConversationRepository> = Arc::new(MemoryChatConversationRepository::new());
    let msg_repo: Arc<dyn ChatMessageRepository> = Arc::new(MemoryChatMessageRepository::new());
    let queue = Arc::new(MessageQueue::new());

    maybe_inject_verification_result_message(&parent_id, &result, &conv_repo, &msg_repo, &queue)
        .await;

    // No message should be stored — dedup guard fired
    let messages = msg_repo
        .get_by_session(&parent_id)
        .await
        .expect("repo should not error");
    assert!(messages.is_empty(), "dedup guard should prevent synthesis when escalated_to_parent");
}

// ---------------------------------------------------------------------------
// NeedsRevision with empty gaps (agent crash fallback)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn needs_revision_agent_crashed_empty_gaps_fallback() {
    let parent_id = IdeationSessionId::new();
    // Simulate agent crash: NeedsRevision with no current_gaps
    let meta = make_meta(Some("agent_crashed_mid_round"), vec![]);
    let result = ReconcileChildCompleteResult {
        terminal_status: VerificationStatus::NeedsRevision,
        parsed_meta: Some(meta),
    };

    let conv_repo: Arc<dyn ChatConversationRepository> = Arc::new(MemoryChatConversationRepository::new());
    let msg_repo: Arc<dyn ChatMessageRepository> = Arc::new(MemoryChatMessageRepository::new());
    let queue = Arc::new(MessageQueue::new());

    maybe_inject_verification_result_message(&parent_id, &result, &conv_repo, &msg_repo, &queue)
        .await;

    let messages = msg_repo
        .get_by_session(&parent_id)
        .await
        .expect("repo should not error");
    assert_eq!(messages.len(), 1, "should still synthesize with empty gaps");

    let content = &messages[0].content;
    // Empty gaps → crash fallback summary
    assert!(
        content.contains("Agent completed without producing gap analysis"),
        "should use crash fallback summary for empty gaps"
    );
    assert!(
        content.contains("<recommended_next_action>rerun_verification</recommended_next_action>"),
        "agent_crashed_mid_round → rerun_verification"
    );
}

// ---------------------------------------------------------------------------
// Non-triggering statuses — no synthesis for Verified
// ---------------------------------------------------------------------------

#[tokio::test]
async fn verified_completion_no_synthesis() {
    let parent_id = IdeationSessionId::new();
    let meta = make_meta(Some("zero_blocking"), vec![]);
    let result = ReconcileChildCompleteResult {
        terminal_status: VerificationStatus::Verified,
        parsed_meta: Some(meta),
    };

    let conv_repo: Arc<dyn ChatConversationRepository> = Arc::new(MemoryChatConversationRepository::new());
    let msg_repo: Arc<dyn ChatMessageRepository> = Arc::new(MemoryChatMessageRepository::new());
    let queue = Arc::new(MessageQueue::new());

    maybe_inject_verification_result_message(&parent_id, &result, &conv_repo, &msg_repo, &queue)
        .await;

    let messages = msg_repo
        .get_by_session(&parent_id)
        .await
        .expect("repo should not error");
    assert!(messages.is_empty(), "Verified status should not synthesize any message");
}

// ---------------------------------------------------------------------------
// Non-triggering statuses — no synthesis for Unverified
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unverified_completion_no_synthesis() {
    let parent_id = IdeationSessionId::new();
    let result = ReconcileChildCompleteResult {
        terminal_status: VerificationStatus::Unverified,
        parsed_meta: None,
    };

    let conv_repo: Arc<dyn ChatConversationRepository> = Arc::new(MemoryChatConversationRepository::new());
    let msg_repo: Arc<dyn ChatMessageRepository> = Arc::new(MemoryChatMessageRepository::new());
    let queue = Arc::new(MessageQueue::new());

    maybe_inject_verification_result_message(&parent_id, &result, &conv_repo, &msg_repo, &queue)
        .await;

    let messages = msg_repo
        .get_by_session(&parent_id)
        .await
        .expect("repo should not error");
    assert!(messages.is_empty(), "Unverified status should not synthesize any message");
}

// ---------------------------------------------------------------------------
// Unit tests for synthesis helpers
// ---------------------------------------------------------------------------

#[test]
fn summarize_gaps_empty_returns_crash_text() {
    let summary = summarize_gaps(&[]);
    assert!(summary.contains("Agent completed without producing gap analysis"));
}

#[test]
fn summarize_gaps_counts_by_severity() {
    let gaps = vec![
        make_gap("critical", "a"),
        make_gap("critical", "b"),
        make_gap("high", "c"),
        make_gap("medium", "d"),
    ];
    let summary = summarize_gaps(&gaps);
    assert!(summary.contains("4 gap(s)"), "should show total count");
    assert!(summary.contains("2 critical"), "should count critical");
    assert!(summary.contains("1 high"), "should count high");
    assert!(summary.contains("1 medium"), "should count medium");
}

#[test]
fn top_3_blockers_returns_at_most_3_sorted_by_severity() {
    let gaps = vec![
        make_gap("low", "low prio"),
        make_gap("medium", "medium prio"),
        make_gap("high", "high prio"),
        make_gap("critical", "critical prio"),
        make_gap("critical", "another critical"),
    ];
    let blockers = top_3_blockers(&gaps);
    assert_eq!(blockers.len(), 3, "should cap at 3");
    assert_eq!(blockers[0].0, "critical");
    assert_eq!(blockers[1].0, "critical");
    assert_eq!(blockers[2].0, "high");
}

#[test]
fn top_3_blockers_caps_description_at_200_chars() {
    let long_desc = "x".repeat(300);
    let gaps = vec![make_gap("critical", &long_desc)];
    let blockers = top_3_blockers(&gaps);
    assert_eq!(blockers.len(), 1);
    // Should be <= 200 chars of content + ellipsis char
    assert!(
        blockers[0].1.chars().count() <= 201,
        "description should be capped at 200 chars + ellipsis"
    );
    assert!(blockers[0].1.ends_with('…'), "truncated description should end with ellipsis");
}

#[test]
fn derive_recommended_action_maps_correctly() {
    assert_eq!(derive_recommended_action(Some("max_rounds")), "revise_plan");
    assert_eq!(derive_recommended_action(Some("jaccard_converged")), "revise_plan");
    assert_eq!(derive_recommended_action(Some("gap_score_plateau")), "explore_code_paths");
    assert_eq!(derive_recommended_action(Some("agent_crashed_mid_round")), "rerun_verification");
    assert_eq!(derive_recommended_action(Some("agent_completed_without_update")), "rerun_verification");
    assert_eq!(derive_recommended_action(None), "rerun_verification");
    assert_eq!(derive_recommended_action(Some("unknown_future_reason")), "rerun_verification");
}

#[test]
fn format_verification_result_xml_structure() {
    let gaps = vec![
        make_gap("critical", "Auth bypass"),
        make_gap("high", "SQL injection risk"),
    ];
    let xml = format_verification_result_xml(
        "child-session-123",
        Some("max_rounds"),
        3,
        5,
        &gaps,
    );

    assert!(xml.starts_with("<verification-result>"), "should open with root tag");
    assert!(xml.ends_with("</verification-result>"), "should close with root tag");
    assert!(xml.contains("<child_session_id>child-session-123</child_session_id>"));
    assert!(xml.contains("<status>needs_revision</status>"));
    assert!(xml.contains("<convergence_reason>max_rounds</convergence_reason>"));
    assert!(xml.contains("<round>3</round>"));
    assert!(xml.contains("<max_rounds>5</max_rounds>"));
    assert!(xml.contains("<top_blockers>"));
    assert!(xml.contains("severity=\"critical\""));
    assert!(xml.contains("<recommended_next_action>revise_plan</recommended_next_action>"));
}

#[test]
fn format_verification_result_xml_no_blockers_section_when_empty() {
    let xml = format_verification_result_xml("id", None, 1, 3, &[]);
    assert!(!xml.contains("<top_blockers>"), "should omit top_blockers when no gaps");
    assert!(xml.contains("<recommended_next_action>"), "should still have action");
}
