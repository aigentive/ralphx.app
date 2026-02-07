// Chat Service Types and Event Payloads
//
// Extracted from chat_service.rs to improve modularity and reduce file size.

use serde::Serialize;

use crate::domain::entities::{ChatConversation, ChatMessage};

// ============================================================================
// Event Name Constants
// ============================================================================
// Unified event names for all chat-related events.
// Use these constants instead of hardcoding event strings.

/// Unified events (new API - includes context_type in payload)
pub mod events {
    /// Agent text chunk event
    pub const AGENT_CHUNK: &str = "agent:chunk";
    /// Agent tool call event
    pub const AGENT_TOOL_CALL: &str = "agent:tool_call";
    /// Agent run started event
    pub const AGENT_RUN_STARTED: &str = "agent:run_started";
    /// Agent run completed event
    pub const AGENT_RUN_COMPLETED: &str = "agent:run_completed";
    /// Agent message created event
    pub const AGENT_MESSAGE_CREATED: &str = "agent:message_created";
    /// Agent error event
    pub const AGENT_ERROR: &str = "agent:error";
    /// Agent queue sent event
    pub const AGENT_QUEUE_SENT: &str = "agent:queue_sent";
    /// Activity stream message event (for execution bar)
    pub const AGENT_MESSAGE: &str = "agent:message";
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_context: Option<serde_json::Value>,
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
