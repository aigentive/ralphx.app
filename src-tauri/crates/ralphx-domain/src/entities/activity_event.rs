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
    /// System-generated event (e.g., merge pipeline steps)
    System,
}

impl fmt::Display for ActivityEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivityEventType::Thinking => write!(f, "thinking"),
            ActivityEventType::ToolCall => write!(f, "tool_call"),
            ActivityEventType::ToolResult => write!(f, "tool_result"),
            ActivityEventType::Text => write!(f, "text"),
            ActivityEventType::Error => write!(f, "error"),
            ActivityEventType::System => write!(f, "system"),
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
            "system" => Ok(ActivityEventType::System),
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

    /// Deserializes an ActivityEvent from a SQLite row
    ///
    /// Expected column order: id, task_id, ideation_session_id, internal_status,
    /// event_type, role, content, metadata, created_at
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let id: String = row.get(0)?;
        let task_id: Option<String> = row.get(1)?;
        let ideation_session_id: Option<String> = row.get(2)?;
        let internal_status_str: Option<String> = row.get(3)?;
        let event_type_str: String = row.get(4)?;
        let role_str: String = row.get(5)?;
        let content: String = row.get(6)?;
        let metadata: Option<String> = row.get(7)?;
        let created_at_str: String = row.get(8)?;

        let event_type: ActivityEventType =
            event_type_str
                .parse()
                .map_err(|e: ParseActivityEventTypeError| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.0)),
                    )
                })?;

        let role: ActivityEventRole =
            role_str.parse().map_err(|e: ParseActivityEventRoleError| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.0)),
                )
            })?;

        let internal_status = internal_status_str
            .map(|s| {
                s.parse::<InternalStatus>().map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                })
            })
            .transpose()?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?
            .with_timezone(&Utc);

        Ok(Self {
            id: ActivityEventId::from_string(id),
            task_id: task_id.map(TaskId::from_string),
            ideation_session_id: ideation_session_id.map(IdeationSessionId::from_string),
            internal_status,
            event_type,
            role,
            content,
            metadata,
            created_at,
        })
    }
}

#[cfg(test)]
#[path = "activity_event_tests.rs"]
mod tests;
