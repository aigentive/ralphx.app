// Activity event entity for persistent activity stream storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{IdeationSessionId, InternalStatus, TaskId};

/// Unique identifier for an activity event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActivityEventId(pub String);

impl ActivityEventId {
    /// Creates a new ActivityEventId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates an ActivityEventId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ActivityEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ActivityEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of activity event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventType {
    /// Claude's thinking/reasoning block
    Thinking,
    /// Tool invocation
    ToolCall,
    /// Result of a tool execution
    ToolResult,
    /// Text output from agent
    Text,
    /// Error during execution
    Error,
}

impl fmt::Display for ActivityEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityEventType::Thinking => write!(f, "thinking"),
            ActivityEventType::ToolCall => write!(f, "tool_call"),
            ActivityEventType::ToolResult => write!(f, "tool_result"),
            ActivityEventType::Text => write!(f, "text"),
            ActivityEventType::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for ActivityEventType {
    type Err = ParseActivityEventTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "thinking" => Ok(ActivityEventType::Thinking),
            "tool_call" => Ok(ActivityEventType::ToolCall),
            "tool_result" => Ok(ActivityEventType::ToolResult),
            "text" => Ok(ActivityEventType::Text),
            "error" => Ok(ActivityEventType::Error),
            _ => Err(ParseActivityEventTypeError(s.to_string())),
        }
    }
}

/// Error parsing ActivityEventType from string
#[derive(Debug, Clone)]
pub struct ParseActivityEventTypeError(pub String);

impl fmt::Display for ParseActivityEventTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid activity event type: {}", self.0)
    }
}

impl std::error::Error for ParseActivityEventTypeError {}

/// Role of the entity that produced the activity event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEventRole {
    /// AI agent (Claude)
    Agent,
    /// System-generated event
    System,
    /// User-generated event
    User,
}

impl Default for ActivityEventRole {
    fn default() -> Self {
        ActivityEventRole::Agent
    }
}

impl fmt::Display for ActivityEventRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityEventRole::Agent => write!(f, "agent"),
            ActivityEventRole::System => write!(f, "system"),
            ActivityEventRole::User => write!(f, "user"),
        }
    }
}

impl std::str::FromStr for ActivityEventRole {
    type Err = ParseActivityEventRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "agent" => Ok(ActivityEventRole::Agent),
            "system" => Ok(ActivityEventRole::System),
            "user" => Ok(ActivityEventRole::User),
            _ => Err(ParseActivityEventRoleError(s.to_string())),
        }
    }
}

/// Error parsing ActivityEventRole from string
#[derive(Debug, Clone)]
pub struct ParseActivityEventRoleError(pub String);

impl fmt::Display for ParseActivityEventRoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid activity event role: {}", self.0)
    }
}

impl std::error::Error for ParseActivityEventRoleError {}

/// An activity event represents a single event in the activity stream.
///
/// Each event belongs to either a task (via task_id) or an ideation session
/// (via ideation_session_id), but never both. This is enforced at the database
/// level with a CHECK constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// Unique identifier for this event
    pub id: ActivityEventId,
    /// Task this event belongs to (mutually exclusive with ideation_session_id)
    pub task_id: Option<TaskId>,
    /// Ideation session this event belongs to (mutually exclusive with task_id)
    pub ideation_session_id: Option<IdeationSessionId>,
    /// Task internal status snapshot when event occurred
    pub internal_status: Option<InternalStatus>,
    /// Type of event
    pub event_type: ActivityEventType,
    /// Role that produced this event
    pub role: ActivityEventRole,
    /// Event content (text, tool JSON, error message, etc.)
    pub content: String,
    /// Additional metadata as JSON (e.g., tool_use_id for tool results)
    pub metadata: Option<String>,
    /// When this event was created
    pub created_at: DateTime<Utc>,
}

