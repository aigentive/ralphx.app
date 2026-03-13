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
    /// Whether the circuit breaker has fired (prevents auto-retry until manual reset)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub circuit_breaker_active: bool,
    /// Human-readable reason why the circuit breaker fired
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub circuit_breaker_reason: Option<String>,
}

impl MergeRecoveryMetadata {
    /// Create new merge recovery metadata with empty event log
    pub fn new() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
            last_state: MergeRecoveryState::Succeeded,
            rate_limit_retry_after: None,
            circuit_breaker_active: false,
            circuit_breaker_reason: None,
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
    /// Worktree path deleted but git entry still exists (TOCTOU race) — safe to auto-retry after prune
    WorktreeMissing,
    /// Git process spawn failed (ENOENT or permission denied) — may indicate worktree issue
    SpawnFailure,
    /// Git lock file contention — transient, safe to retry
    LockContention,
    /// Provider rate limit hit — respect retry_after timestamp
    RateLimited,
    /// Target branch is busy with another merge — deferral-based, never counts toward circuit breaker
    TargetBranchBusy,
    /// Unrecognized failure source from stored metadata (backward compat)
    #[serde(other)]
    Unknown,
}

/// Whether the system will automatically retry a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// System will auto-retry this failure
    AutoRetry,
    /// System will auto-retry but does NOT count toward the circuit breaker threshold (deferral-based)
    AutoRetryNoCB,
    /// System will NOT auto-retry; user manual retry is always allowed
    NoAutomaticRetry,
}

impl MergeFailureSource {
    /// Returns the retry strategy for this failure source.
    pub fn retry_strategy(&self) -> RetryStrategy {
        match self {
            Self::AgentReported | Self::ValidationFailed => RetryStrategy::NoAutomaticRetry,
            Self::TargetBranchBusy => RetryStrategy::AutoRetryNoCB,
            _ => RetryStrategy::AutoRetry,
        }
    }
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

/// Execution recovery metadata stored in tasks.metadata
/// Tracks the full history of execution timeout and retry attempts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRecoveryMetadata {
    /// Schema version for future compatibility
    pub version: u32,
    /// Append-only event log (capped at 50 events, oldest trimmed)
    pub events: Vec<ExecutionRecoveryEvent>,
    /// Current recovery state
    pub last_state: ExecutionRecoveryState,
    /// When true, reconciler will not auto-retry (user stopped retrying or max retries exceeded)
    pub stop_retrying: bool,
}

impl ExecutionRecoveryMetadata {
    /// Create new execution recovery metadata with empty event log
    pub fn new() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
            last_state: ExecutionRecoveryState::Retrying,
            stop_retrying: false,
        }
    }

    /// Maximum number of events to keep in the log
    pub const MAX_EVENTS: usize = 50;

    /// Append a new event to the log
    /// Automatically trims oldest events if cap is exceeded
    pub fn append_event(&mut self, event: ExecutionRecoveryEvent) {
        self.events.push(event);
        self.trim_if_needed();
    }

    /// Append a new event and update last_state
    pub fn append_event_with_state(
        &mut self,
        event: ExecutionRecoveryEvent,
        state: ExecutionRecoveryState,
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

    /// Count `AutoRetryTriggered` events filtered by failure source.
    /// Used for per-source retry budgets (e.g. GitIsolation has its own cap, separate from timeout retries).
    pub fn auto_retry_count_for_source(&self, source: ExecutionFailureSource) -> u32 {
        self.events
            .iter()
            .filter(|e| {
                matches!(e.kind, ExecutionRecoveryEventKind::AutoRetryTriggered)
                    && e.failure_source == Some(source)
            })
            .count() as u32
    }

    /// Returns true if the last recorded failure is transient (safe to auto-retry)
    /// Checks only the most recent event — not historical ones — to avoid stale state.
    pub fn last_failure_is_transient(&self) -> bool {
        self.events
            .last()
            .and_then(|e| e.failure_source.as_ref())
            .map(|source| source.is_transient())
            .unwrap_or(false)
    }

    /// Parse metadata from task's metadata JSON string
    /// Returns Ok(Some(metadata)) if execution_recovery key exists and is valid
    /// Returns Ok(None) if execution_recovery key doesn't exist
    /// Returns Err if JSON is invalid or execution_recovery value can't be parsed
    pub fn from_task_metadata(
        metadata_json: Option<&str>,
    ) -> Result<Option<Self>, serde_json::Error> {
        let Some(json_str) = metadata_json else {
            return Ok(None);
        };
        Self::from_json(json_str)
    }

    /// Parse metadata from a JSON string
    /// Returns Ok(Some(metadata)) if execution_recovery key exists and is valid
    /// Returns Ok(None) if execution_recovery key doesn't exist
    pub fn from_json(json_str: &str) -> Result<Option<Self>, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json_str)?;

        if let Some(execution_recovery) = value.get("execution_recovery") {
            let recovery: ExecutionRecoveryMetadata =
                serde_json::from_value(execution_recovery.clone())?;
            Ok(Some(recovery))
        } else {
            Ok(None)
        }
    }

    /// Update task's metadata JSON string with this execution recovery metadata
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
            obj.insert(
                "execution_recovery".to_string(),
                serde_json::to_value(self)?,
            );
        }

        serde_json::to_string(&metadata_obj)
    }
}

