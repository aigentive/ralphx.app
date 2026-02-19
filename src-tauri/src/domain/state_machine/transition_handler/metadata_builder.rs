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
// ResumeCategory - Categorizes states for smart resume handling
// ============================================================================

/// Categories for how a stopped task should be resumed.
///
/// This enum determines the resume behavior based on the state the task
/// was in when stopped:
///
/// | Category | States | Resume Behavior |
/// |----------|--------|-----------------|
/// | **Direct** | `Executing`, `ReExecuting`, `Reviewing`, `QaRefining`, `QaTesting` | Transition directly, spawn agent |
/// | **Validated** | `Merging`, `PendingMerge`, `MergeConflict`, `MergeIncomplete` | Validate git state first |
/// | **Redirect** | `QaPassed`, `RevisionNeeded`, `PendingReview` | Resume to successor state |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeCategory {
    /// Resume directly to the state (spawn agent immediately).
    /// Used for agent-active states where no pre-validation is needed.
    Direct,
    /// Resume with validation (check git state, cleanup agents).
    /// Used for merge-related states that require external state validation.
    Validated,
    /// Resume to a successor state instead (auto-transition states).
    /// These states would auto-transition anyway, so redirect to the target.
    Redirect,
}

impl ResumeCategory {
    /// Returns the resume category for the given status.
    ///
    /// # Arguments
    /// * `status` - The status the task was in when stopped
    ///
    /// # Returns
    /// The appropriate `ResumeCategory` for resume handling.
    pub fn from_status(status: InternalStatus) -> Self {
        use InternalStatus::*;
        match status {
            // Direct resume: agent-active states that don't require validation
            Executing | ReExecuting | Reviewing | QaRefining | QaTesting => ResumeCategory::Direct,

            // Validated resume: merge-related states requiring git validation
            Merging | PendingMerge | MergeConflict | MergeIncomplete => ResumeCategory::Validated,

            // Redirect: auto-transition states
            QaPassed | RevisionNeeded | PendingReview => ResumeCategory::Redirect,

            // All other states fall back to Direct (they shouldn't be stopped from these)
            // but if they are, resuming directly is the safest fallback
            _ => ResumeCategory::Direct,
        }
    }
}

/// Categorize a status for resume handling.
///
/// This is a convenience function that delegates to `ResumeCategory::from_status`.
///
/// # Arguments
/// * `status` - The status the task was in when stopped
///
/// # Returns
/// The appropriate `ResumeCategory` for resume handling.
pub fn categorize_resume_state(status: InternalStatus) -> ResumeCategory {
    ResumeCategory::from_status(status)
}

/// Get the resume target for a stopped task.
///
/// For redirect states, this returns the successor state that the task
/// should be resumed to. For direct and validated states, this returns
/// the original state.
///
/// # Redirect Mapping
/// - `QaPassed` → `PendingReview` (auto-transitions to review anyway)
/// - `RevisionNeeded` → `ReExecuting` (auto-transitions to re-execution)
/// - `PendingReview` → `Reviewing` (spawn reviewer)
///
/// # Arguments
/// * `status` - The status the task was in when stopped
///
/// # Returns
/// The status the task should be resumed to.
pub fn get_resume_target(status: InternalStatus) -> InternalStatus {
    use InternalStatus::*;
    match status {
        // Redirect states: return successor
        QaPassed => PendingReview,
        RevisionNeeded => ReExecuting,
        PendingReview => Reviewing,

        // All other states: resume directly to same state
        other => other,
    }
}

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

    /// Add a u32 numeric key-value pair to the update.
    pub fn with_u32(mut self, key: impl Into<String>, value: u32) -> Self {
        self.entries
            .insert(key.into(), Value::Number(value.into()));
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
/// - `attempt_count`: Number of execution attempts (from `FailedData.attempt_count`)
pub fn build_failed_metadata(data: &FailedData) -> MetadataUpdate {
    let mut update = MetadataUpdate::new()
        .with_string("failure_error", &data.error)
        .with_bool("is_timeout", data.is_timeout)
        .with_u32("attempt_count", data.attempt_count);

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

/// Build metadata update for task restart: clears stop metadata and optionally stores restart note.
///
/// When `note` is `Some`, stores it as `restart_note` for downstream consumers
/// (agent prompts, context hints) to read. The note is ephemeral — cleared after first read.
///
/// When `note` is `None`, behaves identically to `clear_stop_metadata()`.
pub fn build_restart_metadata(note: Option<&str>) -> MetadataUpdate {
    let base = MetadataUpdate::new().with_null("stop_metadata");
    match note {
        Some(n) => base.with_string("restart_note", n),
        None => base,
    }
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

    // ===== with_u32 Tests =====

    #[test]
    fn test_with_u32_adds_numeric_value() {
        let update = MetadataUpdate::new().with_u32("retry_count", 42);
        let result = update.merge_into(None);

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(
            parsed.get("retry_count").unwrap().as_u64().unwrap(),
            42u64
        );
    }

    #[test]
    fn test_with_u32_zero_value() {
        let update = MetadataUpdate::new().with_u32("count", 0);
        let result = update.merge_into(None);

        let parsed: Map<String, Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.get("count").unwrap().as_u64().unwrap(), 0u64);
    }
}
