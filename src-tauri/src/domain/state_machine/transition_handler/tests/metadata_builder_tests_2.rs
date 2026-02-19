// Tests extracted from metadata_builder.rs #[cfg(test)] mod tests — part 2 of 2
//
// Covers: ResumeCategory, categorize_resume_state, get_resume_target

use super::super::metadata_builder::*;
use crate::domain::entities::status::InternalStatus;

// ===== ResumeCategory Tests =====

#[test]
fn test_resume_category_direct_for_executing() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Executing),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_direct_for_re_executing() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::ReExecuting),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_direct_for_reviewing() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Reviewing),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_direct_for_qa_refining() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::QaRefining),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_direct_for_qa_testing() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::QaTesting),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_validated_for_merging() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Merging),
        ResumeCategory::Validated
    );
}

#[test]
fn test_resume_category_validated_for_pending_merge() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::PendingMerge),
        ResumeCategory::Validated
    );
}

#[test]
fn test_resume_category_validated_for_merge_conflict() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::MergeConflict),
        ResumeCategory::Validated
    );
}

#[test]
fn test_resume_category_validated_for_merge_incomplete() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::MergeIncomplete),
        ResumeCategory::Validated
    );
}

#[test]
fn test_resume_category_redirect_for_qa_passed() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::QaPassed),
        ResumeCategory::Redirect
    );
}

#[test]
fn test_resume_category_redirect_for_revision_needed() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::RevisionNeeded),
        ResumeCategory::Redirect
    );
}

#[test]
fn test_resume_category_redirect_for_pending_review() {
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::PendingReview),
        ResumeCategory::Redirect
    );
}

#[test]
fn test_resume_category_fallback_to_direct_for_other_states() {
    // States that shouldn't be stopped from, but should fall back to Direct
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Ready),
        ResumeCategory::Direct
    );
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Backlog),
        ResumeCategory::Direct
    );
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Blocked),
        ResumeCategory::Direct
    );
    assert_eq!(
        ResumeCategory::from_status(InternalStatus::Approved),
        ResumeCategory::Direct
    );
}

#[test]
fn test_resume_category_serialization() {
    let direct = serde_json::to_string(&ResumeCategory::Direct).unwrap();
    assert_eq!(direct, "\"direct\"");

    let validated = serde_json::to_string(&ResumeCategory::Validated).unwrap();
    assert_eq!(validated, "\"validated\"");

    let redirect = serde_json::to_string(&ResumeCategory::Redirect).unwrap();
    assert_eq!(redirect, "\"redirect\"");
}

#[test]
fn test_resume_category_deserialization() {
    let direct: ResumeCategory = serde_json::from_str("\"direct\"").unwrap();
    assert_eq!(direct, ResumeCategory::Direct);

    let validated: ResumeCategory = serde_json::from_str("\"validated\"").unwrap();
    assert_eq!(validated, ResumeCategory::Validated);

    let redirect: ResumeCategory = serde_json::from_str("\"redirect\"").unwrap();
    assert_eq!(redirect, ResumeCategory::Redirect);
}

// ===== categorize_resume_state Tests =====

#[test]
fn test_categorize_resume_state_matches_from_status() {
    // Test that the function delegates correctly
    for status in InternalStatus::all_variants() {
        assert_eq!(
            categorize_resume_state(*status),
            ResumeCategory::from_status(*status),
            "Mismatch for {:?}",
            status
        );
    }
}

// ===== get_resume_target Tests =====

#[test]
fn test_get_resume_target_qa_passed_to_pending_review() {
    assert_eq!(
        get_resume_target(InternalStatus::QaPassed),
        InternalStatus::PendingReview
    );
}

#[test]
fn test_get_resume_target_revision_needed_to_re_executing() {
    assert_eq!(
        get_resume_target(InternalStatus::RevisionNeeded),
        InternalStatus::ReExecuting
    );
}

#[test]
fn test_get_resume_target_pending_review_to_reviewing() {
    assert_eq!(
        get_resume_target(InternalStatus::PendingReview),
        InternalStatus::Reviewing
    );
}

#[test]
fn test_get_resume_target_direct_states_return_same() {
    // Direct states should return the same state
    assert_eq!(
        get_resume_target(InternalStatus::Executing),
        InternalStatus::Executing
    );
    assert_eq!(
        get_resume_target(InternalStatus::ReExecuting),
        InternalStatus::ReExecuting
    );
    assert_eq!(
        get_resume_target(InternalStatus::Reviewing),
        InternalStatus::Reviewing
    );
    assert_eq!(
        get_resume_target(InternalStatus::QaRefining),
        InternalStatus::QaRefining
    );
    assert_eq!(
        get_resume_target(InternalStatus::QaTesting),
        InternalStatus::QaTesting
    );
}

#[test]
fn test_get_resume_target_validated_states_return_same() {
    // Validated states should return the same state
    assert_eq!(
        get_resume_target(InternalStatus::Merging),
        InternalStatus::Merging
    );
    assert_eq!(
        get_resume_target(InternalStatus::PendingMerge),
        InternalStatus::PendingMerge
    );
    assert_eq!(
        get_resume_target(InternalStatus::MergeConflict),
        InternalStatus::MergeConflict
    );
    assert_eq!(
        get_resume_target(InternalStatus::MergeIncomplete),
        InternalStatus::MergeIncomplete
    );
}

#[test]
fn test_get_resume_target_other_states_return_same() {
    // All other states should return the same state
    assert_eq!(
        get_resume_target(InternalStatus::Ready),
        InternalStatus::Ready
    );
    assert_eq!(
        get_resume_target(InternalStatus::Backlog),
        InternalStatus::Backlog
    );
    assert_eq!(
        get_resume_target(InternalStatus::Merged),
        InternalStatus::Merged
    );
}

#[test]
fn test_get_resume_target_all_states_have_consistent_behavior() {
    // For redirect states, the target should be different from source
    for status in &[
        InternalStatus::QaPassed,
        InternalStatus::RevisionNeeded,
        InternalStatus::PendingReview,
    ] {
        let target = get_resume_target(*status);
        assert_ne!(
            target, *status,
            "Redirect state {:?} should map to a different target",
            status
        );
        assert_eq!(
            categorize_resume_state(*status),
            ResumeCategory::Redirect,
            "State {:?} should be categorized as Redirect",
            status
        );
    }

    // For direct states, the target should be the same as source
    for status in &[
        InternalStatus::Executing,
        InternalStatus::ReExecuting,
        InternalStatus::Reviewing,
        InternalStatus::QaRefining,
        InternalStatus::QaTesting,
    ] {
        let target = get_resume_target(*status);
        assert_eq!(
            target, *status,
            "Direct state {:?} should map to itself",
            status
        );
    }

    // For validated states, the target should be the same as source
    for status in &[
        InternalStatus::Merging,
        InternalStatus::PendingMerge,
        InternalStatus::MergeConflict,
        InternalStatus::MergeIncomplete,
    ] {
        let target = get_resume_target(*status);
        assert_eq!(
            target, *status,
            "Validated state {:?} should map to itself",
            status
        );
    }
}
