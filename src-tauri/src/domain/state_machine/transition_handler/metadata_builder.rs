// Pure metadata builder functions for atomic metadata writes
//
// This module provides builder functions that construct metadata JSON
// from state data without any I/O operations. All functions are pure
// and testable in isolation.

use serde_json::{Map, Value};

use crate::domain::state_machine::types::FailedData;

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

    /// Merge this update into existing metadata, preserving existing keys.
    ///
    /// # Arguments
    /// * `existing` - Optional JSON string of existing metadata
    ///
    /// # Returns
    /// JSON string with merged metadata (update keys overwrite existing)
    pub fn merge_into(self, existing: Option<&str>) -> String {
        let mut base = if let Some(existing_str) = existing {
            // Parse existing metadata or start fresh if invalid
            serde_json::from_str::<Map<String, Value>>(existing_str)
                .unwrap_or_else(|_| Map::new())
        } else {
            Map::new()
        };

        // Merge update entries (overwrites existing keys)
        for (key, value) in self.entries {
            base.insert(key, value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_into_with_none_creates_new_object() {
        let update = MetadataUpdate::new().with_string("key1", "value1");
        let result = update.merge_into(None);

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("key1").unwrap(), &Value::String("value1".to_string()));
    }

    #[test]
    fn test_merge_into_preserves_existing_keys() {
        let existing = r#"{"existing_key":"existing_value"}"#;
        let update = MetadataUpdate::new().with_string("new_key", "new_value");
        let result = update.merge_into(Some(existing));

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("existing_key").unwrap(), &Value::String("existing_value".to_string()));
        assert_eq!(parsed.get("new_key").unwrap(), &Value::String("new_value".to_string()));
    }

    #[test]
    fn test_merge_into_overwrites_duplicate_keys() {
        let existing = r#"{"key1":"old_value"}"#;
        let update = MetadataUpdate::new().with_string("key1", "new_value");
        let result = update.merge_into(Some(existing));

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("key1").unwrap(), &Value::String("new_value".to_string()));
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
        assert_eq!(parsed.get("failure_error").unwrap(), &Value::String("Test error".to_string()));
        assert_eq!(parsed.get("failure_details").unwrap(), &Value::String("Test details".to_string()));
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
        assert_eq!(parsed.get("failure_error").unwrap(), &Value::String("Test error".to_string()));
        assert!(parsed.get("failure_details").is_none());
        assert_eq!(parsed.get("is_timeout").unwrap(), &Value::Bool(false));
    }

    #[test]
    fn test_build_trigger_origin_metadata_produces_trigger_origin() {
        let update = build_trigger_origin_metadata("qa");
        let result = update.merge_into(None);

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("trigger_origin").unwrap(), &Value::String("qa".to_string()));
    }

    #[test]
    fn test_key_exists_in_returns_true_when_key_present() {
        let metadata = r#"{"failure_error":"some error"}"#;
        assert!(MetadataUpdate::key_exists_in("failure_error", Some(metadata)));
    }

    #[test]
    fn test_key_exists_in_returns_false_when_key_absent() {
        let metadata = r#"{"other_key":"value"}"#;
        assert!(!MetadataUpdate::key_exists_in("failure_error", Some(metadata)));
    }

    #[test]
    fn test_key_exists_in_returns_false_when_metadata_none() {
        assert!(!MetadataUpdate::key_exists_in("failure_error", None));
    }

    #[test]
    fn test_key_exists_in_returns_false_when_metadata_invalid_json() {
        let invalid = "not valid json";
        assert!(!MetadataUpdate::key_exists_in("failure_error", Some(invalid)));
    }

    #[test]
    fn test_merge_into_handles_invalid_existing_json() {
        let invalid = "not valid json";
        let update = MetadataUpdate::new().with_string("key1", "value1");
        let result = update.merge_into(Some(invalid));

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("key1").unwrap(), &Value::String("value1".to_string()));
        // Should start fresh when existing is invalid
        assert_eq!(parsed.len(), 1);
    }
}
