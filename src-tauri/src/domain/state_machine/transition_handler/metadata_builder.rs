// Pure metadata builder functions for atomic metadata writes
//
// This module provides builder functions that construct metadata JSON
// from state data without any I/O operations. All functions are pure
// and testable in isolation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::domain::entities::status::InternalStatus;
use crate::domain::state_machine::types::FailedData;

// ============================================================================
// StopMetadata - Captures context when a task is stopped mid-execution
// ============================================================================

/// Metadata captured when a task is stopped mid-execution.
///
/// This enables "smart resume" capability where the system can restore
/// the task to its previous state with context about why it was stopped.
///
/// Schema:
/// ```json
/// {
///   "stopped_from_status": "merging",
///   "stop_reason": "User stopped to protect main branch",
///   "stopped_at": "2026-02-15T10:30:00Z"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopMetadata {
    /// The status the task was in when stopped (stored as snake_case string)
    pub stopped_from_status: String,
    /// Optional reason provided by user for stopping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Timestamp when the task was stopped (RFC3339 format)
    pub stopped_at: String,
}

impl StopMetadata {
    /// Create new stop metadata with the given from_status and optional reason.
    /// The timestamp is automatically set to now.
    pub fn new(from_status: InternalStatus, reason: Option<String>) -> Self {
        Self {
            stopped_from_status: from_status.as_str().to_string(),
            stop_reason: reason,
            stopped_at: Utc::now().to_rfc3339(),
        }
    }

    /// Parse the stopped_from_status string back to InternalStatus.
    /// Returns None if the status string is invalid.
    pub fn parse_from_status(&self) -> Option<InternalStatus> {
        self.stopped_from_status.parse().ok()
    }

    /// Parse the stopped_at timestamp string back to DateTime.
    /// Returns None if the timestamp is invalid.
    pub fn parse_stopped_at(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.stopped_at)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    }
}

/// A metadata update containing key-value pairs to merge into existing task metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataUpdate {
    entries: Map<String, Value>,
}

impl MetadataUpdate {
    /// Create a new empty metadata update.
    pub fn new() -> Self {
        Self {
            entries: Map::new(),
        }
    }

    /// Add a string key-value pair to the update.
    pub fn with_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries.insert(key.into(), Value::String(value.into()));
        self
    }

    /// Add a boolean key-value pair to the update.
    pub fn with_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.entries.insert(key.into(), Value::Bool(value));
        self
    }

    /// Add a null value for a key (used to clear/remove keys).
    pub fn with_null(mut self, key: impl Into<String>) -> Self {
        self.entries.insert(key.into(), Value::Null);
        self
    }

    /// Merge this update into existing metadata, preserving existing keys.
    ///
    /// # Arguments
    /// * `existing` - Optional JSON string of existing metadata
    ///
    /// # Returns
    /// JSON string with merged metadata (update keys overwrite existing).
    /// Null values remove keys from the result.
    pub fn merge_into(self, existing: Option<&str>) -> String {
        let mut base = if let Some(existing_str) = existing {
            // Parse existing metadata or start fresh if invalid
            serde_json::from_str::<Map<String, Value>>(existing_str).unwrap_or_else(|_| Map::new())
        } else {
            Map::new()
        };

        // Merge update entries (overwrites existing keys, null removes)
        for (key, value) in self.entries {
            if value.is_null() {
                base.remove(&key);
            } else {
                base.insert(key, value);
            }
        }

        serde_json::to_string(&base).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if a key exists in the given metadata JSON.
    ///
    /// # Arguments
    /// * `key` - The key to check for
    /// * `metadata` - Optional JSON string to check
    ///
    /// # Returns
    /// `true` if the key exists in the metadata, `false` otherwise
    pub fn key_exists_in(key: &str, metadata: Option<&str>) -> bool {
        if let Some(metadata_str) = metadata {
            if let Ok(obj) = serde_json::from_str::<Map<String, Value>>(metadata_str) {
                return obj.contains_key(key);
            }
        }
        false
    }
}

impl Default for MetadataUpdate {
    fn default() -> Self {
        Self::new()
    }
}

/// Build metadata for a failed task.
///
/// Creates metadata with:
/// - `failure_error`: The error message
/// - `failure_details`: Optional additional details
/// - `is_timeout`: Whether the failure was due to a timeout
pub fn build_failed_metadata(data: &FailedData) -> MetadataUpdate {
    let mut update = MetadataUpdate::new()
        .with_string("failure_error", &data.error)
        .with_bool("is_timeout", data.is_timeout);

    if let Some(ref details) = data.details {
        update = update.with_string("failure_details", details);
    }

    update
}

/// Build metadata for trigger origin tracking.
///
/// Creates metadata with:
/// - `trigger_origin`: The origin identifier (e.g., "qa", "scheduler")
pub fn build_trigger_origin_metadata(origin: &str) -> MetadataUpdate {
    MetadataUpdate::new().with_string("trigger_origin", origin)
}

/// Build stop metadata for a task being stopped mid-execution.
///
/// Creates metadata with:
/// - `stopped_from_status`: The status the task was in when stopped
/// - `stop_reason`: Optional reason provided by user
/// - `stopped_at`: RFC3339 timestamp of when the task was stopped
pub fn build_stop_metadata(from_status: InternalStatus, reason: Option<String>) -> MetadataUpdate {
    let stop_data = StopMetadata::new(from_status, reason);
    let json_value = serde_json::to_string(&stop_data).unwrap_or_else(|_| "{}".to_string());
    MetadataUpdate::new().with_string("stop_metadata", json_value)
}

/// Parse stop metadata from a task's metadata JSON string.
///
/// Returns `Some(StopMetadata)` if the stop_metadata key exists and is valid,
/// or `None` if it doesn't exist or parsing fails.
pub fn parse_stop_metadata(metadata: Option<&str>) -> Option<StopMetadata> {
    let metadata_str = metadata?;
    let obj = serde_json::from_str::<Map<String, Value>>(metadata_str).ok()?;
    let stop_json = obj.get("stop_metadata")?.as_str()?;
    serde_json::from_str(stop_json).ok()
}

/// Build metadata update to clear stop metadata (for restart cleanup).
///
/// When a task is restarted, the stop metadata should be cleared
/// to prevent stale context from affecting future operations.
pub fn clear_stop_metadata() -> MetadataUpdate {
    MetadataUpdate::new().with_null("stop_metadata")
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // ===== clear_stop_metadata Tests =====

    #[test]
    fn test_clear_stop_metadata_removes_key() {
        let existing = r#"{"stop_metadata":"{\"stopped_from_status\":\"merging\"}","other_key":"value"}"#;
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
}
