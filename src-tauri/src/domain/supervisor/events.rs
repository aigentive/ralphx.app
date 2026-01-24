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

    pub fn failed(tool_name: impl Into<String>, arguments: impl Into<String>, error: impl Into<String>) -> Self {
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
    ToolCall {
        task_id: String,
        info: ToolCallInfo,
    },
    /// An error occurred
    Error {
        task_id: String,
        info: ErrorInfo,
    },
    /// Progress tick (periodic check)
    ProgressTick {
        task_id: String,
        info: ProgressInfo,
    },
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
    pub fn time_threshold(task_id: impl Into<String>, elapsed_minutes: u32, threshold_minutes: u32) -> Self {
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
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_info_new() {
        let info = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
        assert_eq!(info.tool_name, "Write");
        assert!(info.success);
        assert!(info.error.is_none());
    }

    #[test]
    fn test_tool_call_info_failed() {
        let info = ToolCallInfo::failed("Write", r#"{"path": "test.txt"}"#, "Permission denied");
        assert!(!info.success);
        assert_eq!(info.error, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_tool_call_is_similar() {
        let info1 = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
        let info2 = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
        let info3 = ToolCallInfo::new("Write", r#"{"path": "other.txt"}"#);
        let info4 = ToolCallInfo::new("Read", r#"{"path": "test.txt"}"#);

        assert!(info1.is_similar_to(&info2));
        assert!(!info1.is_similar_to(&info3));
        assert!(!info1.is_similar_to(&info4));
    }

    #[test]
    fn test_error_info_new() {
        let info = ErrorInfo::new("File not found", "Read");
        assert!(info.recoverable);
    }

    #[test]
    fn test_error_info_fatal() {
        let info = ErrorInfo::fatal("System crash", "Kernel");
        assert!(!info.recoverable);
    }

    #[test]
    fn test_progress_info_new() {
        let info = ProgressInfo::new();
        assert!(!info.has_progress());
    }

    #[test]
    fn test_progress_info_has_progress() {
        let mut info = ProgressInfo::new();
        assert!(!info.has_progress());

        info.has_file_changes = true;
        assert!(info.has_progress());

        info.has_file_changes = false;
        info.has_new_commits = true;
        assert!(info.has_progress());

        info.has_new_commits = false;
        info.files_modified = 1;
        assert!(info.has_progress());
    }

    #[test]
    fn test_supervisor_event_task_start() {
        let event = SupervisorEvent::task_start("task-123", "worker");
        assert_eq!(event.task_id(), "task-123");
        if let SupervisorEvent::TaskStart { agent_role, .. } = &event {
            assert_eq!(agent_role, "worker");
        } else {
            panic!("Expected TaskStart event");
        }
    }

    #[test]
    fn test_supervisor_event_tool_call() {
        let info = ToolCallInfo::new("Write", "{}");
        let event = SupervisorEvent::tool_call("task-123", info.clone());
        assert_eq!(event.task_id(), "task-123");
        if let SupervisorEvent::ToolCall { info: event_info, .. } = &event {
            assert_eq!(event_info.tool_name, "Write");
        } else {
            panic!("Expected ToolCall event");
        }
    }

    #[test]
    fn test_supervisor_event_error() {
        let info = ErrorInfo::new("Error message", "Source");
        let event = SupervisorEvent::error("task-123", info);
        assert_eq!(event.task_id(), "task-123");
    }

    #[test]
    fn test_supervisor_event_progress_tick() {
        let info = ProgressInfo::new();
        let event = SupervisorEvent::progress_tick("task-123", info);
        assert_eq!(event.task_id(), "task-123");
    }

    #[test]
    fn test_supervisor_event_token_threshold() {
        let event = SupervisorEvent::token_threshold("task-123", 60000, 50000);
        assert_eq!(event.task_id(), "task-123");
        if let SupervisorEvent::TokenThreshold { tokens_used, threshold, .. } = &event {
            assert_eq!(*tokens_used, 60000);
            assert_eq!(*threshold, 50000);
        } else {
            panic!("Expected TokenThreshold event");
        }
    }

    #[test]
    fn test_supervisor_event_time_threshold() {
        let event = SupervisorEvent::time_threshold("task-123", 15, 10);
        assert_eq!(event.task_id(), "task-123");
        if let SupervisorEvent::TimeThreshold { elapsed_minutes, threshold_minutes, .. } = &event {
            assert_eq!(*elapsed_minutes, 15);
            assert_eq!(*threshold_minutes, 10);
        } else {
            panic!("Expected TimeThreshold event");
        }
    }

    #[test]
    fn test_supervisor_event_serialize() {
        let event = SupervisorEvent::task_start("task-123", "worker");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"task_start\""));
        assert!(json.contains("\"task_id\":\"task-123\""));
    }

    #[test]
    fn test_supervisor_event_deserialize() {
        let json = r#"{
            "type": "task_start",
            "task_id": "task-123",
            "agent_role": "worker",
            "timestamp": "2026-01-24T10:00:00Z"
        }"#;
        let event: SupervisorEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.task_id(), "task-123");
    }

    #[test]
    fn test_supervisor_event_roundtrip() {
        let original = SupervisorEvent::token_threshold("task-456", 75000, 50000);
        let json = serde_json::to_string(&original).unwrap();
        let restored: SupervisorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(original.task_id(), restored.task_id());
    }
}
