// Task metadata types for structured data stored in tasks.metadata JSON field
// Provides type-safe handling of merge recovery events and other metadata

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::TaskId;

/// Merge recovery metadata stored in tasks.metadata
/// Tracks the full history of merge deferral and retry attempts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRecoveryMetadata {
    /// Schema version for future compatibility
    pub version: u32,
    /// Append-only event log (capped at 50 events, oldest trimmed)
    pub events: Vec<MergeRecoveryEvent>,
    /// Current recovery state
    pub last_state: MergeRecoveryState,
}

impl MergeRecoveryMetadata {
    /// Create new merge recovery metadata with empty event log
    pub fn new() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
            last_state: MergeRecoveryState::Succeeded,
        }
    }

    /// Maximum number of events to keep in the log
    pub const MAX_EVENTS: usize = 50;

    /// Append a new event to the log
    /// Automatically trims oldest events if cap is exceeded
    pub fn append_event(&mut self, event: MergeRecoveryEvent) {
        self.events.push(event);
        self.trim_if_needed();
    }

    /// Append a new event and update last_state
    pub fn append_event_with_state(
        &mut self,
        event: MergeRecoveryEvent,
        state: MergeRecoveryState,
    ) {
        self.append_event(event);
        self.last_state = state;
    }

    /// Trim events if count exceeds MAX_EVENTS
    /// Removes oldest events (from the beginning of the vector)
    fn trim_if_needed(&mut self) {
        if self.events.len() > Self::MAX_EVENTS {
            let excess = self.events.len() - Self::MAX_EVENTS;
            self.events.drain(0..excess);
        }
    }

    /// Parse metadata from task's metadata JSON string
    /// Returns Ok(Some(metadata)) if merge_recovery key exists and is valid
    /// Returns Ok(None) if merge_recovery key doesn't exist
    /// Returns Err if JSON is invalid or merge_recovery value can't be parsed
    pub fn from_task_metadata(
        metadata_json: Option<&str>,
    ) -> Result<Option<Self>, serde_json::Error> {
        let Some(json_str) = metadata_json else {
            return Ok(None);
        };

        let value: serde_json::Value = serde_json::from_str(json_str)?;

        if let Some(merge_recovery) = value.get("merge_recovery") {
            let recovery: MergeRecoveryMetadata = serde_json::from_value(merge_recovery.clone())?;
            Ok(Some(recovery))
        } else {
            Ok(None)
        }
    }

    /// Update task's metadata JSON string with this merge recovery metadata
    /// Preserves other keys in the metadata object
    /// Returns updated JSON string
    pub fn update_task_metadata(
        &self,
        existing_metadata: Option<&str>,
    ) -> Result<String, serde_json::Error> {
        let mut metadata_obj = if let Some(json_str) = existing_metadata {
            serde_json::from_str::<serde_json::Value>(json_str)
                .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        if let Some(obj) = metadata_obj.as_object_mut() {
            obj.insert("merge_recovery".to_string(), serde_json::to_value(self)?);
        }

        serde_json::to_string(&metadata_obj)
    }
}

impl Default for MergeRecoveryMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual merge recovery event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRecoveryEvent {
    /// When this event occurred
    pub at: DateTime<Utc>,
    /// Type of event
    pub kind: MergeRecoveryEventKind,
    /// Who/what triggered this event
    pub source: MergeRecoverySource,
    /// Reason code for categorization
    pub reason_code: MergeRecoveryReasonCode,
    /// Human-readable message
    pub message: String,
    /// Target branch being merged into
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    /// Source branch being merged from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    /// Task blocking this merge (if deferred due to concurrent merge)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_task_id: Option<TaskId>,
    /// Attempt number for retries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    /// Classification of the failure source for smart retry decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_source: Option<MergeFailureSource>,
    /// SHA of source branch at time of failure (for SHA comparison guard)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_sha: Option<String>,
}

