// Tests extracted from metadata_builder.rs #[cfg(test)] mod tests — part 1 of 2
//
// Covers: MetadataUpdate, build_failed_metadata, build_trigger_origin_metadata,
//         key_exists_in, StopMetadata, build_stop_metadata, parse_stop_metadata,
//         build_restart_metadata, clear_stop_metadata, with_null

use super::super::metadata_builder::*;
use crate::domain::entities::status::InternalStatus;
use crate::domain::state_machine::types::FailedData;
use serde_json::{Map, Value};

#[test]
fn test_merge_into_with_none_creates_new_object() {
    let update = MetadataUpdate::new().with_string("key1", "value1");
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("key1").unwrap(),
        &Value::String("value1".to_string())
    );
}

#[test]
fn test_merge_into_preserves_existing_keys() {
    let existing = r#"{"existing_key":"existing_value"}"#;
    let update = MetadataUpdate::new().with_string("new_key", "new_value");
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("existing_key").unwrap(),
        &Value::String("existing_value".to_string())
    );
    assert_eq!(
        parsed.get("new_key").unwrap(),
        &Value::String("new_value".to_string())
    );
}

#[test]
fn test_merge_into_overwrites_duplicate_keys() {
    let existing = r#"{"key1":"old_value"}"#;
    let update = MetadataUpdate::new().with_string("key1", "new_value");
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("key1").unwrap(),
        &Value::String("new_value".to_string())
    );
}

#[test]
fn test_build_failed_metadata_produces_correct_keys() {
    let data = FailedData {
        error: "Test error".to_string(),
        details: Some("Test details".to_string()),
        is_timeout: true,
        notified: false,
        attempt_count: 0,
    };

    let update = build_failed_metadata(&data);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("failure_error").unwrap(),
        &Value::String("Test error".to_string())
    );
    assert_eq!(
        parsed.get("failure_details").unwrap(),
        &Value::String("Test details".to_string())
    );
    assert_eq!(parsed.get("is_timeout").unwrap(), &Value::Bool(true));
}

#[test]
fn test_build_failed_metadata_without_details() {
    let data = FailedData {
        error: "Test error".to_string(),
        details: None,
        is_timeout: false,
        notified: false,
        attempt_count: 0,
    };

    let update = build_failed_metadata(&data);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("failure_error").unwrap(),
        &Value::String("Test error".to_string())
    );
    assert!(parsed.get("failure_details").is_none());
    assert_eq!(parsed.get("is_timeout").unwrap(), &Value::Bool(false));
}

#[test]
fn test_build_trigger_origin_metadata_produces_trigger_origin() {
    let update = build_trigger_origin_metadata("qa");
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("trigger_origin").unwrap(),
        &Value::String("qa".to_string())
    );
}

#[test]
fn test_key_exists_in_returns_true_when_key_present() {
    let metadata = r#"{"failure_error":"some error"}"#;
    assert!(MetadataUpdate::key_exists_in(
        "failure_error",
        Some(metadata)
    ));
}

#[test]
fn test_key_exists_in_returns_false_when_key_absent() {
    let metadata = r#"{"other_key":"value"}"#;
    assert!(!MetadataUpdate::key_exists_in(
        "failure_error",
        Some(metadata)
    ));
}

#[test]
fn test_key_exists_in_returns_false_when_metadata_none() {
    assert!(!MetadataUpdate::key_exists_in("failure_error", None));
}

#[test]
fn test_key_exists_in_returns_false_when_metadata_invalid_json() {
    let invalid = "not valid json";
    assert!(!MetadataUpdate::key_exists_in(
        "failure_error",
        Some(invalid)
    ));
}

#[test]
fn test_merge_into_handles_invalid_existing_json() {
    let invalid = "not valid json";
    let update = MetadataUpdate::new().with_string("key1", "value1");
    let result = update.merge_into(Some(invalid));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("key1").unwrap(),
        &Value::String("value1".to_string())
    );
    // Should start fresh when existing is invalid
    assert_eq!(parsed.len(), 1);
}

// ===== StopMetadata Tests =====

#[test]
fn test_stop_metadata_new_with_reason() {
    let stop = StopMetadata::new(InternalStatus::Merging, Some("User stopped".to_string()));
    assert_eq!(stop.stopped_from_status, "merging");
    assert_eq!(stop.stop_reason, Some("User stopped".to_string()));
    assert!(!stop.stopped_at.is_empty());
}

#[test]
fn test_stop_metadata_new_without_reason() {
    let stop = StopMetadata::new(InternalStatus::Executing, None);
    assert_eq!(stop.stopped_from_status, "executing");
    assert_eq!(stop.stop_reason, None);
    assert!(!stop.stopped_at.is_empty());
}

