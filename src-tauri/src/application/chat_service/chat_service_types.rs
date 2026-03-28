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
    /// Agent turn completed event (interactive mode: turn done but process still alive)
    pub const AGENT_TURN_COMPLETED: &str = "agent:turn_completed";
    /// Agent message created event
    pub const AGENT_MESSAGE_CREATED: &str = "agent:message_created";
    /// Agent error event
    pub const AGENT_ERROR: &str = "agent:error";
    /// Agent queue sent event
    pub const AGENT_QUEUE_SENT: &str = "agent:queue_sent";
    /// Agent message queued event (message entered the queue, agent already running)
    pub const AGENT_MESSAGE_QUEUED: &str = "agent:message_queued";
    /// Activity stream message event (for execution bar)
    pub const AGENT_MESSAGE: &str = "agent:message";
    /// Agent task (subagent) started event
    pub const AGENT_TASK_STARTED: &str = "agent:task_started";
    /// Agent task (subagent) completed event
    pub const AGENT_TASK_COMPLETED: &str = "agent:task_completed";
    /// Agent hook event (started/completed/block)
    pub const AGENT_HOOK: &str = "agent:hook";

    // Team events (agent teams collaboration)
    /// Team created event
    pub const TEAM_CREATED: &str = "team:created";
    /// Teammate spawned event
    pub const TEAM_TEAMMATE_SPAWNED: &str = "team:teammate_spawned";
    /// Teammate idle event
    pub const TEAM_TEAMMATE_IDLE: &str = "team:teammate_idle";
    /// Teammate shutdown event
    pub const TEAM_TEAMMATE_SHUTDOWN: &str = "team:teammate_shutdown";
    /// Team message event (teammate → teammate or user → team)
    pub const TEAM_MESSAGE: &str = "team:message";
    /// Team disbanded event
    pub const TEAM_DISBANDED: &str = "team:disbanded";
    /// Team cost update event
    pub const TEAM_COST_UPDATE: &str = "team:cost_update";
    /// Team artifact created event
    pub const TEAM_ARTIFACT_CREATED: &str = "team:artifact_created";
}

// ============================================================================
// Types
// ============================================================================

/// Context indicating who initiated a `send_message` call.
///
/// Controls whether a `SpawnFailed` error on an ideation context is caught-and-persisted
/// (UserInitiated) or propagated directly (DrainService).  The distinction prevents an
/// infinite drain loop: if the drain service already called `send_message`, capacity is
/// still full — persisting the prompt again and returning `Ok` would cause the drain to
/// re-claim the same session on the next tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SendCallerContext {
    /// User-initiated send (frontend / HTTP handler).
    /// On ideation capacity full → persist message as `pending_initial_prompt` and return
    /// `Ok(SendResult { queued_as_pending: true })`.
    #[default]
    UserInitiated,
    /// Drain-service-initiated send.
    /// On ideation capacity full → return `Err(SpawnFailed)` so the drain service breaks cleanly.
    DrainService,
}

/// Result from sending a message (returns immediately while processing continues in background)
#[derive(Debug, Clone, Serialize, Default)]
pub struct SendResult {
    /// The conversation ID for this chat
    pub conversation_id: String,
    /// The agent run ID tracking this execution
    pub agent_run_id: String,
    /// Whether this is a new conversation (first message)
    pub is_new_conversation: bool,
    /// Whether the message was queued (Gate 2 blocked — agent already running)
    pub was_queued: bool,
    /// The queued message ID if was_queued is true
    pub queued_message_id: Option<String>,
    /// Whether the message was persisted as `pending_initial_prompt` because ideation
    /// capacity was full.  Distinct from `was_queued` (which means an agent is already running
    /// for the context and the message entered the backend queue).
    pub queued_as_pending: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_chain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
}

/// Payload for agent:chunk event
#[derive(Debug, Clone, Serialize)]
pub struct AgentChunkPayload {
    pub text: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub seq: u64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    pub seq: u64,
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
    /// Server-side DB timestamp for the message (RFC3339). Used by frontend to avoid clock skew.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Optional JSON metadata string attached to the message (e.g. recovery_context).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// Payload for agent:run_completed event
#[derive(Debug, Clone, Serialize)]
pub struct AgentRunCompletedPayload {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub claude_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_chain_id: Option<String>,
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

/// Payload for agent:task_started event
#[derive(Debug, Clone, Serialize)]
pub struct AgentTaskStartedPayload {
    pub tool_use_id: String,
    /// Tool name that triggered this: "Task" or "Agent"
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_name: Option<String>,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub seq: u64,
}

/// Payload for agent:task_completed event
#[derive(Debug, Clone, Serialize)]
pub struct AgentTaskCompletedPayload {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tool_use_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_name: Option<String>,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub seq: u64,
}

/// Payload for agent:queue_sent event
#[derive(Debug, Clone, Serialize)]
pub struct AgentQueueSentPayload {
    pub message_id: String,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:message_queued event (message entered the queue at Gate 2)
#[derive(Debug, Clone, Serialize)]
pub struct AgentMessageQueuedPayload {
    pub message_id: String,
    pub content: String,
    pub context_type: String,
    pub context_id: String,
    pub created_at: String,
}

/// Payload for agent:conversation_created event
#[derive(Debug, Clone, Serialize)]
pub struct AgentConversationCreatedPayload {
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for agent:hook event (discriminated by `hook_type`)
#[derive(Debug, Clone, Serialize)]
pub struct AgentHookPayload {
    /// Discriminator: "started", "completed", or "block"
    #[serde(rename = "type")]
    pub hook_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub conversation_id: String,
    pub context_type: String,
    pub context_id: String,
    pub timestamp: i64,
}

// ============================================================================
// Team Event Payloads
// ============================================================================

/// Payload for team:created event
#[derive(Debug, Clone, Serialize)]
pub struct TeamCreatedPayload {
    pub team_name: String,
    pub context_id: String,
    pub context_type: String,
}

/// Payload for team:teammate_spawned event
#[derive(Debug, Clone, Serialize)]
pub struct TeamTeammateSpawnedPayload {
    pub team_name: String,
    pub teammate_name: String,
    pub color: String,
    pub model: String,
    pub role: String,
    pub context_type: String,
    pub context_id: String,
    /// Conversation ID for this teammate's persisted chat history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Payload for team:teammate_idle event
#[derive(Debug, Clone, Serialize)]
pub struct TeamTeammateIdlePayload {
    pub team_name: String,
    pub teammate_name: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for team:teammate_shutdown event
#[derive(Debug, Clone, Serialize)]
pub struct TeamTeammateShutdownPayload {
    pub team_name: String,
    pub teammate_name: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for team:message event
#[derive(Debug, Clone, Serialize)]
pub struct TeamMessagePayload {
    pub team_name: String,
    pub message_id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub message_type: String,
    pub timestamp: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for team:disbanded event
#[derive(Debug, Clone, Serialize)]
pub struct TeamDisbandedPayload {
    pub team_name: String,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for team:cost_update event
#[derive(Debug, Clone, Serialize)]
pub struct TeamCostUpdatePayload {
    pub team_name: String,
    pub teammate_name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_usd: f64,
    pub context_type: String,
    pub context_id: String,
}

/// Payload for team:artifact_created event
#[derive(Debug, Clone, Serialize)]
pub struct TeamArtifactCreatedPayload {
    pub artifact_id: String,
    pub session_id: String,
    pub artifact_type: String,
    pub title: String,
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
