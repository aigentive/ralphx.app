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
