use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{IdeationSessionId, ProjectId, TaskId};

/// Unique identifier for a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatConversationId(Uuid);

impl ChatConversationId {
    /// Create a new random conversation ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Get as string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Create from string (for database deserialization)
    pub fn from_string(s: impl Into<String>) -> Self {
        let s = s.into();
        Self(Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::nil()))
    }
}

impl Default for ChatConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ChatConversationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ChatConversationId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ChatConversationId> for String {
    fn from(id: ChatConversationId) -> Self {
        id.0.to_string()
    }
}

impl std::str::FromStr for ChatConversationId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Type of context a conversation is associated with
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatContextType {
    /// Ideation session context
    Ideation,
    /// Task-specific context
    Task,
    /// Project-level context
    Project,
    /// Task execution context (worker output)
    #[serde(rename = "task_execution")]
    TaskExecution,
    /// Task review context (AI reviewer)
    Review,
    /// Merge conflict resolution context (merger agent)
    Merge,
}

impl fmt::Display for ChatContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChatContextType::Ideation => write!(f, "ideation"),
            ChatContextType::Task => write!(f, "task"),
            ChatContextType::Project => write!(f, "project"),
            ChatContextType::TaskExecution => write!(f, "task_execution"),
            ChatContextType::Review => write!(f, "review"),
            ChatContextType::Merge => write!(f, "merge"),
        }
    }
}

impl std::str::FromStr for ChatContextType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ideation" => Ok(ChatContextType::Ideation),
            "task" => Ok(ChatContextType::Task),
            "project" => Ok(ChatContextType::Project),
            "task_execution" => Ok(ChatContextType::TaskExecution),
            "review" => Ok(ChatContextType::Review),
            "merge" => Ok(ChatContextType::Merge),
            _ => Err(format!("Invalid context type: {}", s)),
        }
    }
}

/// A chat conversation linked to a context (ideation session, task, or project)
///
/// Multiple conversations can exist per context to support conversation history.
/// Each conversation tracks its own Claude CLI session ID for --resume support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversation {
    /// Unique identifier for this conversation
    pub id: ChatConversationId,
    /// Type of context this conversation is for
    pub context_type: ChatContextType,
    /// ID of the context (session_id, task_id, or project_id)
    pub context_id: String,
    /// Claude CLI session UUID for --resume flag
    /// This is captured from Claude's response and enables conversation continuity
    pub claude_session_id: Option<String>,
    /// Auto-generated or user-set title for this conversation
    pub title: Option<String>,
    /// Number of messages in this conversation
    pub message_count: i64,
    /// Timestamp of the last message
    pub last_message_at: Option<DateTime<Utc>>,
    /// When this conversation was created
    pub created_at: DateTime<Utc>,
    /// When this conversation was last updated
    pub updated_at: DateTime<Utc>,
    /// ID of the prior execution's conversation (for TaskExecution re-runs only).
    /// Enables UI navigation between execution generations.
    pub parent_conversation_id: Option<String>,
}

impl ChatConversation {
    /// Create a new conversation for an ideation session
    pub fn new_ideation(session_id: IdeationSessionId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::Ideation,
            context_id: session_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Create a new conversation for a task
    pub fn new_task(task_id: TaskId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::Task,
            context_id: task_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Create a new conversation for a project
    pub fn new_project(project_id: ProjectId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::Project,
            context_id: project_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Create a new conversation for task execution (worker output).
    /// Pass `parent_id` when re-executing a task to link to the prior run's conversation.
    pub fn new_task_execution(task_id: TaskId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::TaskExecution,
            context_id: task_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Create a new conversation for task review (reviewer agent)
    pub fn new_review(task_id: TaskId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::Review,
            context_id: task_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Create a new conversation for merge conflict resolution (merger agent)
    pub fn new_merge(task_id: TaskId) -> Self {
        let now = Utc::now();
        Self {
            id: ChatConversationId::new(),
            context_type: ChatContextType::Merge,
            context_id: task_id.as_str().to_string(),
            claude_session_id: None,
            title: None,
            message_count: 0,
            last_message_at: None,
            created_at: now,
            updated_at: now,
            parent_conversation_id: None,
        }
    }

    /// Update the Claude session ID (after first message in conversation)
    pub fn set_claude_session_id(&mut self, session_id: impl Into<String>) {
        self.claude_session_id = Some(session_id.into());
        self.updated_at = Utc::now();
    }

    /// Set or update the conversation title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
        self.updated_at = Utc::now();
    }

    /// Check if this conversation has a Claude session (can use --resume)
    pub fn has_claude_session(&self) -> bool {
        self.claude_session_id.is_some()
    }

    /// Get a display title for this conversation
    pub fn display_title(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| "Untitled conversation".to_string())
    }
}

#[cfg(test)]
#[path = "chat_conversation_tests.rs"]
mod tests;
