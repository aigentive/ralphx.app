// Supervisor events
// Events emitted by the agent execution layer for supervisor monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about a tool call
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// Name of the tool (e.g., "Write", "Edit", "Bash")
    pub tool_name: String,
    /// Arguments passed to the tool (as JSON string)
    pub arguments: String,
    /// Timestamp of the call
    pub timestamp: DateTime<Utc>,
    /// Whether the call succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl ToolCallInfo {
    pub fn new(tool_name: impl Into<String>, arguments: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments: arguments.into(),
            timestamp: Utc::now(),
            success: true,
            error: None,
        }
    }

    pub fn failed(
        tool_name: impl Into<String>,
        arguments: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments: arguments.into(),
            timestamp: Utc::now(),
            success: false,
            error: Some(error.into()),
        }
    }

    /// Check if this call is similar to another (same tool and similar args)
    pub fn is_similar_to(&self, other: &ToolCallInfo) -> bool {
        self.tool_name == other.tool_name && self.arguments == other.arguments
    }
}

/// Information about an error that occurred
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Error message
    pub message: String,
    /// Source of the error (tool name, component, etc.)
    pub source: String,
    /// Whether this is a recoverable error
    pub recoverable: bool,
    /// Timestamp of the error
    pub timestamp: DateTime<Utc>,
}

impl ErrorInfo {
    pub fn new(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: source.into(),
            recoverable: true,
            timestamp: Utc::now(),
        }
    }

    pub fn fatal(message: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: source.into(),
            recoverable: false,
            timestamp: Utc::now(),
        }
    }
}

/// Information about execution progress
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Whether there are uncommitted file changes
    pub has_file_changes: bool,
    /// Number of files modified since last check
    pub files_modified: usize,
    /// Whether there are new commits
    pub has_new_commits: bool,
    /// Token usage so far
    pub tokens_used: u32,
    /// Time elapsed in seconds
    pub elapsed_seconds: u64,
    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,
}

impl ProgressInfo {
    pub fn new() -> Self {
        Self {
            has_file_changes: false,
            files_modified: 0,
            has_new_commits: false,
            tokens_used: 0,
            elapsed_seconds: 0,
            timestamp: Utc::now(),
        }
    }

    /// Check if there's meaningful progress
    pub fn has_progress(&self) -> bool {
        self.has_file_changes || self.has_new_commits || self.files_modified > 0
    }
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Events emitted for supervisor monitoring
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SupervisorEvent {
    /// Task execution started
    TaskStart {
        task_id: String,
        agent_role: String,
        timestamp: DateTime<Utc>,
    },
    /// A tool was called
    ToolCall { task_id: String, info: ToolCallInfo },
    /// An error occurred
    Error { task_id: String, info: ErrorInfo },
    /// Progress tick (periodic check)
    ProgressTick { task_id: String, info: ProgressInfo },
    /// Token usage threshold exceeded
    TokenThreshold {
        task_id: String,
        tokens_used: u32,
        threshold: u32,
        timestamp: DateTime<Utc>,
    },
    /// Time threshold exceeded
    TimeThreshold {
        task_id: String,
        elapsed_minutes: u32,
        threshold_minutes: u32,
        timestamp: DateTime<Utc>,
    },
}

impl SupervisorEvent {
    /// Create a TaskStart event
    pub fn task_start(task_id: impl Into<String>, agent_role: impl Into<String>) -> Self {
        Self::TaskStart {
            task_id: task_id.into(),
            agent_role: agent_role.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a ToolCall event
    pub fn tool_call(task_id: impl Into<String>, info: ToolCallInfo) -> Self {
        Self::ToolCall {
            task_id: task_id.into(),
            info,
        }
    }

    /// Create an Error event
    pub fn error(task_id: impl Into<String>, info: ErrorInfo) -> Self {
        Self::Error {
            task_id: task_id.into(),
            info,
        }
    }

    /// Create a ProgressTick event
    pub fn progress_tick(task_id: impl Into<String>, info: ProgressInfo) -> Self {
        Self::ProgressTick {
            task_id: task_id.into(),
            info,
        }
    }

    /// Create a TokenThreshold event
    pub fn token_threshold(task_id: impl Into<String>, tokens_used: u32, threshold: u32) -> Self {
        Self::TokenThreshold {
            task_id: task_id.into(),
            tokens_used,
            threshold,
            timestamp: Utc::now(),
        }
    }

    /// Create a TimeThreshold event
    pub fn time_threshold(
        task_id: impl Into<String>,
        elapsed_minutes: u32,
        threshold_minutes: u32,
    ) -> Self {
        Self::TimeThreshold {
            task_id: task_id.into(),
            elapsed_minutes,
            threshold_minutes,
            timestamp: Utc::now(),
        }
    }

    /// Get the task ID from any event
    pub fn task_id(&self) -> &str {
        match self {
            Self::TaskStart { task_id, .. } => task_id,
            Self::ToolCall { task_id, .. } => task_id,
            Self::Error { task_id, .. } => task_id,
            Self::ProgressTick { task_id, .. } => task_id,
            Self::TokenThreshold { task_id, .. } => task_id,
            Self::TimeThreshold { task_id, .. } => task_id,
        }
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