impl Default for ExecutionRecoveryMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual execution recovery event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionRecoveryEvent {
    /// When this event occurred
    pub at: DateTime<Utc>,
    /// Type of event
    pub kind: ExecutionRecoveryEventKind,
    /// Who/what triggered this event
    pub source: ExecutionRecoverySource,
    /// Reason code for categorization
    pub reason_code: ExecutionRecoveryReasonCode,
    /// Human-readable message
    pub message: String,
    /// Attempt number for retries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    /// Classification of the failure source for smart retry decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_source: Option<ExecutionFailureSource>,
}

impl ExecutionRecoveryEvent {
    /// Create a new execution recovery event
    pub fn new(
        kind: ExecutionRecoveryEventKind,
        source: ExecutionRecoverySource,
        reason_code: ExecutionRecoveryReasonCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            at: Utc::now(),
            kind,
            source,
            reason_code,
            message: message.into(),
            attempt: None,
            failure_source: None,
        }
    }

    /// Builder method to add attempt number
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = Some(attempt);
        self
    }

    /// Builder method to set failure source classification
    pub fn with_failure_source(mut self, failure_source: ExecutionFailureSource) -> Self {
        self.failure_source = Some(failure_source);
        self
    }
}

/// Error prefix for git isolation failures.
/// Use this constant at both the generation site (on_enter_states.rs) and
/// classification site (task_transition_service.rs) to avoid fragile string matching.
pub const GIT_ISOLATION_ERROR_PREFIX: &str = "Git isolation failed";

/// Classification of why an execution failure occurred.
/// Used by the reconciler to decide whether auto-retry is safe.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionFailureSource {
    /// Agent stream produced no output within timeout — safe to auto-retry
    TransientTimeout,
    /// Agent stream stalled during parse — safe to auto-retry
    ParseStall,
    /// Agent process exited unexpectedly — safe to auto-retry
    AgentCrash,
    /// Provider returned an error (handled by Paused) — do NOT auto-retry here
    ProviderError,
    /// Wall-clock (C5) timeout fired — do NOT auto-retry (would loop infinitely)
    WallClockTimeout,
    /// Transient git isolation failure (stale index.lock, leftover worktree dir, concurrent git op) — safe to auto-retry after cleanup
    GitIsolation,
    /// Unknown/unclassified failure
    Unknown,
}

impl ExecutionFailureSource {
    /// Returns true if this failure source is transient and safe to auto-retry
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            ExecutionFailureSource::TransientTimeout
                | ExecutionFailureSource::ParseStall
                | ExecutionFailureSource::AgentCrash
                | ExecutionFailureSource::GitIsolation
        )
    }
}

/// Type of execution recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRecoveryEventKind {
    /// Execution failed and was recorded
    Failed,
    /// Automatic retry was triggered by the reconciler
    AutoRetryTriggered,
    /// Retry attempt started
    AttemptStarted,
    /// Retry attempt succeeded
    AttemptSucceeded,
    /// Manual retry initiated by user
    ManualRetry,
    /// User or system stopped further retries
    StopRetrying,
}

/// Source of the execution recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRecoverySource {
    /// Triggered by system logic
    System,
    /// Triggered by automatic retry mechanism
    Auto,
    /// Triggered by user action
    User,
    /// Triggered by startup recovery (recover_timeout_failures) — GAP M5 sentinel
    Startup,
}

/// Reason code for execution recovery event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRecoveryReasonCode {
    /// Agent stream timed out (no output)
    Timeout,
    /// Agent stream stalled during parse
    ParseStall,
    /// Agent process exited unexpectedly
    AgentExit,
    /// Provider returned an error
    ProviderError,
    /// Wall-clock (C5) hard limit exceeded
    WallClockExceeded,
    /// Maximum retry budget exhausted
    MaxRetriesExceeded,
    /// User explicitly stopped retrying
    UserStopped,
    /// Git isolation failure (stale index.lock, leftover worktree dir, concurrent git op)
    GitIsolationFailed,
    /// Unknown/unclassified reason
    Unknown,
}

/// Current state of execution recovery
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRecoveryState {
    /// Currently retrying (auto-retry in progress)
    Retrying,
    /// Recovery permanently failed (max retries exceeded or user stopped)
    Failed,
    /// Execution completed successfully after retries
    Succeeded,
}

#[cfg(test)]
#[path = "task_metadata_tests.rs"]
mod tests;