#[test]
fn test_stop_metadata_parse_from_status_valid() {
    let stop = StopMetadata::new(InternalStatus::PendingMerge, None);
    let parsed = stop.parse_from_status();
    assert_eq!(parsed, Some(InternalStatus::PendingMerge));
}

#[test]
fn test_stop_metadata_parse_from_status_invalid() {
    let stop = StopMetadata {
        stopped_from_status: "invalid_status".to_string(),
        stop_reason: None,
        stopped_at: "2026-02-15T10:00:00Z".to_string(),
    };
    assert_eq!(stop.parse_from_status(), None);
}

#[test]
fn test_stop_metadata_parse_stopped_at_valid() {
    let stop = StopMetadata {
        stopped_from_status: "merging".to_string(),
        stop_reason: None,
        stopped_at: "2026-02-15T10:30:00+00:00".to_string(),
    };
    let parsed = stop.parse_stopped_at();
    assert!(parsed.is_some());
}

#[test]
fn test_stop_metadata_parse_stopped_at_invalid() {
    let stop = StopMetadata {
        stopped_from_status: "merging".to_string(),
        stop_reason: None,
        stopped_at: "not-a-timestamp".to_string(),
    };
    assert_eq!(stop.parse_stopped_at(), None);
}

#[test]
fn test_stop_metadata_serialization() {
    let stop = StopMetadata {
        stopped_from_status: "merging".to_string(),
        stop_reason: Some("User stopped to protect main branch".to_string()),
        stopped_at: "2026-02-15T10:30:00+00:00".to_string(),
    };
    let json = serde_json::to_string(&stop).unwrap();
    assert!(json.contains("merging"));
    assert!(json.contains("User stopped"));
    assert!(json.contains("2026-02-15T10:30:00+00:00"));
}

#[test]
fn test_stop_metadata_deserialization() {
    let json = r#"{"stopped_from_status":"reviewing","stop_reason":"Test reason","stopped_at":"2026-02-15T10:30:00Z"}"#;
    let stop: StopMetadata = serde_json::from_str(json).unwrap();
    assert_eq!(stop.stopped_from_status, "reviewing");
    assert_eq!(stop.stop_reason, Some("Test reason".to_string()));
    assert_eq!(stop.stopped_at, "2026-02-15T10:30:00Z");
}

#[test]
fn test_stop_metadata_skips_none_reason_in_serialization() {
    let stop = StopMetadata {
        stopped_from_status: "executing".to_string(),
        stop_reason: None,
        stopped_at: "2026-02-15T10:30:00Z".to_string(),
    };
    let json = serde_json::to_string(&stop).unwrap();
    assert!(!json.contains("stop_reason"));
}

// ===== build_stop_metadata Tests =====

#[test]
fn test_build_stop_metadata_with_reason() {
    let update = build_stop_metadata(InternalStatus::Merging, Some("Protect main".to_string()));
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    let stop_json = parsed.get("stop_metadata").unwrap().as_str().unwrap();
    let stop: StopMetadata = serde_json::from_str(stop_json).unwrap();

    assert_eq!(stop.stopped_from_status, "merging");
    assert_eq!(stop.stop_reason, Some("Protect main".to_string()));
}

#[test]
fn test_build_stop_metadata_without_reason() {
    let update = build_stop_metadata(InternalStatus::Executing, None);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    let stop_json = parsed.get("stop_metadata").unwrap().as_str().unwrap();
    let stop: StopMetadata = serde_json::from_str(stop_json).unwrap();

    assert_eq!(stop.stopped_from_status, "executing");
    assert_eq!(stop.stop_reason, None);
}

#[test]
fn test_build_stop_metadata_preserves_existing_keys() {
    let existing = r#"{"trigger_origin":"scheduler"}"#;
    let update = build_stop_metadata(InternalStatus::Merging, Some("Reason".to_string()));
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(parsed.contains_key("trigger_origin"));
    assert!(parsed.contains_key("stop_metadata"));
}

// ===== parse_stop_metadata Tests =====

#[test]
fn test_parse_stop_metadata_valid() {
    let update = build_stop_metadata(InternalStatus::Reviewing, Some("Test".to_string()));
    let metadata_json = update.merge_into(None);

    let parsed = parse_stop_metadata(Some(&metadata_json));
    assert!(parsed.is_some());
    let parsed = parsed.unwrap();
    assert_eq!(parsed.stopped_from_status, "reviewing");
    assert_eq!(parsed.stop_reason, Some("Test".to_string()));
}

#[test]
fn test_parse_stop_metadata_none_returns_none() {
    assert_eq!(parse_stop_metadata(None), None);
}

