// Unified Tauri commands for all chat contexts
//
// These commands use the unified ChatService that consolidates
// OrchestratorService and ExecutionChatService functionality.
//
// Event namespace: agent:* (instead of chat:*/execution:*)
// - agent:run_started - Agent begins processing
// - agent:chunk - Streaming text chunk
// - agent:tool_call - Tool invocation
// - agent:message_created - Message persisted
// - agent:run_completed - Agent finished successfully
// - agent:error - Agent failed
// - agent:queue_sent - Queued message sent

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::{AppState, ChatService, ClaudeChatService, SendResult};
use crate::commands::ExecutionState;
use crate::domain::entities::{ChatContextType, ChatConversation};
use crate::domain::services::QueuedMessage;

// ============================================================================
// Request/Response types
// ============================================================================

/// Input for send_agent_message command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentMessageInput {
    pub context_type: String,
    pub context_id: String,
    pub content: String,
}

/// Response from send_agent_message command
#[derive(Debug, Serialize)]
pub struct SendAgentMessageResponse {
    pub conversation_id: String,
    pub agent_run_id: String,
    pub is_new_conversation: bool,
}

impl From<SendResult> for SendAgentMessageResponse {
    fn from(result: SendResult) -> Self {
        Self {
            conversation_id: result.conversation_id,
            agent_run_id: result.agent_run_id,
            is_new_conversation: result.is_new_conversation,
        }
    }
}

/// Input for queue_agent_message command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueAgentMessageInput {
    pub context_type: String,
    pub context_id: String,
    pub content: String,
    /// Client-provided ID for tracking (optional, allows frontend/backend to use same ID)
    pub client_id: Option<String>,
}

/// Response for queued message
#[derive(Debug, Serialize)]
pub struct QueuedMessageResponse {
    pub id: String,
    pub content: String,
    pub created_at: String,
    pub is_editing: bool,
}

impl From<QueuedMessage> for QueuedMessageResponse {
    fn from(msg: QueuedMessage) -> Self {
        Self {
            id: msg.id,
            content: msg.content,
            created_at: msg.created_at,
            is_editing: msg.is_editing,
        }
    }
}