impl MergeRecoveryEvent {
    /// Create a new merge recovery event
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: MergeRecoveryEventKind,
        source: MergeRecoverySource,
        reason_code: MergeRecoveryReasonCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            at: Utc::now(),
            kind,
            source,
            reason_code,
            message: message.into(),
            target_branch: None,
            source_branch: None,
            blocking_task_id: None,
            attempt: None,
            failure_source: None,
            source_sha: None,
        }
    }

    /// Builder method to add target branch
    pub fn with_target_branch(mut self, branch: impl Into<String>) -> Self {
        self.target_branch = Some(branch.into());
        self
    }

    /// Builder method to add source branch
    pub fn with_source_branch(mut self, branch: impl Into<String>) -> Self {
        self.source_branch = Some(branch.into());
        self
    }

    /// Builder method to add blocking task
    pub fn with_blocking_task(mut self, task_id: TaskId) -> Self {
        self.blocking_task_id = Some(task_id);
        self
    }

    /// Builder method to add attempt number
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = Some(attempt);
        self
    }

    /// Builder method to set failure source classification
    pub fn with_failure_source(mut self, failure_source: MergeFailureSource) -> Self {
        self.failure_source = Some(failure_source);
        self
    }

    /// Builder method to record source branch SHA at failure time
    pub fn with_source_sha(mut self, sha: impl Into<String>) -> Self {
        self.source_sha = Some(sha.into());
        self
    }
}

/// Classification of why a merge failure occurred.
/// Used by the reconciler to decide whether auto-retry is safe.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeFailureSource {
    /// Transient git operation error (lock files, timeouts, network) — safe to auto-retry
    TransientGit,
    /// Agent explicitly called report_conflict — human deliberate decision, do NOT auto-retry
    AgentReported,
    /// System detected failure (conflict markers, MERGE_HEAD, stale rebase) — safe to auto-retry
    SystemDetected,
    /// Post-merge validation reverted — do not auto-retry until code changes
    ValidationFailed,
}

/// Type of merge recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeRecoveryEventKind {
    /// Merge was deferred due to conflict or blocking condition
    Deferred,
    /// Automatic retry was triggered by system
    AutoRetryTriggered,
    /// Retry attempt started
    AttemptStarted,
    /// Retry attempt failed
    AttemptFailed,
    /// Retry attempt succeeded
    AttemptSucceeded,
    /// Manual retry initiated by user
    ManualRetry,
    /// Main-branch merge deferred because agents are running
    MainMergeDeferred,
    /// Main-branch merge retry triggered after agents went idle
    MainMergeRetry,
}

/// Source of the merge recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeRecoverySource {
    /// Triggered by system logic
    System,
    /// Triggered by automatic retry mechanism
    Auto,
    /// Triggered by user action
    User,
}

/// Reason code for merge recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeRecoveryReasonCode {
    /// Target branch is busy with another merge
    TargetBranchBusy,
    /// Git operation error
    GitError,
    /// Validation failed
    ValidationFailed,
    /// Source or target branch does not exist
    BranchNotFound,
    /// Merge deferred because agents are running globally
    AgentsRunning,
    /// Deferred merge forced retry after timeout expired
    DeferredTimeout,
    /// Unknown/unclassified reason
    Unknown,
}

