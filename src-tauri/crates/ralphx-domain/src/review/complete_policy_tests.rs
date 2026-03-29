use super::*;

#[test]
fn test_parse_review_decision_accepts_all_known_values() {
    assert_eq!(
        parse_review_decision("approved").unwrap(),
        ReviewToolOutcome::Approved
    );
    assert_eq!(
        parse_review_decision("approved_no_changes").unwrap(),
        ReviewToolOutcome::ApprovedNoChanges
    );
    assert_eq!(
        parse_review_decision("needs_changes").unwrap(),
        ReviewToolOutcome::NeedsChanges
    );
    assert_eq!(
        parse_review_decision("escalate").unwrap(),
        ReviewToolOutcome::Escalate
    );
}

#[test]
fn test_parse_review_decision_rejects_unknown_value() {
    let err = parse_review_decision("nope").unwrap_err();
    assert!(err.to_string().contains("Invalid decision"));
}

#[test]
fn test_validate_complete_review_policy_requires_scope_classification_for_expansion() {
    let err = validate_complete_review_policy(
        ScopeDriftStatus::ScopeExpansion,
        &["ralphx.yaml".to_string()],
        None,
        ReviewToolOutcome::NeedsChanges,
        0,
        &ReviewSettings::default(),
        1,
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("Scope drift classification required"));
}

#[test]
fn test_validate_complete_review_policy_rejects_approving_unrelated_drift() {
    let err = validate_complete_review_policy(
        ScopeDriftStatus::ScopeExpansion,
        &["ralphx.yaml".to_string()],
        Some(ScopeDriftClassification::UnrelatedDrift),
        ReviewToolOutcome::Approved,
        0,
        &ReviewSettings::default(),
        0,
    )
    .unwrap_err();
    assert!(err.to_string().contains("Cannot approve task"));
}

#[test]
fn test_validate_complete_review_policy_requires_revision_exhaustion_before_escalation() {
    let err = validate_complete_review_policy(
        ScopeDriftStatus::ScopeExpansion,
        &["ralphx.yaml".to_string()],
        Some(ScopeDriftClassification::UnrelatedDrift),
        ReviewToolOutcome::Escalate,
        2,
        &ReviewSettings::default(),
        0,
    )
    .unwrap_err();
    assert!(err.to_string().contains("revision budget remains"));
}

#[test]
fn test_validate_complete_review_policy_requires_issues_for_unrelated_drift_revise() {
    let err = validate_complete_review_policy(
        ScopeDriftStatus::ScopeExpansion,
        &["ralphx.yaml".to_string()],
        Some(ScopeDriftClassification::UnrelatedDrift),
        ReviewToolOutcome::NeedsChanges,
        0,
        &ReviewSettings::default(),
        0,
    )
    .unwrap_err();
    assert!(err.to_string().contains("structured issue"));
}

#[test]
fn test_review_outcome_for_tool_maps_expected_values() {
    assert_eq!(
        review_outcome_for_tool(ReviewToolOutcome::ApprovedNoChanges),
        ReviewOutcome::ApprovedNoChanges
    );
    assert_eq!(
        review_outcome_for_tool(ReviewToolOutcome::NeedsChanges),
        ReviewOutcome::ChangesRequested
    );
}