/// Response for conversation listing
#[derive(Debug, Serialize)]
pub struct AgentConversationResponse {
    pub id: String,
    pub context_type: String,
    pub context_id: String,
    pub claude_session_id: Option<String>,
    pub title: Option<String>,
    pub message_count: i64,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ChatConversation> for AgentConversationResponse {
    fn from(c: ChatConversation) -> Self {
        Self {
            id: c.id.as_str(),
            context_type: c.context_type.to_string(),
            context_id: c.context_id,
            claude_session_id: c.claude_session_id,
            title: c.title,
            message_count: c.message_count,
            last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

/// Response for conversation with messages
#[derive(Debug, Serialize)]
pub struct AgentConversationWithMessagesResponse {
    pub conversation: AgentConversationResponse,
    pub messages: Vec<AgentMessageResponse>,
}

/// Response for a single message
#[derive(Debug, Serialize)]
pub struct AgentMessageResponse {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub content_blocks: Option<serde_json::Value>,
    pub created_at: String,
}

/// Response for agent run status
#[derive(Debug, Serialize)]
pub struct AgentRunStatusResponse {
    pub id: String,
    pub conversation_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

// ============================================================================
// Helper to create ChatService
// ============================================================================

fn create_chat_service(
    state: &AppState,
    app_handle: tauri::AppHandle,
    execution_state: &Arc<ExecutionState>,
) -> ClaudeChatService<tauri::Wry> {
    ClaudeChatService::new(
        state.chat_message_repo.clone(),
        state.chat_conversation_repo.clone(),
        state.agent_run_repo.clone(),
        state.project_repo.clone(),
        state.task_repo.clone(),
        state.ideation_session_repo.clone(),
        state.activity_event_repo.clone(),
        state.message_queue.clone(),
        state.running_agent_registry.clone(),
    )
    .with_app_handle(app_handle)
    .with_execution_state(Arc::clone(execution_state))
}

/// Parse context type string to enum
fn parse_context_type(context_type: &str) -> Result<ChatContextType, String> {
    context_type
        .parse()
        .map_err(|e: String| format!("Invalid context type '{}': {}", context_type, e))
}

// ============================================================================
// Commands
// ============================================================================

/// Send a message to an agent in any context
///
/// Returns immediately with conversation_id and agent_run_id.
/// Processing happens in background with events emitted via Tauri.
///
/// Events emitted:
/// - agent:run_started - When agent begins
/// - agent:chunk - Streaming text chunks
/// - agent:tool_call - Tool invocations
/// - agent:message_created - When messages are persisted
/// - agent:run_completed - When agent finishes
/// - agent:error - On failure
#[tauri::command]
pub async fn send_agent_message(
    input: SendAgentMessageInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<SendAgentMessageResponse, String> {
    let context_type = parse_context_type(&input.context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    if !service.is_available().await {
        return Err(
            "Claude CLI is not available. Please ensure 'claude' is installed and in your PATH."
                .to_string(),
        );
    }

    service
        .send_message(context_type, &input.context_id, &input.content)
        .await
        .map(SendAgentMessageResponse::from)
        .map_err(|e| e.to_string())
}

/// Queue a message to be sent when the current agent run completes
///
/// The message is held in the backend queue and automatically sent
/// via --resume when the current run finishes.
///
/// If `client_id` is provided, that ID will be used for the message,
/// allowing frontend and backend to use the same ID for tracking.
#[tauri::command]
pub async fn queue_agent_message(
    input: QueueAgentMessageInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<QueuedMessageResponse, String> {
    let context_type = parse_context_type(&input.context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    service
        .queue_message(
            context_type,
            &input.context_id,
            &input.content,
            input.client_id.as_deref(),
        )
        .await
        .map(QueuedMessageResponse::from)
        .map_err(|e| e.to_string())
}

/// Get all queued messages for a context
#[tauri::command]
pub async fn get_queued_agent_messages(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<Vec<QueuedMessageResponse>, String> {
    let context_type = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    service
        .get_queued_messages(context_type, &context_id)
        .await
        .map(|msgs| msgs.into_iter().map(QueuedMessageResponse::from).collect())
        .map_err(|e| e.to_string())
}

/// Delete a queued message before it's sent
#[tauri::command]
pub async fn delete_queued_agent_message(
    context_type: String,
    context_id: String,
    message_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let context_type = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    service
        .delete_queued_message(context_type, &context_id, &message_id)
        .await
        .map_err(|e| e.to_string())
}

/// List all conversations for a context
#[tauri::command]
pub async fn list_agent_conversations(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<Vec<AgentConversationResponse>, String> {
    let context_type_enum = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    service
        .list_conversations(context_type_enum, &context_id)
        .await
        .map(|convs| {
            convs
                .into_iter()
                .map(|c| AgentConversationResponse {
                    id: c.id.as_str().to_string(),
                    context_type: c.context_type.to_string(),
                    context_id: c.context_id,
                    claude_session_id: c.claude_session_id,
                    title: c.title,
                    message_count: c.message_count,
                    last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
                    created_at: c.created_at.to_rfc3339(),
                    updated_at: c.updated_at.to_rfc3339(),
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get a conversation with all its messages
#[tauri::command]
pub async fn get_agent_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<Option<AgentConversationWithMessagesResponse>, String> {
    use crate::domain::entities::ChatConversationId;

    let conversation_id = ChatConversationId::from_string(&conversation_id);

    let service = create_chat_service(&state, app, &execution_state);

    service
        .get_conversation_with_messages(&conversation_id)
        .await
        .map(|opt| {
            opt.map(|cwm| AgentConversationWithMessagesResponse {
                conversation: AgentConversationResponse {
                    id: cwm.conversation.id.as_str().to_string(),
                    context_type: cwm.conversation.context_type.to_string(),
                    context_id: cwm.conversation.context_id,
                    claude_session_id: cwm.conversation.claude_session_id,
                    title: cwm.conversation.title,
                    message_count: cwm.conversation.message_count,
                    last_message_at: cwm.conversation.last_message_at.map(|dt| dt.to_rfc3339()),
                    created_at: cwm.conversation.created_at.to_rfc3339(),
                    updated_at: cwm.conversation.updated_at.to_rfc3339(),
                },
                messages: cwm
                    .messages
                    .into_iter()
                    .map(|m| AgentMessageResponse {
                        id: m.id.as_str().to_string(),
                        role: m.role.to_string(),
                        content: m.content,
                        tool_calls: m.tool_calls.and_then(|tc| serde_json::from_str(&tc).ok()),
                        content_blocks: m.content_blocks.and_then(|cb| serde_json::from_str(&cb).ok()),
                        created_at: m.created_at.to_rfc3339(),
                    })
                    .collect(),
            })
        })
        .map_err(|e| e.to_string())
}

/// Get the active agent run for a conversation
#[tauri::command]
pub async fn get_agent_run_status_unified(
    conversation_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<Option<AgentRunStatusResponse>, String> {
    use crate::domain::entities::ChatConversationId;

    let conversation_id = ChatConversationId::from_string(&conversation_id);

    let service = create_chat_service(&state, app, &execution_state);

    service
        .get_active_run(&conversation_id)
        .await
        .map(|opt| {
            opt.map(|run| AgentRunStatusResponse {
                id: run.id.as_str().to_string(),
                conversation_id: run.conversation_id.as_str().to_string(),
                status: run.status.to_string(),
                started_at: run.started_at.to_rfc3339(),
                completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
                error_message: run.error_message,
            })
        })
        .map_err(|e| e.to_string())
}

/// Check if the chat service is available
#[tauri::command]
pub async fn is_chat_service_available(
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let service = create_chat_service(&state, app, &execution_state);
    Ok(service.is_available().await)
}

/// Stop a running agent for a context
///
/// Sends SIGTERM to the running agent process and emits agent:stopped event.
/// Returns true if an agent was stopped, false if no agent was running.
///
/// Events emitted:
/// - agent:stopped - When agent is terminated
/// - agent:run_completed - So frontend knows agent is no longer running
#[tauri::command]
pub async fn stop_agent(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let context_type = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    service
        .stop_agent(context_type, &context_id)
        .await
        .map_err(|e| e.to_string())
}

/// Check if an agent is running for a context
#[tauri::command]
pub async fn is_agent_running(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let context_type = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state);

    Ok(service.is_agent_running(context_type, &context_id).await)
}

/// Input for create_agent_conversation command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentConversationInput {
    pub context_type: String,
    pub context_id: String,
}

/// Create a new conversation for a context
#[tauri::command]
pub async fn create_agent_conversation(
    input: CreateAgentConversationInput,
    state: State<'_, AppState>,
) -> Result<AgentConversationResponse, String> {
    use crate::domain::entities::{
        ChatConversation, IdeationSessionId, ProjectId, TaskId,
    };

    let context_type = parse_context_type(&input.context_type)?;

    let conversation = match context_type {
        ChatContextType::Ideation => {
            ChatConversation::new_ideation(IdeationSessionId::from_string(&input.context_id))
        }
        ChatContextType::Task => {
            ChatConversation::new_task(TaskId::from_string(input.context_id.clone()))
        }
        ChatContextType::Project => {
            ChatConversation::new_project(ProjectId::from_string(input.context_id.clone()))
        }
        ChatContextType::TaskExecution => {
            ChatConversation::new_task_execution(TaskId::from_string(input.context_id.clone()))
        }
        ChatContextType::Review => {
            ChatConversation::new_review(TaskId::from_string(input.context_id.clone()))
        }
    };

    state
        .chat_conversation_repo
        .create(conversation)
        .await
        .map(AgentConversationResponse::from)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_context_type() {
        assert!(matches!(
            parse_context_type("ideation"),
            Ok(ChatContextType::Ideation)
        ));
        assert!(matches!(
            parse_context_type("task"),
            Ok(ChatContextType::Task)
        ));
        assert!(matches!(
            parse_context_type("project"),
            Ok(ChatContextType::Project)
        ));
        assert!(matches!(
            parse_context_type("task_execution"),
            Ok(ChatContextType::TaskExecution)
        ));
        assert!(parse_context_type("invalid").is_err());
    }

    #[test]
    fn test_send_agent_message_response_from() {
        let result = SendResult {
            conversation_id: "conv-123".to_string(),
            agent_run_id: "run-456".to_string(),
            is_new_conversation: true,
        };

        let response = SendAgentMessageResponse::from(result);
        assert_eq!(response.conversation_id, "conv-123");
        assert_eq!(response.agent_run_id, "run-456");
        assert!(response.is_new_conversation);
    }

    #[test]
    fn test_queued_message_response_from() {
        let msg = QueuedMessage::new("Test content".to_string());
        let response = QueuedMessageResponse::from(msg.clone());

        assert_eq!(response.id, msg.id);
        assert_eq!(response.content, "Test content");
        assert!(!response.is_editing);
    }

    #[test]
    fn test_response_serialization() {
        let response = SendAgentMessageResponse {
            conversation_id: "conv-123".to_string(),
            agent_run_id: "run-456".to_string(),
            is_new_conversation: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("conversation_id")); // snake_case (Rust default)
        assert!(json.contains("agent_run_id"));
        assert!(json.contains("is_new_conversation"));
    }
}
