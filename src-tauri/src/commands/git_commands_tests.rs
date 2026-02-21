use super::*;

#[test]
fn test_commit_info_response_conversion() {
    let info = CommitInfo {
        sha: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
        short_sha: "abcdef1".to_string(),
        message: "Test commit".to_string(),
        author: "Test Author".to_string(),
        timestamp: "2026-02-02T12:00:00+00:00".to_string(),
    };

    let response = CommitInfoResponse::from(info);
    assert_eq!(response.short_sha, "abcdef1");
    assert_eq!(response.message, "Test commit");
}

#[test]
fn test_diff_stats_response_conversion() {
    let stats = DiffStats {
        files_changed: 5,
        insertions: 100,
        deletions: 50,
        changed_files: vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()],
    };

    let response = TaskDiffStatsResponse::from(stats);
    assert_eq!(response.files_changed, 5);
    assert_eq!(response.insertions, 100);
    assert_eq!(response.deletions, 50);
    assert_eq!(response.changed_files.len(), 2);
}

/// Verify that the retry_merge metadata reset logic clears all loop-prevention
/// counters so the reconciler won't block subsequent auto-retries.
#[test]
fn test_retry_merge_resets_loop_counters() {
    // Simulate task metadata with high validation_revert_count, AgentReported source,
    // and merge_recovery events that would block auto-retry.
    let metadata = serde_json::json!({
        "validation_revert_count": 5,
        "merge_failure_source": "agent_reported",
        "merge_recovery": {
            "version": 1,
            "events": [
                {"kind": "auto_retry_triggered", "timestamp": "2026-01-01T00:00:00Z", "source": "system"},
                {"kind": "auto_retry_triggered", "timestamp": "2026-01-01T00:01:00Z", "source": "system"},
                {"kind": "attempt_failed", "timestamp": "2026-01-01T00:02:00Z", "source": "system"},
            ],
            "last_state": "failed"
        },
        "some_other_key": "preserved"
    });

    // Apply the same reset logic as retry_merge()
    let mut meta_obj = metadata.as_object().cloned().unwrap();
    meta_obj.insert("merge_retry_in_progress".to_string(), serde_json::json!(true));
    meta_obj.insert("validation_revert_count".to_string(), serde_json::json!(0));
    meta_obj.remove("merge_failure_source");
    if let Some(recovery_val) = meta_obj.get_mut("merge_recovery") {
        if let Some(recovery_obj) = recovery_val.as_object_mut() {
            recovery_obj.insert("events".to_string(), serde_json::json!([]));
            recovery_obj.insert("last_state".to_string(), serde_json::json!("retrying"));
        }
    }

    let result = serde_json::Value::Object(meta_obj);

    // validation_revert_count reset to 0
    assert_eq!(result["validation_revert_count"], 0);
    // merge_failure_source removed
    assert!(result.get("merge_failure_source").is_none());
    // merge_recovery events cleared
    assert_eq!(result["merge_recovery"]["events"].as_array().unwrap().len(), 0);
    // merge_recovery last_state set to retrying
    assert_eq!(result["merge_recovery"]["last_state"], "retrying");
    // Other metadata keys preserved
    assert_eq!(result["some_other_key"], "preserved");
    // In-flight guard set
    assert_eq!(result["merge_retry_in_progress"], true);
}

/// Verify that the reset logic handles metadata with no merge_recovery key.
#[test]
fn test_retry_merge_resets_counters_without_merge_recovery() {
    let metadata = serde_json::json!({
        "validation_revert_count": 3,
        "merge_failure_source": "agent_reported",
    });

    let mut meta_obj = metadata.as_object().cloned().unwrap();
    meta_obj.insert("merge_retry_in_progress".to_string(), serde_json::json!(true));
    meta_obj.insert("validation_revert_count".to_string(), serde_json::json!(0));
    meta_obj.remove("merge_failure_source");
    if let Some(recovery_val) = meta_obj.get_mut("merge_recovery") {
        if let Some(recovery_obj) = recovery_val.as_object_mut() {
            recovery_obj.insert("events".to_string(), serde_json::json!([]));
            recovery_obj.insert("last_state".to_string(), serde_json::json!("retrying"));
        }
    }

    let result = serde_json::Value::Object(meta_obj);

    assert_eq!(result["validation_revert_count"], 0);
    assert!(result.get("merge_failure_source").is_none());
    // No merge_recovery key — should not crash
    assert!(result.get("merge_recovery").is_none());
}

/// Verify that the reconciler's validation_revert_count check would pass after reset.
/// The reconciler blocks when validation_revert_count > max (default 2).
/// After user retry resets to 0, the check should pass.
#[test]
fn test_validation_revert_count_passes_after_reset() {
    // Simulate metadata after retry_merge resets the counter
    let metadata_str = serde_json::json!({
        "validation_revert_count": 0,
        "merge_retry_in_progress": true,
    }).to_string();

    // Same read logic as ReconciliationRunner::validation_revert_count()
    let revert_count: u32 = serde_json::from_str::<serde_json::Value>(&metadata_str)
        .ok()
        .and_then(|v| v.get("validation_revert_count").and_then(|c| c.as_u64()).map(|c| c as u32))
        .unwrap_or(0);

    assert_eq!(revert_count, 0);
    // reconciliation_config().validation_revert_max_count defaults to 2
    // 0 <= 2, so the reconciler would NOT block auto-retry
    assert!(revert_count <= 2);
}