/// Current state of merge recovery
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeRecoveryState {
    /// Merge is deferred, waiting for retry
    Deferred,
    /// Currently retrying
    Retrying,
    /// Recovery failed
    Failed,
    /// Recovery succeeded
    Succeeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_recovery_metadata_new_creates_empty() {
        let meta = MergeRecoveryMetadata::new();
        assert_eq!(meta.version, 1);
        assert!(meta.events.is_empty());
        assert_eq!(meta.last_state, MergeRecoveryState::Succeeded);
    }

    #[test]
    fn merge_recovery_metadata_default_works() {
        let meta = MergeRecoveryMetadata::default();
        assert_eq!(meta.version, 1);
        assert!(meta.events.is_empty());
    }

    #[test]
    fn merge_recovery_metadata_max_events_constant() {
        assert_eq!(MergeRecoveryMetadata::MAX_EVENTS, 50);
    }

    #[test]
    fn merge_recovery_event_new_sets_defaults() {
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Merge deferred",
        );

        assert_eq!(event.kind, MergeRecoveryEventKind::Deferred);
        assert_eq!(event.source, MergeRecoverySource::System);
        assert_eq!(event.reason_code, MergeRecoveryReasonCode::TargetBranchBusy);
        assert_eq!(event.message, "Merge deferred");
        assert!(event.target_branch.is_none());
        assert!(event.source_branch.is_none());
        assert!(event.blocking_task_id.is_none());
        assert!(event.attempt.is_none());
    }

    #[test]
    fn merge_recovery_event_builder_methods() {
        let task_id = TaskId::new();
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::AutoRetryTriggered,
            MergeRecoverySource::Auto,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Auto retry",
        )
        .with_target_branch("main")
        .with_source_branch("task-branch")
        .with_blocking_task(task_id.clone())
        .with_attempt(2);

        assert_eq!(event.target_branch, Some("main".to_string()));
        assert_eq!(event.source_branch, Some("task-branch".to_string()));
        assert_eq!(event.blocking_task_id, Some(task_id));
        assert_eq!(event.attempt, Some(2));
    }

    #[test]
    fn merge_recovery_event_serializes_to_json() {
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Merge deferred",
        )
        .with_target_branch("main")
        .with_attempt(1);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"kind\":\"deferred\""));
        assert!(json.contains("\"source\":\"system\""));
        assert!(json.contains("\"reason_code\":\"target_branch_busy\""));
        assert!(json.contains("\"target_branch\":\"main\""));
        assert!(json.contains("\"attempt\":1"));
    }

    #[test]
    fn merge_recovery_event_deserializes_from_json() {
        let json = r#"{
            "at": "2026-02-11T10:00:00Z",
            "kind": "deferred",
            "source": "system",
            "reason_code": "target_branch_busy",
            "message": "Merge deferred",
            "target_branch": "main",
            "attempt": 1
        }"#;

        let event: MergeRecoveryEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.kind, MergeRecoveryEventKind::Deferred);
        assert_eq!(event.source, MergeRecoverySource::System);
        assert_eq!(event.reason_code, MergeRecoveryReasonCode::TargetBranchBusy);
        assert_eq!(event.message, "Merge deferred");
        assert_eq!(event.target_branch, Some("main".to_string()));
        assert_eq!(event.attempt, Some(1));
    }

    #[test]
    fn merge_recovery_event_skips_serializing_none_fields() {
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Merge deferred",
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("\"target_branch\""));
        assert!(!json.contains("\"source_branch\""));
        assert!(!json.contains("\"blocking_task_id\""));
        assert!(!json.contains("\"attempt\""));
    }

    #[test]
    fn merge_recovery_metadata_serializes_to_json() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.events.push(MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        ));
        meta.last_state = MergeRecoveryState::Deferred;

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"events\":["));
        assert!(json.contains("\"last_state\":\"deferred\""));
    }

    #[test]
    fn merge_recovery_metadata_deserializes_from_json() {
        let json = r#"{
            "version": 1,
            "events": [
                {
                    "at": "2026-02-11T10:00:00Z",
                    "kind": "deferred",
                    "source": "system",
                    "reason_code": "target_branch_busy",
                    "message": "Deferred"
                }
            ],
            "last_state": "deferred"
        }"#;

        let meta: MergeRecoveryMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.version, 1);
        assert_eq!(meta.events.len(), 1);
        assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
    }

    #[test]
    fn merge_recovery_metadata_roundtrip() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.events.push(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Deferred",
            )
            .with_target_branch("main")
            .with_attempt(1),
        );
        meta.last_state = MergeRecoveryState::Deferred;

        let json = serde_json::to_string(&meta).unwrap();
        let restored: MergeRecoveryMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(meta, restored);
    }

    #[test]
    fn merge_recovery_event_kind_serialization() {
        let kinds = [
            (MergeRecoveryEventKind::Deferred, "deferred"),
            (
                MergeRecoveryEventKind::AutoRetryTriggered,
                "auto_retry_triggered",
            ),
            (MergeRecoveryEventKind::AttemptStarted, "attempt_started"),
            (MergeRecoveryEventKind::AttemptFailed, "attempt_failed"),
            (
                MergeRecoveryEventKind::AttemptSucceeded,
                "attempt_succeeded",
            ),
            (MergeRecoveryEventKind::ManualRetry, "manual_retry"),
            (MergeRecoveryEventKind::MainMergeDeferred, "main_merge_deferred"),
            (MergeRecoveryEventKind::MainMergeRetry, "main_merge_retry"),
        ];

        for (kind, expected) in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    #[test]
    fn merge_recovery_source_serialization() {
        let sources = [
            (MergeRecoverySource::System, "system"),
            (MergeRecoverySource::Auto, "auto"),
            (MergeRecoverySource::User, "user"),
        ];

        for (source, expected) in &sources {
            let json = serde_json::to_string(source).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    #[test]
    fn merge_recovery_reason_code_serialization() {
        let codes = [
            (
                MergeRecoveryReasonCode::TargetBranchBusy,
                "target_branch_busy",
            ),
            (MergeRecoveryReasonCode::GitError, "git_error"),
            (
                MergeRecoveryReasonCode::ValidationFailed,
                "validation_failed",
            ),
            (MergeRecoveryReasonCode::BranchNotFound, "branch_not_found"),
            (MergeRecoveryReasonCode::AgentsRunning, "agents_running"),
            (MergeRecoveryReasonCode::DeferredTimeout, "deferred_timeout"),
            (MergeRecoveryReasonCode::Unknown, "unknown"),
        ];

        for (code, expected) in &codes {
            let json = serde_json::to_string(code).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    #[test]
    fn merge_recovery_state_serialization() {
        let states = [
            (MergeRecoveryState::Deferred, "deferred"),
            (MergeRecoveryState::Retrying, "retrying"),
            (MergeRecoveryState::Failed, "failed"),
            (MergeRecoveryState::Succeeded, "succeeded"),
        ];

        for (state, expected) in &states {
            let json = serde_json::to_string(state).unwrap();
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    // ===== Helper Function Tests =====

    #[test]
    fn append_event_adds_to_log() {
        let mut meta = MergeRecoveryMetadata::new();
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        );

        meta.append_event(event.clone());

        assert_eq!(meta.events.len(), 1);
        assert_eq!(meta.events[0].message, "Deferred");
    }

    #[test]
    fn append_event_with_state_updates_both() {
        let mut meta = MergeRecoveryMetadata::new();
        let event = MergeRecoveryEvent::new(
            MergeRecoveryEventKind::Deferred,
            MergeRecoverySource::System,
            MergeRecoveryReasonCode::TargetBranchBusy,
            "Deferred",
        );

        meta.append_event_with_state(event, MergeRecoveryState::Deferred);

        assert_eq!(meta.events.len(), 1);
        assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
    }

    #[test]
    fn append_event_trims_when_exceeds_max() {
        let mut meta = MergeRecoveryMetadata::new();

        // Add MAX_EVENTS + 5 events
        for i in 0..(MergeRecoveryMetadata::MAX_EVENTS + 5) {
            let event = MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                format!("Event {}", i),
            );
            meta.append_event(event);
        }

        // Should only keep MAX_EVENTS
        assert_eq!(meta.events.len(), MergeRecoveryMetadata::MAX_EVENTS);

        // Oldest events should be trimmed, newest kept
        // First event should now be event #5 (0-4 trimmed)
        assert_eq!(meta.events[0].message, "Event 5");
        assert_eq!(
            meta.events[MergeRecoveryMetadata::MAX_EVENTS - 1].message,
            format!("Event {}", MergeRecoveryMetadata::MAX_EVENTS + 4)
        );
    }

    #[test]
    fn append_event_preserves_chronological_order() {
        let mut meta = MergeRecoveryMetadata::new();

        for i in 0..10 {
            let event = MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                format!("Event {}", i),
            );
            meta.append_event(event);
        }

        // Events should be in order
        for i in 0..10 {
            assert_eq!(meta.events[i].message, format!("Event {}", i));
        }
    }

    #[test]
    fn from_task_metadata_with_no_metadata() {
        let result = MergeRecoveryMetadata::from_task_metadata(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn from_task_metadata_with_empty_json() {
        let result = MergeRecoveryMetadata::from_task_metadata(Some("{}")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn from_task_metadata_with_other_keys_only() {
        let json = r#"{"error": "some error", "source_branch": "task-branch"}"#;
        let result = MergeRecoveryMetadata::from_task_metadata(Some(json)).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn from_task_metadata_with_valid_recovery_data() {
        let json = r#"{
            "error": "some error",
            "merge_recovery": {
                "version": 1,
                "events": [
                    {
                        "at": "2026-02-11T10:00:00Z",
                        "kind": "deferred",
                        "source": "system",
                        "reason_code": "target_branch_busy",
                        "message": "Deferred"
                    }
                ],
                "last_state": "deferred"
            }
        }"#;

        let result = MergeRecoveryMetadata::from_task_metadata(Some(json)).unwrap();
        assert!(result.is_some());

        let meta = result.unwrap();
        assert_eq!(meta.version, 1);
        assert_eq!(meta.events.len(), 1);
        assert_eq!(meta.last_state, MergeRecoveryState::Deferred);
        assert_eq!(meta.events[0].message, "Deferred");
    }

    #[test]
    fn from_task_metadata_with_invalid_json() {
        let result = MergeRecoveryMetadata::from_task_metadata(Some("not json"));
        assert!(result.is_err());
    }

    #[test]
    fn from_task_metadata_with_invalid_recovery_structure() {
        let json = r#"{"merge_recovery": "not an object"}"#;
        let result = MergeRecoveryMetadata::from_task_metadata(Some(json));
        assert!(result.is_err());
    }

    #[test]
    fn update_task_metadata_creates_new_object() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.append_event_with_state(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Deferred",
            ),
            MergeRecoveryState::Deferred,
        );

        let result = meta.update_task_metadata(None).unwrap();

        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(value.get("merge_recovery").is_some());
        assert_eq!(value["merge_recovery"]["version"], 1);
        assert_eq!(
            value["merge_recovery"]["events"].as_array().unwrap().len(),
            1
        );
        assert_eq!(value["merge_recovery"]["last_state"], "deferred");
    }

    #[test]
    fn update_task_metadata_preserves_existing_keys() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.append_event_with_state(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Deferred",
            ),
            MergeRecoveryState::Deferred,
        );

        let existing = r#"{"error": "some error", "source_branch": "task-branch"}"#;
        let result = meta.update_task_metadata(Some(existing)).unwrap();

        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["error"], "some error");
        assert_eq!(value["source_branch"], "task-branch");
        assert!(value.get("merge_recovery").is_some());
    }

    #[test]
    fn update_task_metadata_overwrites_existing_recovery() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.append_event_with_state(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::AutoRetryTriggered,
                MergeRecoverySource::Auto,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Retry",
            ),
            MergeRecoveryState::Retrying,
        );

        let existing = r#"{
            "merge_recovery": {
                "version": 1,
                "events": [],
                "last_state": "succeeded"
            }
        }"#;

        let result = meta.update_task_metadata(Some(existing)).unwrap();

        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            value["merge_recovery"]["events"].as_array().unwrap().len(),
            1
        );
        assert_eq!(value["merge_recovery"]["last_state"], "retrying");
    }

    #[test]
    fn update_task_metadata_handles_malformed_existing() {
        let mut meta = MergeRecoveryMetadata::new();
        meta.append_event_with_state(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Deferred",
            ),
            MergeRecoveryState::Deferred,
        );

        // Malformed JSON should be replaced with empty object
        let result = meta.update_task_metadata(Some("not json")).unwrap();

        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(value.get("merge_recovery").is_some());
        // Only merge_recovery key should exist (malformed JSON was discarded)
        assert_eq!(value.as_object().unwrap().len(), 1);
    }

    #[test]
    fn roundtrip_from_and_update_task_metadata() {
        // Create metadata with events
        let mut original = MergeRecoveryMetadata::new();
        original.append_event_with_state(
            MergeRecoveryEvent::new(
                MergeRecoveryEventKind::Deferred,
                MergeRecoverySource::System,
                MergeRecoveryReasonCode::TargetBranchBusy,
                "Deferred",
            )
            .with_target_branch("main")
            .with_attempt(1),
            MergeRecoveryState::Deferred,
        );

        // Serialize to task metadata JSON
        let json_str = original.update_task_metadata(None).unwrap();

        // Parse back from task metadata JSON
        let parsed = MergeRecoveryMetadata::from_task_metadata(Some(&json_str))
            .unwrap()
            .unwrap();

        // Should match original
        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.events.len(), original.events.len());
        assert_eq!(parsed.last_state, original.last_state);
        assert_eq!(parsed.events[0].message, "Deferred");
        assert_eq!(parsed.events[0].target_branch, Some("main".to_string()));
        assert_eq!(parsed.events[0].attempt, Some(1));
    }

    #[test]
    fn merge_recovery_reason_code_branch_not_found_serializes() {
        let code = MergeRecoveryReasonCode::BranchNotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"branch_not_found\"");

        let parsed: MergeRecoveryReasonCode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MergeRecoveryReasonCode::BranchNotFound);
    }
}
