// Chat Service Types and Event Payloads
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use serde::Serialize;

use crate::domain::entities::{ChatConversation, ChatMessage};

// ============================================================================
// Types
// ============================================================================

/// Result from sending a message (returns immediately while processing continues in background)
#[derive(Debug, Clone, Serialize)]
pub struct SendResult {
    /// The conversation ID for this chat
    pub conversation_id: String,
    /// The agent run ID tracking this execution
    pub agent_run_id: String,
    /// Whether this is a new conversation (first message)
    pub is_new_conversation: bool,
}

/// A conversation with its messages
#[derive(Debug, Clone)]
pub struct ChatConversationWithMessages {
    pub conversation: ChatConversation,
    pub messages: Vec<ChatMessage>,
}

// ============================================================================
// Unified Event Payloads (agent:* namespace)
// ============================================================================

/// Payload for agent:run_started event
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunStartedPayload {
    pub run_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:chunk event
#[derive(Debug, Clone, Serialize)]
pub struct AgentChunkPayload {
    pub text: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:tool_call event
#[derive(Debug, Clone, Serialize)]
pub struct AgentToolCallPayload {
    pub tool_name: String,
    pub tool_id: Option<String>,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:message_created event
#[derive(Debug, Clone, Serialize)]
pub struct AgentMessageCreatedPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub role: String,
    pub content: String,
}

/// Payload for agent:run_completed event
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunCompletedPayload {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub claude_session_id: Option<String>,
}

/// Payload for agent:error event
#[derive(Debug, Clone, Serialize)]
pub struct AgentErrorPayload {
    pub conversation_id: Option<String>,
    pub context_type: String,
    pub context_id: String,
    pub error: String,
    pub stderr: Option<String>,
}

/// Payload for agent:queue_sent event
#[derive(Debug, Clone, Serialize)]
pub struct AgentQueueSentPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, Clone)]
pub enum ChatServiceError {
    AgentNotAvailable(String),
    SpawnFailed(String),
    CommunicationFailed(String),
    ParseError(String),
    ContextNotFound(String),
    ConversationNotFound(String),
    RepositoryError(String),
    AgentRunFailed(String),
}

impl std::fmt::Display for ChatServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotAvailable(msg) => write!(f, "Agent not available: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Failed to spawn agent: {}", msg),
            Self::CommunicationFailed(msg) => write!(f, "Communication failed: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ContextNotFound(msg) => write!(f, "Context not found: {}", msg),
            Self::ConversationNotFound(msg) => write!(f, "Conversation not found: {}", msg),
            Self::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            Self::AgentRunFailed(msg) => write!(f, "Agent run failed: {}", msg),
        }
    }
}

impl std::error::Error for ChatServiceError {}
