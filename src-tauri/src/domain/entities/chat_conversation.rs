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
}

impl fmt::Display for ChatContextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChatContextType::Ideation => write!(f, "ideation"),
            ChatContextType::Task => write!(f, "task"),
            ChatContextType::Project => write!(f, "project"),
            ChatContextType::TaskExecution => write!(f, "task_execution"),
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
        }
    }

    /// Create a new conversation for task execution (worker output)
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
mod tests {
    use super::*;

    #[test]
    fn test_conversation_id_creation() {
        let id1 = ChatConversationId::new();
        let id2 = ChatConversationId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_conversation_id_from_string() {
        let id = ChatConversationId::new();
        let str_id = id.to_string();
        let parsed_id: ChatConversationId = str_id.parse().unwrap();
        assert_eq!(id, parsed_id);
    }

    #[test]
    fn test_context_type_serialization() {
        assert_eq!(ChatContextType::Ideation.to_string(), "ideation");
        assert_eq!(ChatContextType::Task.to_string(), "task");
        assert_eq!(ChatContextType::Project.to_string(), "project");
        assert_eq!(ChatContextType::TaskExecution.to_string(), "task_execution");
    }

    #[test]
    fn test_context_type_parsing() {
        assert_eq!("ideation".parse::<ChatContextType>().unwrap(), ChatContextType::Ideation);
        assert_eq!("task".parse::<ChatContextType>().unwrap(), ChatContextType::Task);
        assert_eq!("project".parse::<ChatContextType>().unwrap(), ChatContextType::Project);
        assert_eq!("task_execution".parse::<ChatContextType>().unwrap(), ChatContextType::TaskExecution);
        assert!("invalid".parse::<ChatContextType>().is_err());
    }

    #[test]
    fn test_new_ideation_conversation() {
        let session_id = IdeationSessionId::new();
        let expected_context_id = session_id.as_str().to_string();
        let conv = ChatConversation::new_ideation(session_id);

        assert_eq!(conv.context_type, ChatContextType::Ideation);
        assert_eq!(conv.context_id, expected_context_id);
        assert_eq!(conv.claude_session_id, None);
        assert_eq!(conv.message_count, 0);
        assert!(!conv.has_claude_session());
    }

    #[test]
    fn test_set_claude_session_id() {
        let session_id = IdeationSessionId::new();
        let mut conv = ChatConversation::new_ideation(session_id);

        conv.set_claude_session_id("550e8400-e29b-41d4-a716-446655440000");
        assert!(conv.has_claude_session());
        assert_eq!(conv.claude_session_id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }

    #[test]
    fn test_set_title() {
        let session_id = IdeationSessionId::new();
        let mut conv = ChatConversation::new_ideation(session_id);

        conv.set_title("Dark mode implementation");
        assert_eq!(conv.display_title(), "Dark mode implementation");
    }

    #[test]
    fn test_display_title_default() {
        let session_id = IdeationSessionId::new();
        let conv = ChatConversation::new_ideation(session_id);
        assert_eq!(conv.display_title(), "Untitled conversation");
    }

    #[test]
    fn test_new_task_execution_conversation() {
        let task_id = TaskId::new();
        let expected_context_id = task_id.as_str().to_string();
        let conv = ChatConversation::new_task_execution(task_id);

        assert_eq!(conv.context_type, ChatContextType::TaskExecution);
        assert_eq!(conv.context_id, expected_context_id);
        assert_eq!(conv.claude_session_id, None);
        assert_eq!(conv.message_count, 0);
        assert!(!conv.has_claude_session());
    }
}
