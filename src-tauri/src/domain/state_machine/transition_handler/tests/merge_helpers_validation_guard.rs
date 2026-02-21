// Tests for has_prior_validation_failure() in merge_helpers.rs
//
// Verifies that the guard correctly detects prior validation failures
// from three independent metadata fields: merge_commit_unrevertable,
// merge_failure_source, and validation_revert_count.

use super::helpers::*;
use super::super::merge_helpers::has_prior_validation_failure;

#[test]
fn returns_false_for_no_metadata() {
    let task = make_task(None, None);
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_false_for_empty_metadata() {
    let mut task = make_task(None, None);
    task.metadata = Some("{}".to_string());
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_false_for_unrelated_metadata() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"some_key": "value"}"#.to_string());
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_true_for_unrevertable_flag() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"merge_commit_unrevertable": true}"#.to_string());
    assert!(has_prior_validation_failure(&task));
}

#[test]
fn returns_false_for_unrevertable_false() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"merge_commit_unrevertable": false}"#.to_string());
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_true_for_validation_failed_source() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"merge_failure_source": "validation_failed"}"#.to_string());
    assert!(has_prior_validation_failure(&task));
}

#[test]
fn returns_false_for_other_failure_source() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"merge_failure_source": "agent_reported"}"#.to_string());
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_true_for_revert_count_positive() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"validation_revert_count": 1}"#.to_string());
    assert!(has_prior_validation_failure(&task));
}

#[test]
fn returns_false_for_revert_count_zero() {
    let mut task = make_task(None, None);
    task.metadata = Some(r#"{"validation_revert_count": 0}"#.to_string());
    assert!(!has_prior_validation_failure(&task));
}

#[test]
fn returns_true_for_multiple_flags() {
    let mut task = make_task(None, None);
    task.metadata = Some(
        serde_json::json!({
            "merge_commit_unrevertable": true,
            "merge_failure_source": "validation_failed",
            "validation_revert_count": 2
        })
        .to_string(),
    );
    assert!(has_prior_validation_failure(&task));
}