impl ActivityEvent {
    /// Create a new activity event for a task context
    pub fn new_task_event(
        task_id: TaskId,
        event_type: ActivityEventType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: ActivityEventId::new(),
            task_id: Some(task_id),
            ideation_session_id: None,
            internal_status: None,
            event_type,
            role: ActivityEventRole::Agent,
            content: content.into(),
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new activity event for an ideation session context
    pub fn new_session_event(
        session_id: IdeationSessionId,
        event_type: ActivityEventType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: ActivityEventId::new(),
            task_id: None,
            ideation_session_id: Some(session_id),
            internal_status: None,
            event_type,
            role: ActivityEventRole::Agent,
            content: content.into(),
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Set the internal status snapshot
    pub fn with_status(mut self, status: InternalStatus) -> Self {
        self.internal_status = Some(status);
        self
    }

    /// Set the role
    pub fn with_role(mut self, role: ActivityEventRole) -> Self {
        self.role = role;
        self
    }

    /// Set the metadata JSON
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_event_id_generates_unique_ids() {
        let id1 = ActivityEventId::new();
        let id2 = ActivityEventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn activity_event_id_from_string_preserves_value() {
        let id = ActivityEventId::from_string("test-id");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn activity_event_type_display() {
        assert_eq!(ActivityEventType::Thinking.to_string(), "thinking");
        assert_eq!(ActivityEventType::ToolCall.to_string(), "tool_call");
        assert_eq!(ActivityEventType::ToolResult.to_string(), "tool_result");
        assert_eq!(ActivityEventType::Text.to_string(), "text");
        assert_eq!(ActivityEventType::Error.to_string(), "error");
    }

    #[test]
    fn activity_event_type_parsing() {
        assert_eq!("thinking".parse::<ActivityEventType>().unwrap(), ActivityEventType::Thinking);
        assert_eq!("tool_call".parse::<ActivityEventType>().unwrap(), ActivityEventType::ToolCall);
        assert_eq!("tool_result".parse::<ActivityEventType>().unwrap(), ActivityEventType::ToolResult);
        assert_eq!("text".parse::<ActivityEventType>().unwrap(), ActivityEventType::Text);
        assert_eq!("error".parse::<ActivityEventType>().unwrap(), ActivityEventType::Error);
        assert!("invalid".parse::<ActivityEventType>().is_err());
    }

    #[test]
    fn activity_event_role_display() {
        assert_eq!(ActivityEventRole::Agent.to_string(), "agent");
        assert_eq!(ActivityEventRole::System.to_string(), "system");
        assert_eq!(ActivityEventRole::User.to_string(), "user");
    }

    #[test]
    fn activity_event_role_parsing() {
        assert_eq!("agent".parse::<ActivityEventRole>().unwrap(), ActivityEventRole::Agent);
        assert_eq!("system".parse::<ActivityEventRole>().unwrap(), ActivityEventRole::System);
        assert_eq!("user".parse::<ActivityEventRole>().unwrap(), ActivityEventRole::User);
        assert!("invalid".parse::<ActivityEventRole>().is_err());
    }

    #[test]
    fn activity_event_role_default() {
        assert_eq!(ActivityEventRole::default(), ActivityEventRole::Agent);
    }

    #[test]
    fn new_task_event_creates_correct_event() {
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(
            task_id.clone(),
            ActivityEventType::Thinking,
            "test content",
        );

        assert_eq!(event.task_id, Some(task_id));
        assert_eq!(event.ideation_session_id, None);
        assert_eq!(event.event_type, ActivityEventType::Thinking);
        assert_eq!(event.role, ActivityEventRole::Agent);
        assert_eq!(event.content, "test content");
        assert_eq!(event.metadata, None);
        assert_eq!(event.internal_status, None);
    }

    #[test]
    fn new_session_event_creates_correct_event() {
        let session_id = IdeationSessionId::new();
        let event = ActivityEvent::new_session_event(
            session_id.clone(),
            ActivityEventType::Text,
            "session content",
        );

        assert_eq!(event.task_id, None);
        assert_eq!(event.ideation_session_id, Some(session_id));
        assert_eq!(event.event_type, ActivityEventType::Text);
        assert_eq!(event.role, ActivityEventRole::Agent);
        assert_eq!(event.content, "session content");
    }

    #[test]
    fn with_status_sets_status() {
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "content")
            .with_status(InternalStatus::Executing);

        assert_eq!(event.internal_status, Some(InternalStatus::Executing));
    }

    #[test]
    fn with_role_sets_role() {
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Error, "error")
            .with_role(ActivityEventRole::System);

        assert_eq!(event.role, ActivityEventRole::System);
    }

    #[test]
    fn with_metadata_sets_metadata() {
        let task_id = TaskId::new();
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::ToolResult, "result")
            .with_metadata(r#"{"tool_use_id": "abc123"}"#);

        assert_eq!(event.metadata, Some(r#"{"tool_use_id": "abc123"}"#.to_string()));
    }

    #[test]
    fn activity_event_serializes_to_json() {
        let task_id = TaskId::from_string("task-123".to_string());
        let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Thinking, "test");

        let json = serde_json::to_string(&event).expect("Should serialize");
        assert!(json.contains("\"event_type\":\"thinking\""));
        assert!(json.contains("\"role\":\"agent\""));
    }
}
