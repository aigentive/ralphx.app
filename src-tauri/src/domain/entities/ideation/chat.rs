//! Chat message entities

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::domain::entities::{ChatConversationId, ChatMessageId, IdeationSessionId, ProjectId, TaskId};
use super::types::parse_datetime_helper;

// ============================================================================
// ChatMessage and Related Types
// ============================================================================

/// Role of the message sender in a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// Message from the human user
    User,
    /// Message from the Orchestrator AI agent
    Orchestrator,
    /// System message (e.g., session started, context changed)
    System,
    /// Message from the Worker AI agent (task execution output)
    Worker,
    /// Message from the Reviewer AI agent (task review)
    Reviewer,
}

impl Default for MessageRole {
    fn default() -> Self {
        Self::User
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Orchestrator => write!(f, "orchestrator"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Worker => write!(f, "worker"),
            MessageRole::Reviewer => write!(f, "reviewer"),
        }
    }
}

/// Error type for parsing MessageRole from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMessageRoleError {
    pub value: String,
}

impl std::fmt::Display for ParseMessageRoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown message role: '{}'", self.value)
    }
}

impl std::error::Error for ParseMessageRoleError {}

impl FromStr for MessageRole {
    type Err = ParseMessageRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(MessageRole::User),
            "orchestrator" => Ok(MessageRole::Orchestrator),
            "system" => Ok(MessageRole::System),
            "worker" => Ok(MessageRole::Worker),
            _ => Err(ParseMessageRoleError {
                value: s.to_string(),
            }),
        }
    }
}

/// A chat message in an ideation session or project/task context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique identifier for this message
    pub id: ChatMessageId,
    /// Session this message belongs to (for ideation context)
    pub session_id: Option<IdeationSessionId>,
    /// Project this message belongs to (for project context without session)
    pub project_id: Option<ProjectId>,
    /// Task this message is about (for task-specific context)
    pub task_id: Option<TaskId>,
    /// Conversation this message belongs to (for context-aware chat)
    pub conversation_id: Option<ChatConversationId>,
    /// Who sent the message
    pub role: MessageRole,
    /// The message content (supports Markdown)
    pub content: String,
    /// Optional metadata (JSON) for additional context
    pub metadata: Option<String>,
    /// Parent message ID for threading (if applicable)
    pub parent_message_id: Option<ChatMessageId>,
    /// Tool calls made during this message (JSON array)
    /// Stores the tools that Claude called when generating this message
    pub tool_calls: Option<String>,
    /// Content blocks in order (text and tool calls interleaved, JSON array)
    /// When present, this preserves the order of text and tool calls
    pub content_blocks: Option<String>,
    /// When the message was created
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new user message in an ideation session
    pub fn user_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new orchestrator message in an ideation session
    pub fn orchestrator_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::Orchestrator,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new system message in an ideation session
    pub fn system_in_session(session_id: IdeationSessionId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: Some(session_id),
            project_id: None,
            task_id: None,
            conversation_id: None,
            role: MessageRole::System,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new user message in a project context (no session)
    pub fn user_in_project(project_id: ProjectId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: Some(project_id),
            task_id: None,
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new user message about a specific task
    pub fn user_about_task(task_id: TaskId, content: impl Into<String>) -> Self {
        Self {
            id: ChatMessageId::new(),
            session_id: None,
            project_id: None,
            task_id: Some(task_id),
            conversation_id: None,
            role: MessageRole::User,
            content: content.into(),
            metadata: None,
            parent_message_id: None,
            tool_calls: None,
            content_blocks: None,
            created_at: Utc::now(),
        }
    }

    /// Set metadata on this message
    pub fn with_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.metadata = Some(metadata.into());
        self
    }

    /// Set parent message for threading
    pub fn with_parent(mut self, parent_id: ChatMessageId) -> Self {
        self.parent_message_id = Some(parent_id);
        self
    }

    /// Check if this is a user message
    pub fn is_user(&self) -> bool {
        self.role == MessageRole::User
    }

    /// Check if this is an orchestrator message
    pub fn is_orchestrator(&self) -> bool {
        self.role == MessageRole::Orchestrator
    }

    /// Check if this is a system message
    pub fn is_system(&self) -> bool {
        self.role == MessageRole::System
    }

    /// Create from a rusqlite Row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let id: String = row.get("id")?;
        let session_id: Option<String> = row.get("session_id")?;
        let project_id: Option<String> = row.get("project_id")?;
        let task_id: Option<String> = row.get("task_id")?;
        let conversation_id: Option<String> = row.get("conversation_id").ok().flatten();
        let role: String = row.get("role")?;
        let content: String = row.get("content")?;
        let metadata: Option<String> = row.get("metadata")?;
        let parent_message_id: Option<String> = row.get("parent_message_id")?;
        let tool_calls: Option<String> = row.get("tool_calls").ok().flatten();
        let content_blocks: Option<String> = row.get("content_blocks").ok().flatten();
        let created_at_str: String = row.get("created_at")?;

        Ok(Self {
            id: ChatMessageId::from_string(id),
            session_id: session_id.map(IdeationSessionId::from_string),
            project_id: project_id.map(ProjectId::from_string),
            task_id: task_id.map(TaskId::from_string),
            conversation_id: conversation_id.map(ChatConversationId::from_string),
            role: MessageRole::from_str(&role).unwrap_or(MessageRole::User),
            content,
            metadata,
            parent_message_id: parent_message_id.map(ChatMessageId::from_string),
            tool_calls,
            content_blocks,
            created_at: parse_datetime_helper(created_at_str),
        })
    }
}
