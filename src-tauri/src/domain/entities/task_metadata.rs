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
    /// ISO 8601 timestamp: do not retry merge until this time (provider rate limit)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rate_limit_retry_after: Option<String>,
}

impl MergeRecoveryMetadata {
    /// Create new merge recovery metadata with empty event log
    pub fn new() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
            last_state: MergeRecoveryState::Succeeded,
            rate_limit_retry_after: None,
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
    /// Provider rate limit hit during merge agent run
    ProviderRateLimited,
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
    /// Waiting for provider rate limit to clear
    RateLimited,
}

#[cfg(test)]
#[path = "task_metadata_tests.rs"]
mod tests;