#[test]
fn test_parse_stop_metadata_missing_key_returns_none() {
    let metadata = r#"{"other_key":"value"}"#;
    assert_eq!(parse_stop_metadata(Some(metadata)), None);
}

#[test]
fn test_parse_stop_metadata_invalid_json_returns_none() {
    let metadata = "not valid json";
    assert_eq!(parse_stop_metadata(Some(metadata)), None);
}

#[test]
fn test_parse_stop_metadata_invalid_inner_json_returns_none() {
    let metadata = r#"{"stop_metadata":"not valid json"}"#;
    assert_eq!(parse_stop_metadata(Some(metadata)), None);
}

// ===== build_restart_metadata Tests =====

#[test]
fn test_build_restart_metadata_with_note_stores_restart_note() {
    let update = build_restart_metadata(Some("Fix the broken auth flow"));
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("restart_note").unwrap(),
        &Value::String("Fix the broken auth flow".to_string())
    );
    assert!(!parsed.contains_key("stop_metadata"));
}

#[test]
fn test_build_restart_metadata_without_note_leaves_no_restart_note() {
    let update = build_restart_metadata(None);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(!parsed.contains_key("restart_note"));
    assert!(!parsed.contains_key("stop_metadata"));
}

#[test]
fn test_build_restart_metadata_clears_stop_metadata_with_note() {
    let existing = r#"{"stop_metadata":"{\"stopped_from_status\":\"executing\"}","trigger_origin":"scheduler"}"#;
    let update = build_restart_metadata(Some("User note"));
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(!parsed.contains_key("stop_metadata"));
    assert_eq!(
        parsed.get("restart_note").unwrap(),
        &Value::String("User note".to_string())
    );
    // Existing fields preserved
    assert_eq!(
        parsed.get("trigger_origin").unwrap(),
        &Value::String("scheduler".to_string())
    );
}

#[test]
fn test_build_restart_metadata_clears_stop_metadata_without_note() {
    let existing = r#"{"stop_metadata":"{\"stopped_from_status\":\"executing\"}","trigger_origin":"scheduler"}"#;
    let update = build_restart_metadata(None);
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(!parsed.contains_key("stop_metadata"));
    assert!(!parsed.contains_key("restart_note"));
    // Existing fields preserved
    assert_eq!(
        parsed.get("trigger_origin").unwrap(),
        &Value::String("scheduler".to_string())
    );
}

#[test]
fn test_clear_stop_metadata_removes_key() {
    let existing =
        r#"{"stop_metadata":"{\"stopped_from_status\":\"merging\"}","other_key":"value"}"#;
    let update = clear_stop_metadata();
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(!parsed.contains_key("stop_metadata"));
    assert!(parsed.contains_key("other_key"));
}

#[test]
fn test_clear_stop_metadata_on_empty_metadata() {
    let update = clear_stop_metadata();
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_empty());
}

// ===== with_null and merge_into null handling Tests =====

#[test]
fn test_with_null_removes_existing_key() {
    let existing = r#"{"key_to_remove":"value","keep_this":"kept"}"#;
    let update = MetadataUpdate::new().with_null("key_to_remove");
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert!(!parsed.contains_key("key_to_remove"));
    assert_eq!(
        parsed.get("keep_this").unwrap(),
        &Value::String("kept".to_string())
    );
}

#[test]
fn test_with_null_on_nonexistent_key_is_noop() {
    let existing = r#"{"existing_key":"value"}"#;
    let update = MetadataUpdate::new().with_null("nonexistent_key");
    let result = update.merge_into(Some(existing));

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.len(), 1);
    assert!(parsed.contains_key("existing_key"));
}

#[test]
fn test_build_failed_metadata_includes_attempt_count() {
    let data = FailedData::new("Error").with_attempt_count(3);
    let result = build_failed_metadata(&data).merge_into(None);
    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("attempt_count").unwrap(),
        &Value::Number(3.into())
    );
}

#[test]
fn test_build_failed_metadata_attempt_count_zero_by_default() {
    let data = FailedData::new("Error");
    let result = build_failed_metadata(&data).merge_into(None);
    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed.get("attempt_count").unwrap(),
        &Value::Number(0.into())
    );
}

#[test]
fn test_with_u32_adds_numeric_value() {
    let update = MetadataUpdate::new().with_u32("retry_count", 42);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.get("retry_count").unwrap().as_u64().unwrap(), 42u64);
}

#[test]
fn test_with_u32_zero_value() {
    let update = MetadataUpdate::new().with_u32("count", 0);
    let result = update.merge_into(None);

    let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.get("count").unwrap().as_u64().unwrap(), 0u64);
}
