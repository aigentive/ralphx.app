// HTTP handlers for conversation endpoints
//
// Endpoints for querying conversation state, used by frontend to hydrate
// streaming UI when navigating to an active agent execution.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use super::*;
use crate::domain::services::RunningAgentKey;

/// GET /api/conversations/:id/active-state
///
/// Returns the current streaming state for a conversation, including:
/// - Whether an agent is currently running for this conversation
/// - Active tool calls in progress or recently completed
/// - Streaming tasks (subagents) currently running or completed
/// - Partial text content accumulated from agent:chunk events
///
/// Used by frontend to hydrate streaming UI when navigating to a conversation
/// where an agent execution was already in progress.
pub async fn get_conversation_active_state(
    State(state): State<HttpServerState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ActiveStateResponse>, StatusCode> {
    // Check if any agent is running for this conversation
    // We need to check all context types since a conversation belongs to a specific context
    // The RunningAgentRegistry stores info by context_type + context_id, but we have conversation_id
    // We can look up the conversation to find its context, then check the registry

    let conversation = state
        .app_state
        .chat_conversation_repo
        .get_by_id(&crate::domain::entities::ChatConversationId::from_string(
            conversation_id.clone(),
        ))
        .await
        .map_err(|e| {
            tracing::error!("Failed to get conversation: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if agent is running for this context
    let context_key = RunningAgentKey::new(
        conversation.context_type.to_string(),
        conversation.context_id.clone(),
    );
    let is_active = state
        .app_state
        .running_agent_registry
        .is_running(&context_key)
        .await;

    // Get streaming state from cache
    let cached_state = state
        .app_state
        .streaming_state_cache
        .get(&conversation_id)
        .await;

    // Build response
    let response = if let Some(state) = cached_state {
        ActiveStateResponse {
            is_active,
            tool_calls: state
                .tool_calls
                .into_iter()
                .map(ActiveToolCall::from)
                .collect(),
            streaming_tasks: state
                .streaming_tasks
                .into_iter()
                .map(ActiveStreamingTask::from)
                .collect(),
            partial_text: state.partial_text,
        }
    } else {
        ActiveStateResponse {
            is_active,
            tool_calls: Vec::new(),
            streaming_tasks: Vec::new(),
            partial_text: String::new(),
        }
    };

    Ok(Json(response))
}

#[cfg(test)]
#[path = "conversations_tests.rs"]
mod tests;
