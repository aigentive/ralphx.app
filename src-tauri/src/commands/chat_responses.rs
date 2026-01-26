// Shared response types for chat-related commands
// Used by both ideation_commands and context_chat_commands

use serde::Serialize;

use crate::domain::entities::ChatMessage;

/// Response for ChatMessage
/// Shared across all chat-related endpoints for API consistency
#[derive(Debug, Serialize, Clone)]
pub struct ChatMessageResponse {
    pub id: String,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub conversation_id: Option<String>,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
    pub parent_message_id: Option<String>,
    pub tool_calls: Option<String>,
    pub content_blocks: Option<String>,
    pub created_at: String,
}

impl From<ChatMessage> for ChatMessageResponse {
    fn from(message: ChatMessage) -> Self {
        Self {
            id: message.id.as_str().to_string(),
            session_id: message.session_id.map(|id| id.as_str().to_string()),
            project_id: message.project_id.map(|id| id.as_str().to_string()),
            task_id: message.task_id.map(|id| id.as_str().to_string()),
            conversation_id: message.conversation_id.map(|id| id.as_str().to_string()),
            role: message.role.to_string(),
            content: message.content,
            metadata: message.metadata,
            parent_message_id: message.parent_message_id.map(|id| id.as_str().to_string()),
            tool_calls: message.tool_calls,
            content_blocks: message.content_blocks,
            created_at: message.created_at.to_rfc3339(),
        }
    }
}
