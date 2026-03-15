use chrono::{Duration, Utc};
use serde_json::{Map, Value, json};

/// Replicates the staleness check logic from `recover_timeout_failures`.
///
/// Returns `true` if `failed_at` is present, parseable, and older than `threshold_secs`.
/// Returns `false` when `failed_at` is absent (non-stale: pre-existing tasks get one attempt).
fn is_task_stale(metadata: &Map<String, Value>, threshold_secs: u64) -> bool {
    if let Some(failed_at_str) = metadata.get("failed_at").and_then(|v| v.as_str()) {
        if let Ok(failed_at) = chrono::DateTime::parse_from_rfc3339(failed_at_str) {
            let age_secs = (Utc::now() - failed_at.with_timezone(&Utc)).num_seconds();
            return age_secs > threshold_secs as i64;
        }
    }
    false // absent failed_at = non-stale
}

#[test]
fn test_staleness_check_stale_task() {
    let mut metadata = Map::new();
    // 2 days ago — well beyond the 86400s (1 day) threshold
    let two_days_ago = Utc::now() - Duration::seconds(172_800);
    metadata.insert("failed_at".to_string(), json!(two_days_ago.to_rfc3339()));

    assert!(
        is_task_stale(&metadata, 86400),
        "Task with failed_at 2 days ago should be stale"
    );
}

#[test]
fn test_staleness_check_non_stale_no_failed_at() {
    let metadata = Map::new(); // empty — no failed_at key

    assert!(
        !is_task_stale(&metadata, 86400),
        "Task with no failed_at should be non-stale"
    );
}

#[test]
fn test_staleness_check_recent_task_not_stale() {
    let mut metadata = Map::new();
    // 1 hour ago — well within the 86400s (1 day) threshold
    let one_hour_ago = Utc::now() - Duration::seconds(3600);
    metadata.insert("failed_at".to_string(), json!(one_hour_ago.to_rfc3339()));

    assert!(
        !is_task_stale(&metadata, 86400),
        "Task failed 1 hour ago should not be stale"
    );
}

// ============================================================
// Tests for is_permanent_git_error() classifier
// ============================================================

/// Replicates is_permanent_git_error() from execution.rs for testing.
/// This mirrors the production function to verify classification behavior.
fn is_permanent_git_error_test(msg: &str) -> bool {
    msg.contains("invalid reference")
        || msg.contains("not a valid object name")
        || msg.contains("does not point to a valid object")
        || msg.contains("no longer exists")
}

#[test]
fn test_permanent_git_error_invalid_reference() {
    // Git says the branch ref is invalid (deleted branch)
    let msg = "Git isolation failed: invalid reference 'refs/heads/ralphx/task-abc'";
    assert!(
        is_permanent_git_error_test(msg),
        "Should detect 'invalid reference' as permanent git error"
    );
}

#[test]
fn test_permanent_git_error_not_valid_object() {
    let msg = "fatal: not a valid object name: 'ralphx/task-abc'";
    assert!(
        is_permanent_git_error_test(msg),
        "Should detect 'not a valid object name' as permanent git error"
    );
}

#[test]
fn test_permanent_git_error_does_not_point_to_valid_object() {
    let msg = "error: refs/heads/ralphx/task-abc does not point to a valid object";
    assert!(
        is_permanent_git_error_test(msg),
        "Should detect 'does not point to a valid object' as permanent git error"
    );
}

#[test]
fn test_permanent_git_error_branch_no_longer_exists() {
    // Matches the error from Fix 4 (branch_exists check in on_enter_states.rs)
    let msg = "branch 'ralphx/task-abc' no longer exists (deleted during prior merge cleanup)";
    assert!(
        is_permanent_git_error_test(msg),
        "Should detect 'no longer exists' as permanent git error"
    );
}

#[test]
fn test_permanent_git_error_transient_not_matched() {
    // Transient errors should NOT be classified as permanent
    let transient_errors = [
        "fatal: Unable to create '.git/index.lock': File exists",
        "error: timeout waiting for git",
        "network error: connection refused",
        "error: unable to acquire lock on git index",
        "fatal: Out of memory, malloc failed",
    ];
    for msg in &transient_errors {
        assert!(
            !is_permanent_git_error_test(msg),
            "Should NOT classify transient error as permanent: {}",
            msg
        );
    }
}

#[test]
fn test_permanent_git_error_empty_message_not_permanent() {
    assert!(
        !is_permanent_git_error_test(""),
        "Empty message should not be classified as permanent git error"
    );
}
