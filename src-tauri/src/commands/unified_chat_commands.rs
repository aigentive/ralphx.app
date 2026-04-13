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
// - agent:run_completed - Agent finished successfully (or agent:turn_completed in interactive mode)
// - agent:error - Agent failed
// - agent:queue_sent - Queued message sent

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use tauri::State;

use crate::application::{AppChatService, AppState, ChatService, SendResult};
use crate::commands::ExecutionState;
use crate::domain::entities::{
    AgentRunId, AgentRunStatus, ChatContextType, ChatConversation, ChatConversationId,
    DelegatedSessionId, IdeationSessionId, TaskId,
};
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
    /// Optional target for team message routing.
    /// When set to a teammate name, the message is routed to that teammate's stdin
    /// instead of the lead's. "lead" or None routes to the lead (default behavior).
    pub target: Option<String>,
}

/// Response from send_agent_message command
#[derive(Debug, Serialize)]
pub struct SendAgentMessageResponse {
    pub conversation_id: String,
    pub agent_run_id: String,
    pub is_new_conversation: bool,
    #[serde(default)]
    pub was_queued: bool,
    #[serde(default)]
    pub queued_message_id: Option<String>,
}

impl From<SendResult> for SendAgentMessageResponse {
    fn from(result: SendResult) -> Self {
        Self {
            conversation_id: result.conversation_id,
            agent_run_id: result.agent_run_id,
            is_new_conversation: result.is_new_conversation,
            was_queued: result.was_queued,
            queued_message_id: result.queued_message_id,
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
    /// Optional target for team message routing (teammate name or "lead").
    pub target: Option<String>,
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
    pub provider_session_id: Option<String>,
    pub provider_harness: Option<String>,
    pub upstream_provider: Option<String>,
    pub provider_profile: Option<String>,
    pub title: Option<String>,
    pub message_count: i64,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ChatConversation> for AgentConversationResponse {
    fn from(c: ChatConversation) -> Self {
        let (claude_session_id, provider_session_id, provider_harness) =
            c.compatible_provider_session_fields();

        Self {
            id: c.id.as_str(),
            context_type: c.context_type.to_string(),
            context_id: c.context_id,
            claude_session_id,
            provider_session_id,
            provider_harness: provider_harness.map(|harness| harness.to_string()),
            upstream_provider: c.upstream_provider,
            provider_profile: c.provider_profile,
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
    pub metadata: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub content_blocks: Option<serde_json::Value>,
    pub attribution_source: Option<String>,
    pub provider_harness: Option<String>,
    pub provider_session_id: Option<String>,
    pub upstream_provider: Option<String>,
    pub provider_profile: Option<String>,
    pub logical_model: Option<String>,
    pub effective_model_id: Option<String>,
    pub logical_effort: Option<String>,
    pub effective_effort: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub estimated_usd: Option<f64>,
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
    pub model_id: Option<String>,
    pub model_label: Option<String>,
}

#[derive(Debug, Clone)]
struct DelegatedToolRuntimeSnapshot {
    session_id: String,
    conversation_id: Option<String>,
    agent_run_id: Option<String>,
    agent_name: String,
    title: Option<String>,
    harness: String,
    provider_session_id: Option<String>,
    session_status: String,
    session_error: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
    latest_run: Option<JsonValue>,
    recent_messages: Vec<JsonValue>,
}

fn is_delegate_start_tool_name(name: &str) -> bool {
    name == "delegate_start" || name.ends_with("::delegate_start")
}

fn parse_wrapped_mcp_result_object(result: &JsonValue) -> Option<JsonMap<String, JsonValue>> {
    if let Some(object) = result.as_object() {
        if let Some(content) = object.get("content").and_then(JsonValue::as_array) {
            if let Some(inner_text) = content
                .iter()
                .find_map(|entry| entry.get("text").and_then(JsonValue::as_str))
            {
                if let Ok(JsonValue::Object(inner)) = serde_json::from_str::<JsonValue>(inner_text) {
                    return Some(inner);
                }
            }
        }
        return Some(object.clone());
    }

    result
        .as_str()
        .and_then(|raw| serde_json::from_str::<JsonValue>(raw).ok())
        .and_then(|parsed| parsed.as_object().cloned())
}

fn get_string_field<'a>(object: &'a JsonMap<String, JsonValue>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(JsonValue::as_str)
}

fn provider_chat_message_recent_payload(content: &str, created_at: &str) -> JsonValue {
    serde_json::json!({
        "role": "assistant",
        "content": content,
        "created_at": created_at,
    })
}

fn delegated_agent_state_label(status: &str) -> &'static str {
    if status == AgentRunStatus::Running.to_string() {
        "likely_generating"
    } else {
        "idle"
    }
}

fn delegated_total_tokens_from_run(run: &crate::domain::entities::AgentRun) -> Option<u64> {
    let total = run.input_tokens.unwrap_or(0)
        + run.output_tokens.unwrap_or(0)
        + run.cache_creation_tokens.unwrap_or(0)
        + run.cache_read_tokens.unwrap_or(0);
    if total == 0
        && run.input_tokens.is_none()
        && run.output_tokens.is_none()
        && run.cache_creation_tokens.is_none()
        && run.cache_read_tokens.is_none()
    {
        None
    } else {
        Some(total)
    }
}

async fn load_delegated_tool_runtime_snapshot(
    state: &AppState,
    delegated_session_id: &str,
    delegated_conversation_id: Option<&str>,
    delegated_agent_run_id: Option<&str>,
) -> Option<DelegatedToolRuntimeSnapshot> {
    let session = state
        .delegated_session_repo
        .get_by_id(&DelegatedSessionId::from_string(delegated_session_id))
        .await
        .ok()
        .flatten()?;

    let conversation_id = delegated_conversation_id.map(str::to_string);
    let latest_run = if let Some(run_id) = delegated_agent_run_id {
        state
            .agent_run_repo
            .get_by_id(&AgentRunId::from_string(run_id))
            .await
            .ok()
            .flatten()
    } else if let Some(conversation_id) = delegated_conversation_id {
        state
            .agent_run_repo
            .get_latest_for_conversation(&ChatConversationId::from_string(conversation_id))
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let recent_messages = if let Some(conversation_id) = delegated_conversation_id {
        state
            .chat_message_repo
            .get_by_conversation(&ChatConversationId::from_string(conversation_id))
            .await
            .ok()
            .map(|messages| {
                messages
                    .into_iter()
                    .filter(|message| matches!(message.role.to_string().as_str(), "assistant" | "orchestrator"))
                    .rev()
                    .find_map(|message| {
                        let content = message.content.trim();
                        if content.is_empty() {
                            None
                        } else {
                            Some(provider_chat_message_recent_payload(
                                content,
                                &message.created_at.to_rfc3339(),
                            ))
                        }
                    })
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let latest_run_json = latest_run.as_ref().map(|run| {
        serde_json::json!({
            "agent_run_id": run.id.as_str(),
            "status": run.status.to_string(),
            "started_at": run.started_at.to_rfc3339(),
            "completed_at": run.completed_at.map(|timestamp| timestamp.to_rfc3339()),
            "error_message": run.error_message,
            "harness": run.harness.map(|value| value.to_string()),
            "provider_session_id": run.provider_session_id,
            "upstream_provider": run.upstream_provider,
            "provider_profile": run.provider_profile,
            "logical_model": run.logical_model,
            "effective_model_id": run.effective_model_id,
            "logical_effort": run.logical_effort.map(|value| value.to_string()),
            "effective_effort": run.effective_effort,
            "approval_policy": run.approval_policy,
            "sandbox_mode": run.sandbox_mode,
            "input_tokens": run.input_tokens,
            "output_tokens": run.output_tokens,
            "cache_creation_tokens": run.cache_creation_tokens,
            "cache_read_tokens": run.cache_read_tokens,
            "estimated_usd": run.estimated_usd,
            "total_tokens": delegated_total_tokens_from_run(run),
        })
    });

    Some(DelegatedToolRuntimeSnapshot {
        session_id: session.id.as_str().to_string(),
        conversation_id,
        agent_run_id: latest_run.as_ref().map(|run| run.id.as_str()),
        agent_name: session.agent_name,
        title: session.title,
        harness: session.harness.to_string(),
        provider_session_id: session.provider_session_id,
        session_status: latest_run
            .as_ref()
            .map(|run| run.status.to_string())
            .unwrap_or_else(|| session.status.clone()),
        session_error: latest_run
            .as_ref()
            .and_then(|run| run.error_message.clone())
            .or(session.error),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        completed_at: latest_run
            .as_ref()
            .and_then(|run| run.completed_at.map(|timestamp| timestamp.to_rfc3339()))
            .or_else(|| session.completed_at.map(|timestamp| timestamp.to_rfc3339())),
        latest_run: latest_run_json,
        recent_messages,
    })
}

fn merge_delegated_snapshot_into_result(
    result: &mut JsonValue,
    snapshot: &DelegatedToolRuntimeSnapshot,
) {
    let JsonValue::Object(result_object) = result else {
        return;
    };

    result_object.insert(
        "job_status".to_string(),
        JsonValue::String(snapshot.session_status.clone()),
    );
    result_object.insert(
        "status".to_string(),
        JsonValue::String(snapshot.session_status.clone()),
    );
    result_object.insert(
        "agent_name".to_string(),
        JsonValue::String(snapshot.agent_name.clone()),
    );
    result_object.insert(
        "delegated_session_id".to_string(),
        JsonValue::String(snapshot.session_id.clone()),
    );
    result_object.insert(
        "harness".to_string(),
        JsonValue::String(snapshot.harness.clone()),
    );
    if let Some(conversation_id) = snapshot.conversation_id.as_ref() {
        result_object.insert(
            "delegated_conversation_id".to_string(),
            JsonValue::String(conversation_id.clone()),
        );
    }
    if let Some(agent_run_id) = snapshot.agent_run_id.as_ref() {
        result_object.insert(
            "delegated_agent_run_id".to_string(),
            JsonValue::String(agent_run_id.clone()),
        );
    }
    if let Some(provider_session_id) = snapshot.provider_session_id.as_ref() {
        result_object.insert(
            "provider_session_id".to_string(),
            JsonValue::String(provider_session_id.clone()),
        );
    }
    if let Some(error) = snapshot.session_error.as_ref() {
        result_object.insert("error".to_string(), JsonValue::String(error.clone()));
    }
    if let Some(completed_at) = snapshot.completed_at.as_ref() {
        result_object.insert(
            "completed_at".to_string(),
            JsonValue::String(completed_at.clone()),
        );
    }

    result_object.insert(
        "delegated_status".to_string(),
        serde_json::json!({
            "session": {
                "id": snapshot.session_id,
                "title": snapshot.title,
                "status": snapshot.session_status,
                "parent_context_type": "ideation",
                "parent_context_id": JsonValue::Null,
                "agent_name": snapshot.agent_name,
                "harness": snapshot.harness,
                "provider_session_id": snapshot.provider_session_id,
                "created_at": snapshot.created_at,
                "updated_at": snapshot.updated_at,
                "completed_at": snapshot.completed_at,
            },
            "agent_state": {
                "estimated_status": delegated_agent_state_label(&snapshot.session_status),
            },
            "conversation_id": snapshot.conversation_id,
            "latest_run": snapshot.latest_run,
            "recent_messages": if snapshot.recent_messages.is_empty() {
                JsonValue::Null
            } else {
                JsonValue::Array(snapshot.recent_messages.clone())
            },
        }),
    );
}

async fn reconcile_delegated_result_payloads(
    state: &AppState,
    tool_calls: Option<String>,
    content_blocks: Option<String>,
) -> (Option<JsonValue>, Option<JsonValue>) {
    let mut snapshot_cache = HashMap::<String, DelegatedToolRuntimeSnapshot>::new();

    async fn reconcile_value_array(
        state: &AppState,
        raw: Option<String>,
        snapshot_cache: &mut HashMap<String, DelegatedToolRuntimeSnapshot>,
    ) -> Option<JsonValue> {
        let mut parsed = serde_json::from_str::<JsonValue>(&raw?).ok()?;
        let items = parsed.as_array_mut()?;

        for item in items.iter_mut() {
            let Some(item_object) = item.as_object_mut() else {
                continue;
            };
            let Some(name) = item_object.get("name").and_then(JsonValue::as_str) else {
                continue;
            };
            if !is_delegate_start_tool_name(name) {
                continue;
            }

            let Some(result) = item_object.get_mut("result") else {
                continue;
            };
            let Some(parsed_result) = parse_wrapped_mcp_result_object(result) else {
                continue;
            };

            let delegated_session_id = get_string_field(&parsed_result, "delegated_session_id")
                .or_else(|| get_string_field(&parsed_result, "delegatedSessionId"));
            let Some(delegated_session_id) = delegated_session_id else {
                continue;
            };
            let delegated_conversation_id = get_string_field(&parsed_result, "delegated_conversation_id")
                .or_else(|| get_string_field(&parsed_result, "delegatedConversationId"));
            let delegated_agent_run_id = get_string_field(&parsed_result, "delegated_agent_run_id")
                .or_else(|| get_string_field(&parsed_result, "delegatedAgentRunId"));

            let snapshot = if let Some(snapshot) = snapshot_cache.get(delegated_session_id) {
                snapshot.clone()
            } else {
                let Some(snapshot) = load_delegated_tool_runtime_snapshot(
                    state,
                    delegated_session_id,
                    delegated_conversation_id,
                    delegated_agent_run_id,
                )
                .await
                else {
                    continue;
                };
                snapshot_cache.insert(delegated_session_id.to_string(), snapshot.clone());
                snapshot
            };

            merge_delegated_snapshot_into_result(result, &snapshot);
        }

        Some(parsed)
    }

    let tool_calls = reconcile_value_array(state, tool_calls, &mut snapshot_cache).await;
    let content_blocks = reconcile_value_array(state, content_blocks, &mut snapshot_cache).await;
    (tool_calls, content_blocks)
}

// ============================================================================
// Helper to create ChatService
// ============================================================================

pub(crate) fn create_chat_service(
    state: &AppState,
    _app_handle: tauri::AppHandle,
    execution_state: &Arc<ExecutionState>,
    team_service: Option<std::sync::Arc<crate::application::TeamService>>,
) -> AppChatService<tauri::Wry> {
    let mut service = state.build_chat_service_with_execution_state(Arc::clone(execution_state));
    if let Some(svc) = team_service {
        service = service.with_team_service(svc);
    }
    service
}

/// Parse context type string to enum
#[doc(hidden)]
pub fn parse_context_type(context_type: &str) -> Result<ChatContextType, String> {
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
/// - agent:run_completed or agent:turn_completed (interactive) - When agent finishes
/// - agent:error - On failure
#[tauri::command]
pub async fn send_agent_message(
    input: SendAgentMessageInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    team_service: State<'_, std::sync::Arc<crate::application::TeamService>>,
    app: tauri::AppHandle,
) -> Result<SendAgentMessageResponse, String> {
    tracing::info!(
        context_type = %input.context_type,
        context_id = %input.context_id,
        content_len = input.content.len(),
        target = ?input.target,
        "[SEND_MSG] send_agent_message command invoked"
    );
    let context_type = parse_context_type(&input.context_type)?;

    let mut service = create_chat_service(
        &state,
        app,
        &execution_state,
        Some(team_service.inner().clone()),
    );

    // For ideation contexts, check if the session has team_mode enabled
    if context_type == ChatContextType::Ideation {
        let session_id = IdeationSessionId::from_string(&input.context_id);
        if let Ok(Some(session)) = state.ideation_session_repo.get_by_id(&session_id).await {
            let is_team = session.team_mode.as_deref().is_some_and(|m| m != "solo");
            if is_team {
                service = service.with_team_mode(true);
            }
        }
    }

    // For execution contexts, check if the task's metadata has agent_variant = "team"
    if context_type == ChatContextType::TaskExecution {
        let task_id = TaskId::from_string(input.context_id.clone());
        if let Ok(Some(task)) = state.task_repo.get_by_id(&task_id).await {
            let is_team = task
                .metadata
                .as_ref()
                .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                .and_then(|meta| {
                    meta.get("agent_variant")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "team")
                })
                .unwrap_or(false);
            if is_team {
                service = service.with_team_mode(true);
            }
        }
    }

    crate::application::validate_chat_runtime_for_context(
        &state,
        context_type,
        &input.context_id,
        "send_agent_message",
    )
    .await?;

    // Route to teammate stdin when target is a specific teammate (not "lead")
    let target = input.target.as_deref();
    if let Some(teammate_name) = target.filter(|t| *t != "lead") {
        // Find the active team for this context
        if let Some(team_name) = team_service.find_team_by_context_id(&input.context_id).await {
            let formatted =
                crate::infrastructure::agents::claude::format_stream_json_input(&input.content);
            team_service
                .send_stdin_message(&team_name, teammate_name, &formatted)
                .await
                .map_err(|e| format!("Failed to send to teammate {}: {}", teammate_name, e))?;

            tracing::info!(
                teammate = %teammate_name,
                team = %team_name,
                "Routed user message to teammate stdin"
            );

            // Return a synthetic response — the teammate's stream processor handles
            // conversation persistence and event emission.
            return Ok(SendAgentMessageResponse {
                conversation_id: String::new(),
                agent_run_id: uuid::Uuid::new_v4().to_string(),
                is_new_conversation: false,
                was_queued: false,
                queued_message_id: None,
            });
        }
        // Team not found for context — fall through to normal lead path
        tracing::warn!(
            target = %teammate_name,
            context_id = %input.context_id,
            "No active team found for context, falling back to lead"
        );
    }

    service
        .send_message(context_type, &input.context_id, &input.content, Default::default())
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
    tracing::info!(
        context_type = %input.context_type,
        context_id = %input.context_id,
        content_len = input.content.len(),
        "[QUEUE_MSG] queue_agent_message command invoked"
    );
    let context_type = parse_context_type(&input.context_type)?;

    let service = create_chat_service(&state, app, &execution_state, None);

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

    let service = create_chat_service(&state, app, &execution_state, None);

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

    let service = create_chat_service(&state, app, &execution_state, None);

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

    let service = create_chat_service(&state, app, &execution_state, None);

    service
        .list_conversations(context_type_enum, &context_id)
        .await
        .map(|convs| {
            convs
                .into_iter()
                .map(AgentConversationResponse::from)
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

    let service = create_chat_service(&state, app, &execution_state, None);

    let conversation = service
        .get_conversation_with_messages(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(cwm) = conversation else {
        return Ok(None);
    };

    let mut messages = Vec::with_capacity(cwm.messages.len());
    for message in cwm.messages {
        let (tool_calls, content_blocks) = reconcile_delegated_result_payloads(
            &state,
            message.tool_calls.clone(),
            message.content_blocks.clone(),
        )
        .await;

        messages.push(AgentMessageResponse {
            id: message.id.as_str().to_string(),
            role: message.role.to_string(),
            content: message.content,
            metadata: message.metadata,
            tool_calls,
            content_blocks,
            attribution_source: message.attribution_source,
            provider_harness: message.provider_harness.map(|value| value.to_string()),
            provider_session_id: message.provider_session_id,
            upstream_provider: message.upstream_provider,
            provider_profile: message.provider_profile,
            logical_model: message.logical_model,
            effective_model_id: message.effective_model_id,
            logical_effort: message.logical_effort.map(|value| value.to_string()),
            effective_effort: message.effective_effort,
            input_tokens: message.input_tokens,
            output_tokens: message.output_tokens,
            cache_creation_tokens: message.cache_creation_tokens,
            cache_read_tokens: message.cache_read_tokens,
            estimated_usd: message.estimated_usd,
            created_at: message.created_at.to_rfc3339(),
        });
    }

    Ok(Some(AgentConversationWithMessagesResponse {
        conversation: AgentConversationResponse::from(cwm.conversation),
        messages,
    }))
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
    use crate::domain::services::RunningAgentKey;
    use crate::infrastructure::agents::claude::model_labels::model_id_to_label;

    let conv_id = ChatConversationId::from_string(&conversation_id);

    let service = create_chat_service(&state, app, &execution_state, None);

    let Some(run) = service
        .get_active_run(&conv_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    // Look up conversation to get context_type/context_id for registry lookup
    let (model_id, model_label) = if let Ok(Some(conv)) =
        state.chat_conversation_repo.get_by_id(&conv_id).await
    {
        let key = RunningAgentKey::new(conv.context_type.to_string(), conv.context_id.clone());
        let agent_info = state.running_agent_registry.get(&key).await;
        let mid = agent_info.and_then(|info| info.model);
        let mlabel = mid.as_deref().map(|id| model_id_to_label(id));
        (mid, mlabel)
    } else {
        (None, None)
    };

    Ok(Some(AgentRunStatusResponse {
        id: run.id.as_str().to_string(),
        conversation_id: run.conversation_id.as_str().to_string(),
        status: run.status.to_string(),
        started_at: run.started_at.to_rfc3339(),
        completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
        error_message: run.error_message,
        model_id,
        model_label,
    }))
}

/// Check if the chat service is available
#[tauri::command]
pub async fn is_chat_service_available(
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let service = create_chat_service(&state, app, &execution_state, None);
    Ok(service.is_available().await)
}

/// Stop a running agent for a context
///
/// Sends SIGTERM to the running agent process and emits agent:stopped event.
/// Returns true if an agent was stopped, false if no agent was running.
///
/// Events emitted:
/// - agent:stopped - When agent is terminated
/// - agent:run_completed or agent:turn_completed (interactive) - So frontend knows agent is no longer running
#[tauri::command]
pub async fn stop_agent(
    context_type: String,
    context_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let context_type = parse_context_type(&context_type)?;

    let service = create_chat_service(&state, app, &execution_state, None);

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

    let service = create_chat_service(&state, app, &execution_state, None);

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
        ChatConversation, DelegatedSessionId, IdeationSessionId, ProjectId, TaskId,
    };

    let context_type = parse_context_type(&input.context_type)?;

    let conversation = match context_type {
        ChatContextType::Ideation => {
            ChatConversation::new_ideation(IdeationSessionId::from_string(&input.context_id))
        }
        ChatContextType::Delegation => {
            ChatConversation::new_delegation(DelegatedSessionId::from_string(&input.context_id))
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
        ChatContextType::Merge => {
            ChatConversation::new_merge(TaskId::from_string(input.context_id.clone()))
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
    use super::{
        merge_delegated_snapshot_into_result, parse_wrapped_mcp_result_object,
        AgentConversationResponse, DelegatedToolRuntimeSnapshot,
    };
    use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
    use crate::domain::entities::{ChatConversation, ProjectId};
    use serde_json::json;

    #[test]
    fn agent_conversation_response_derives_provider_metadata_from_legacy_claude_session() {
        let mut conversation =
            ChatConversation::new_project(ProjectId::from_string("project-1".to_string()));
        conversation.claude_session_id = Some("claude-session-123".to_string());

        let response = AgentConversationResponse::from(conversation);

        assert_eq!(
            response.claude_session_id,
            Some("claude-session-123".to_string())
        );
        assert_eq!(
            response.provider_session_id,
            Some("claude-session-123".to_string())
        );
        assert_eq!(response.provider_harness, Some("claude".to_string()));
    }

    #[test]
    fn agent_conversation_response_keeps_codex_metadata_without_legacy_alias() {
        let mut conversation =
            ChatConversation::new_project(ProjectId::from_string("project-1".to_string()));
        conversation.set_provider_session_ref(ProviderSessionRef {
            harness: AgentHarnessKind::Codex,
            provider_session_id: "codex-thread-123".to_string(),
        });

        let response = AgentConversationResponse::from(conversation);

        assert_eq!(response.claude_session_id, None);
        assert_eq!(
            response.provider_session_id,
            Some("codex-thread-123".to_string())
        );
        assert_eq!(response.provider_harness, Some("codex".to_string()));
    }

    #[test]
    fn agent_conversation_response_restores_legacy_alias_for_canonical_claude_provider_metadata() {
        let mut conversation =
            ChatConversation::new_project(ProjectId::from_string("project-1".to_string()));
        conversation.provider_harness = Some(AgentHarnessKind::Claude);
        conversation.provider_session_id = Some("claude-session-456".to_string());
        conversation.claude_session_id = None;

        let response = AgentConversationResponse::from(conversation);

        assert_eq!(
            response.claude_session_id,
            Some("claude-session-456".to_string())
        );
        assert_eq!(
            response.provider_session_id,
            Some("claude-session-456".to_string())
        );
        assert_eq!(response.provider_harness, Some("claude".to_string()));
    }

    #[test]
    fn parse_wrapped_mcp_result_object_extracts_embedded_json_payload() {
        let result = json!({
            "content": [
                {
                    "type": "text",
                    "text": "{\"delegated_session_id\":\"delegated-1\",\"status\":\"running\"}"
                }
            ]
        });

        let parsed = parse_wrapped_mcp_result_object(&result).expect("parsed result");

        assert_eq!(
            parsed.get("delegated_session_id").and_then(|value| value.as_str()),
            Some("delegated-1")
        );
        assert_eq!(
            parsed.get("status").and_then(|value| value.as_str()),
            Some("running")
        );
    }

    #[test]
    fn merge_delegated_snapshot_overrides_running_result_with_terminal_runtime_state() {
        let mut result = json!({
            "delegated_session_id": "delegated-1",
            "status": "running",
            "job_status": "running"
        });
        let snapshot = DelegatedToolRuntimeSnapshot {
            session_id: "delegated-1".to_string(),
            conversation_id: Some("conversation-1".to_string()),
            agent_run_id: Some("run-1".to_string()),
            agent_name: "ralphx-plan-critic-completeness".to_string(),
            title: Some("Completeness critic".to_string()),
            harness: "codex".to_string(),
            provider_session_id: Some("provider-1".to_string()),
            session_status: "completed".to_string(),
            session_error: None,
            created_at: "2026-04-13T10:00:00Z".to_string(),
            updated_at: "2026-04-13T10:01:00Z".to_string(),
            completed_at: Some("2026-04-13T10:01:30Z".to_string()),
            latest_run: Some(json!({
                "agent_run_id": "run-1",
                "status": "completed"
            })),
            recent_messages: vec![json!({
                "role": "assistant",
                "content": "Completeness: no critical blockers found.",
                "created_at": "2026-04-13T10:01:20Z"
            })],
        };

        merge_delegated_snapshot_into_result(&mut result, &snapshot);

        assert_eq!(result.get("status").and_then(|value| value.as_str()), Some("completed"));
        assert_eq!(
            result.get("job_status").and_then(|value| value.as_str()),
            Some("completed")
        );
        assert_eq!(
            result
                .get("delegated_status")
                .and_then(|value| value.get("latest_run"))
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str()),
            Some("completed")
        );
        assert_eq!(
            result
                .get("delegated_status")
                .and_then(|value| value.get("recent_messages"))
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(1)
        );
    }
}
