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
use tauri::{Emitter, State};

use crate::application::agent_conversation_workspace::{
    agent_name_for_workspace_mode, prepare_agent_conversation_workspace,
    resolve_valid_agent_conversation_workspace_path, AgentConversationWorkspaceBaseSelection,
};
use crate::application::chat_service::{
    create_assistant_message, AgentConversationCreatedPayload, SendMessageOptions,
};
use crate::application::git_service::GitService;
use crate::application::publish_resilience::{
    classify_publish_failure, count_publish_reviewable_commits, ensure_publish_branch_fresh,
    inspect_publish_branch_freshness, publish_push_status_for_failure, push_publish_branch,
    review_base_for_publish, PublishBranchFreshnessOutcome, PublishBranchFreshnessStatus,
    PublishFailureClass,
};
use crate::application::{
    AgentMessageCreatedPayload, AppChatService, AppState, ChatService, ChatServiceError, SendResult,
};
use crate::commands::ExecutionState;
use crate::domain::agents::AgentHarnessKind;
use crate::domain::entities::{
    AgentConversationWorkspace, AgentConversationWorkspaceMode,
    AgentConversationWorkspacePublicationEvent, AgentRunId, AgentRunStatus, ChatContextType,
    ChatConversation, ChatConversationId, DelegatedSessionId, IdeationAnalysisBaseRefKind,
    IdeationSessionId, ProjectId, TaskId,
};
use crate::domain::services::{AgentWorkspacePrPublisher, QueuedMessage, RunningAgentKey};

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
    /// Optional existing conversation to continue.
    pub conversation_id: Option<String>,
    /// Optional provider harness override for the first spawn of a conversation.
    pub provider_harness: Option<String>,
    /// Optional explicit model override for the spawned agent.
    pub model_override: Option<String>,
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
    pub queued_as_pending: bool,
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
            queued_as_pending: result.queued_as_pending,
            queued_message_id: result.queued_message_id,
        }
    }
}

/// Input for creating a project-backed agent conversation with an isolated workspace.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartAgentConversationInput {
    pub project_id: String,
    pub content: String,
    /// Optional draft conversation to use after uploading pending attachments.
    pub conversation_id: Option<String>,
    /// Optional provider harness override for the first spawn of the conversation.
    pub provider_harness: Option<String>,
    /// Optional explicit model override for the spawned agent.
    pub model_override: Option<String>,
    /// Agent mode: "chat" routes to read-only explorer; all modes create a selected-base workspace for the runtime CWD.
    pub mode: Option<String>,
    /// Optional base ref kind using ideation naming: project_default, current_branch, local_branch.
    pub base_ref_kind: Option<String>,
    /// Optional selected branch/ref name for the base.
    pub base_ref: Option<String>,
    /// Optional user-facing base ref label.
    pub base_display_name: Option<String>,
}

/// Response for an agent conversation workspace.
#[derive(Debug, Serialize)]
pub struct AgentConversationWorkspaceResponse {
    pub conversation_id: String,
    pub project_id: String,
    pub mode: String,
    pub base_ref_kind: String,
    pub base_ref: String,
    pub base_display_name: Option<String>,
    pub base_commit: Option<String>,
    pub branch_name: String,
    pub worktree_path: String,
    pub linked_ideation_session_id: Option<String>,
    pub linked_plan_branch_id: Option<String>,
    pub publication_pr_number: Option<i64>,
    pub publication_pr_url: Option<String>,
    pub publication_pr_status: Option<String>,
    pub publication_push_status: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AgentConversationWorkspace> for AgentConversationWorkspaceResponse {
    fn from(workspace: AgentConversationWorkspace) -> Self {
        Self {
            conversation_id: workspace.conversation_id.as_str(),
            project_id: workspace.project_id.as_str().to_string(),
            mode: workspace.mode.to_string(),
            base_ref_kind: workspace.base_ref_kind.to_string(),
            base_ref: workspace.base_ref,
            base_display_name: workspace.base_display_name,
            base_commit: workspace.base_commit,
            branch_name: workspace.branch_name,
            worktree_path: workspace.worktree_path,
            linked_ideation_session_id: workspace
                .linked_ideation_session_id
                .map(|id| id.as_str().to_string()),
            linked_plan_branch_id: workspace
                .linked_plan_branch_id
                .map(|id| id.as_str().to_string()),
            publication_pr_number: workspace.publication_pr_number,
            publication_pr_url: workspace.publication_pr_url,
            publication_pr_status: workspace.publication_pr_status,
            publication_push_status: workspace.publication_push_status,
            status: workspace.status.to_string(),
            created_at: workspace.created_at.to_rfc3339(),
            updated_at: workspace.updated_at.to_rfc3339(),
        }
    }
}

/// Response from start_agent_conversation command.
#[derive(Debug, Serialize)]
pub struct StartAgentConversationResponse {
    pub conversation: AgentConversationResponse,
    pub workspace: Option<AgentConversationWorkspaceResponse>,
    pub send_result: SendAgentMessageResponse,
}

/// Input for changing the active mode of an existing project-backed agent conversation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchAgentConversationModeInput {
    pub conversation_id: String,
    pub mode: String,
    /// Optional base ref kind used when upgrading a branchless chat into edit/ideation mode.
    pub base_ref_kind: Option<String>,
    /// Optional selected branch/ref name for the base.
    pub base_ref: Option<String>,
    /// Optional user-facing base ref label.
    pub base_display_name: Option<String>,
}

/// Response from switch_agent_conversation_mode command.
#[derive(Debug, Serialize)]
pub struct SwitchAgentConversationModeResponse {
    pub conversation: AgentConversationResponse,
    pub workspace: Option<AgentConversationWorkspaceResponse>,
}

/// Response from publishing a project-backed agent conversation workspace.
#[derive(Debug, Serialize)]
pub struct PublishAgentConversationWorkspaceResponse {
    pub workspace: AgentConversationWorkspaceResponse,
    pub commit_sha: Option<String>,
    pub pushed: bool,
    pub created_pr: bool,
    pub pr_number: Option<i64>,
    pub pr_url: Option<String>,
}

/// Read-only freshness state for an edit-agent workspace base branch.
#[derive(Debug, Serialize)]
pub struct AgentConversationWorkspaceFreshnessResponse {
    pub conversation_id: String,
    pub base_ref: String,
    pub base_display_name: Option<String>,
    pub target_ref: String,
    pub captured_base_commit: Option<String>,
    pub target_base_commit: String,
    pub is_base_ahead: bool,
}

impl AgentConversationWorkspaceFreshnessResponse {
    fn from_workspace_status(
        workspace: &AgentConversationWorkspace,
        status: PublishBranchFreshnessStatus,
    ) -> Self {
        Self {
            conversation_id: workspace.conversation_id.as_str(),
            base_ref: workspace.base_ref.clone(),
            base_display_name: workspace.base_display_name.clone(),
            target_ref: status.target_ref,
            captured_base_commit: status.captured_base_commit,
            target_base_commit: status.target_base_commit,
            is_base_ahead: status.is_base_ahead,
        }
    }
}

/// Result of explicitly updating an edit-agent workspace branch from its base.
#[derive(Debug, Serialize)]
pub struct UpdateAgentConversationWorkspaceFromBaseResponse {
    pub workspace: AgentConversationWorkspaceResponse,
    pub updated: bool,
    pub target_ref: String,
    pub base_commit: String,
}

/// Durable publish operation event for an agent conversation workspace.
#[derive(Debug, Serialize)]
pub struct AgentConversationWorkspacePublicationEventResponse {
    pub id: String,
    pub conversation_id: String,
    pub step: String,
    pub status: String,
    pub summary: String,
    pub classification: Option<String>,
    pub created_at: String,
}

impl From<AgentConversationWorkspacePublicationEvent>
    for AgentConversationWorkspacePublicationEventResponse
{
    fn from(event: AgentConversationWorkspacePublicationEvent) -> Self {
        Self {
            id: event.id,
            conversation_id: event.conversation_id.as_str(),
            step: event.step,
            status: event.status,
            summary: event.summary,
            classification: event.classification,
            created_at: event.created_at.to_rfc3339(),
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
    pub agent_mode: Option<String>,
    pub title: Option<String>,
    pub message_count: i64,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
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
            agent_mode: c.agent_mode.map(|mode| mode.to_string()),
            title: c.title,
            message_count: c.message_count,
            last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
            archived_at: c.archived_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Response for paginated conversation listing
#[derive(Debug, Serialize)]
pub struct AgentConversationListPageResponse {
    pub conversations: Vec<AgentConversationResponse>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
}

/// Response for conversation with messages
#[derive(Debug, Serialize)]
pub struct AgentConversationWithMessagesResponse {
    pub conversation: AgentConversationResponse,
    pub messages: Vec<AgentMessageResponse>,
}

/// Response for a paginated conversation message window
#[derive(Debug, Serialize)]
pub struct AgentConversationMessagesPageResponse {
    pub conversation: AgentConversationResponse,
    pub messages: Vec<AgentMessageResponse>,
    pub limit: u32,
    pub offset: u32,
    pub total_message_count: i64,
    pub has_older: bool,
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

/// Input for append_agent_bridge_message.
///
/// Used by the project-agent UI bridge to persist child workflow milestones
/// (ideation verification, proposals, task execution) into the parent project
/// conversation without spawning another model turn.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendAgentBridgeMessageInput {
    pub conversation_id: String,
    pub source_session_id: String,
    pub event_type: String,
    pub event_key: String,
    pub content: String,
    #[serde(default)]
    pub metadata: Option<JsonValue>,
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
                if let Ok(JsonValue::Object(inner)) = serde_json::from_str::<JsonValue>(inner_text)
                {
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
                    .filter(|message| {
                        matches!(
                            message.role.to_string().as_str(),
                            "assistant" | "orchestrator"
                        )
                    })
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
            let delegated_conversation_id =
                get_string_field(&parsed_result, "delegated_conversation_id")
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

fn parse_agent_workspace_mode(
    mode: Option<&str>,
) -> Result<AgentConversationWorkspaceMode, String> {
    mode.map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("edit")
        .parse::<AgentConversationWorkspaceMode>()
}

fn parse_agent_workspace_base_kind(
    kind: Option<&str>,
) -> Result<Option<IdeationAnalysisBaseRefKind>, String> {
    kind.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<IdeationAnalysisBaseRefKind>)
        .transpose()
}

fn agent_mode_requires_workspace(mode: AgentConversationWorkspaceMode) -> bool {
    matches!(
        mode,
        AgentConversationWorkspaceMode::Chat
            | AgentConversationWorkspaceMode::Edit
            | AgentConversationWorkspaceMode::Ideation
    )
}

fn validate_agent_conversation_mode_transition(
    current_mode: AgentConversationWorkspaceMode,
    target_mode: AgentConversationWorkspaceMode,
    workspace_has_state_owner: bool,
) -> Result<(), String> {
    if current_mode == AgentConversationWorkspaceMode::Ideation
        && target_mode != AgentConversationWorkspaceMode::Ideation
    {
        return Err(
            "Ideation mode conversations cannot be switched to another mode yet".to_string(),
        );
    }

    if workspace_has_state_owner && target_mode != AgentConversationWorkspaceMode::Ideation {
        return Err(
            "This workspace is owned by ideation or execution state and cannot leave Ideation Mode"
                .to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod agent_mode_workspace_tests {
    use super::*;

    #[test]
    fn all_agent_conversation_modes_require_workspace() {
        assert!(agent_mode_requires_workspace(
            AgentConversationWorkspaceMode::Chat
        ));
        assert!(agent_mode_requires_workspace(
            AgentConversationWorkspaceMode::Edit
        ));
        assert!(agent_mode_requires_workspace(
            AgentConversationWorkspaceMode::Ideation
        ));
    }

    #[test]
    fn active_agent_conversations_support_expected_valid_mode_transition_matrix() {
        let valid_transitions = [
            (
                AgentConversationWorkspaceMode::Chat,
                AgentConversationWorkspaceMode::Chat,
            ),
            (
                AgentConversationWorkspaceMode::Chat,
                AgentConversationWorkspaceMode::Edit,
            ),
            (
                AgentConversationWorkspaceMode::Chat,
                AgentConversationWorkspaceMode::Ideation,
            ),
            (
                AgentConversationWorkspaceMode::Edit,
                AgentConversationWorkspaceMode::Chat,
            ),
            (
                AgentConversationWorkspaceMode::Edit,
                AgentConversationWorkspaceMode::Edit,
            ),
            (
                AgentConversationWorkspaceMode::Edit,
                AgentConversationWorkspaceMode::Ideation,
            ),
            (
                AgentConversationWorkspaceMode::Ideation,
                AgentConversationWorkspaceMode::Ideation,
            ),
        ];

        for (current_mode, target_mode) in valid_transitions {
            assert!(
                validate_agent_conversation_mode_transition(current_mode, target_mode, false)
                    .is_ok(),
                "{current_mode} -> {target_mode} should be allowed"
            );
        }
    }

    #[test]
    fn active_ideation_conversations_cannot_leave_ideation_mode() {
        for target_mode in [
            AgentConversationWorkspaceMode::Chat,
            AgentConversationWorkspaceMode::Edit,
        ] {
            let error = validate_agent_conversation_mode_transition(
                AgentConversationWorkspaceMode::Ideation,
                target_mode,
                false,
            )
            .expect_err("ideation conversations should not leave ideation mode");

            assert!(error.contains("Ideation mode conversations cannot be switched"));
        }
    }

    #[test]
    fn state_owned_workspaces_can_only_target_ideation_mode() {
        for target_mode in [
            AgentConversationWorkspaceMode::Chat,
            AgentConversationWorkspaceMode::Edit,
        ] {
            let error = validate_agent_conversation_mode_transition(
                AgentConversationWorkspaceMode::Chat,
                target_mode,
                true,
            )
            .expect_err("state-owned workspaces should not leave ideation ownership");

            assert!(error.contains("owned by ideation or execution state"));
        }

        assert!(validate_agent_conversation_mode_transition(
            AgentConversationWorkspaceMode::Chat,
            AgentConversationWorkspaceMode::Ideation,
            true,
        )
        .is_ok());
    }
}

fn build_agent_workspace_commit_message(conversation: &ChatConversation) -> String {
    let title = conversation
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "Untitled agent")
        .unwrap_or("agent conversation work");
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("feat: {title}")
}

// ============================================================================
// Commands
// ============================================================================

/// Start a project-backed agent conversation in an isolated feature worktree.
#[tauri::command]
pub async fn start_agent_conversation(
    input: StartAgentConversationInput,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    team_service: State<'_, std::sync::Arc<crate::application::TeamService>>,
    app: tauri::AppHandle,
) -> Result<StartAgentConversationResponse, String> {
    tracing::info!(
        project_id = %input.project_id,
        content_len = input.content.len(),
        mode = ?input.mode,
        base_ref_kind = ?input.base_ref_kind,
        base_ref = ?input.base_ref,
        "[START_AGENT_CONVERSATION] command invoked"
    );

    crate::application::validate_chat_runtime_for_context(
        &state,
        ChatContextType::Project,
        &input.project_id,
        "start_agent_conversation",
    )
    .await?;

    let mode = parse_agent_workspace_mode(input.mode.as_deref())?;
    let base_ref_kind = parse_agent_workspace_base_kind(input.base_ref_kind.as_deref())?;
    let project_id = ProjectId::from_string(input.project_id.clone());
    let project = state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project not found: {}", input.project_id))?;

    let draft_conversation_id = input
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|conversation_id| !conversation_id.is_empty())
        .map(ChatConversationId::from_string);
    let mut conversation = if let Some(conversation_id) = draft_conversation_id {
        let conversation = state
            .chat_conversation_repo
            .get_by_id(&conversation_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Conversation not found: {}", conversation_id))?;
        if conversation.context_type != ChatContextType::Project
            || conversation.context_id != input.project_id
        {
            return Err(format!(
                "Conversation {} does not belong to project {}",
                conversation.id, input.project_id
            ));
        }
        conversation
    } else {
        ChatConversation::new_project(project_id)
    };
    conversation.set_agent_mode(Some(mode));
    let should_create_conversation = draft_conversation_id.is_none();
    let workspace = if agent_mode_requires_workspace(mode) {
        Some(
            prepare_agent_conversation_workspace(
                &project,
                &conversation.id,
                mode,
                AgentConversationWorkspaceBaseSelection {
                    kind: base_ref_kind,
                    base_ref: input
                        .base_ref
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    display_name: input
                        .base_display_name
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                },
            )
            .await
            .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };

    let conversation = if should_create_conversation {
        state
            .chat_conversation_repo
            .create(conversation)
            .await
            .map_err(|error| error.to_string())?
    } else {
        state
            .chat_conversation_repo
            .update_agent_mode(&conversation.id, Some(mode))
            .await
            .map_err(|error| error.to_string())?;
        conversation
    };
    let workspace = match workspace {
        Some(workspace) => match state
            .agent_conversation_workspace_repo
            .create_or_update(workspace)
            .await
        {
            Ok(workspace) => Some(workspace),
            Err(error) => {
                if should_create_conversation {
                    let _ = state.chat_conversation_repo.delete(&conversation.id).await;
                }
                return Err(error.to_string());
            }
        },
        None => None,
    };

    if should_create_conversation {
        let _ = app.emit(
            "agent:conversation_created",
            AgentConversationCreatedPayload {
                conversation_id: conversation.id.as_str(),
                context_type: ChatContextType::Project.to_string(),
                context_id: input.project_id.clone(),
            },
        );
    }

    let service = create_chat_service(
        &state,
        app,
        &execution_state,
        Some(team_service.inner().clone()),
    );
    let harness_override = input
        .provider_harness
        .as_deref()
        .map(str::parse::<AgentHarnessKind>)
        .transpose()?;
    let model_override = input
        .model_override
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string);
    let send_result = service
        .send_message(
            ChatContextType::Project,
            &input.project_id,
            &input.content,
            SendMessageOptions {
                harness_override,
                agent_name_override: Some(agent_name_for_workspace_mode(mode).to_string()),
                model_override,
                conversation_id_override: Some(conversation.id),
                ..Default::default()
            },
        )
        .await
        .map(SendAgentMessageResponse::from)
        .map_err(|error| error.to_string())?;

    Ok(StartAgentConversationResponse {
        conversation: AgentConversationResponse::from(conversation),
        workspace: workspace.map(AgentConversationWorkspaceResponse::from),
        send_result,
    })
}

/// Switch a project-backed agent conversation between chat/edit/ideation modes.
#[tauri::command]
pub async fn switch_agent_conversation_mode(
    input: SwitchAgentConversationModeInput,
    state: State<'_, AppState>,
) -> Result<SwitchAgentConversationModeResponse, String> {
    let conversation_id = ChatConversationId::from_string(input.conversation_id.clone());
    let target_mode = parse_agent_workspace_mode(Some(input.mode.as_str()))?;
    let base_ref_kind = parse_agent_workspace_base_kind(input.base_ref_kind.as_deref())?;

    let mut conversation = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Conversation not found: {}", conversation_id))?;
    if conversation.context_type != ChatContextType::Project {
        return Err("Only project agent conversations can change mode".to_string());
    }

    let running_key = RunningAgentKey::new(
        ChatContextType::Project.to_string(),
        conversation.id.as_str(),
    );
    if state.running_agent_registry.is_running(&running_key).await {
        return Err("Cannot change mode while the agent is running".to_string());
    }

    let existing_workspace = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation.id)
        .await
        .map_err(|error| error.to_string())?;
    let current_mode = conversation
        .agent_mode
        .or_else(|| existing_workspace.as_ref().map(|workspace| workspace.mode))
        .unwrap_or(AgentConversationWorkspaceMode::Chat);

    validate_agent_conversation_mode_transition(
        current_mode,
        target_mode,
        existing_workspace
            .as_ref()
            .map(|workspace| {
                workspace.linked_ideation_session_id.is_some()
                    || workspace.linked_plan_branch_id.is_some()
            })
            .unwrap_or(false),
    )?;

    let workspace = if agent_mode_requires_workspace(target_mode) {
        Some(match existing_workspace {
            Some(mut workspace) => {
                if workspace.mode != target_mode {
                    workspace.mode = target_mode;
                    workspace.updated_at = chrono::Utc::now();
                    state
                        .agent_conversation_workspace_repo
                        .create_or_update(workspace)
                        .await
                        .map_err(|error| error.to_string())?
                } else {
                    workspace
                }
            }
            None => {
                let project_id = ProjectId::from_string(conversation.context_id.clone());
                let project = state
                    .project_repo
                    .get_by_id(&project_id)
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("Project not found: {}", conversation.context_id))?;
                let workspace = prepare_agent_conversation_workspace(
                    &project,
                    &conversation.id,
                    target_mode,
                    AgentConversationWorkspaceBaseSelection {
                        kind: base_ref_kind,
                        base_ref: input
                            .base_ref
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty()),
                        display_name: input
                            .base_display_name
                            .map(|value| value.trim().to_string())
                            .filter(|value| !value.is_empty()),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
                state
                    .agent_conversation_workspace_repo
                    .create_or_update(workspace)
                    .await
                    .map_err(|error| error.to_string())?
            }
        })
    } else {
        existing_workspace
    };

    state
        .chat_conversation_repo
        .update_agent_mode(&conversation.id, Some(target_mode))
        .await
        .map_err(|error| error.to_string())?;
    conversation.set_agent_mode(Some(target_mode));
    if current_mode != target_mode {
        state
            .chat_conversation_repo
            .clear_provider_session_ref(&conversation.id)
            .await
            .map_err(|error| error.to_string())?;
        conversation.clear_provider_session_ref();
    }

    let conversation = state
        .chat_conversation_repo
        .get_by_id(&conversation.id)
        .await
        .map_err(|error| error.to_string())?
        .unwrap_or(conversation);

    Ok(SwitchAgentConversationModeResponse {
        conversation: AgentConversationResponse::from(conversation),
        workspace: workspace.map(AgentConversationWorkspaceResponse::from),
    })
}

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
        if let Some(team_name) = team_service
            .find_team_by_context_id(&input.context_id)
            .await
        {
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
                queued_as_pending: false,
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

    let harness_override = input
        .provider_harness
        .as_deref()
        .map(str::parse::<AgentHarnessKind>)
        .transpose()?;
    let model_override = input
        .model_override
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string);
    let conversation_id_override = input
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|conversation_id| !conversation_id.is_empty())
        .map(ChatConversationId::from_string);

    service
        .send_message(
            context_type,
            &input.context_id,
            &input.content,
            SendMessageOptions {
                harness_override,
                model_override,
                conversation_id_override,
                ..Default::default()
            },
        )
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
    include_archived: Option<bool>,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    app: tauri::AppHandle,
) -> Result<Vec<AgentConversationResponse>, String> {
    let context_type_enum = parse_context_type(&context_type)?;

    let include_archived = include_archived.unwrap_or(false);
    let conversations = if include_archived {
        state
            .chat_conversation_repo
            .get_by_context_filtered(context_type_enum, &context_id, true)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let service = create_chat_service(&state, app, &execution_state, None);
        service
            .list_conversations(context_type_enum, &context_id)
            .await
            .map_err(|e| e.to_string())?
    };

    Ok(conversations
        .into_iter()
        .map(AgentConversationResponse::from)
        .collect())
}

/// List a page of conversations for a context with optional title search.
#[tauri::command]
pub async fn list_agent_conversations_page(
    context_type: String,
    context_id: String,
    include_archived: Option<bool>,
    archived_only: Option<bool>,
    offset: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<AgentConversationListPageResponse, String> {
    let context_type_enum = parse_context_type(&context_type)?;
    let archived_only = archived_only.unwrap_or(false);
    let include_archived = include_archived.unwrap_or(false) || archived_only;
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(6);

    let page = state
        .chat_conversation_repo
        .get_by_context_page_filtered(
            context_type_enum,
            &context_id,
            include_archived,
            archived_only,
            offset,
            limit,
            search.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let has_more = page.has_more();

    Ok(AgentConversationListPageResponse {
        conversations: page
            .conversations
            .into_iter()
            .map(AgentConversationResponse::from)
            .collect(),
        total: page.total_count,
        limit: page.limit,
        offset: page.offset,
        has_more,
    })
}

/// Archive a conversation.
#[tauri::command]
pub async fn archive_agent_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<AgentConversationResponse, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    state
        .chat_conversation_repo
        .archive(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;
    state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .map(AgentConversationResponse::from)
        .ok_or_else(|| "Conversation not found".to_string())
}

/// Restore an archived conversation.
#[tauri::command]
pub async fn restore_agent_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<AgentConversationResponse, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    state
        .chat_conversation_repo
        .restore(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;
    state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .map(AgentConversationResponse::from)
        .ok_or_else(|| "Conversation not found".to_string())
}

/// Get workspace metadata for a project-backed agent conversation.
#[tauri::command]
pub async fn get_agent_conversation_workspace(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AgentConversationWorkspaceResponse>, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())
        .map(|workspace| workspace.map(AgentConversationWorkspaceResponse::from))
}

/// List workspace metadata for project-backed agent conversations.
#[tauri::command]
pub async fn list_agent_conversation_workspaces_by_project(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentConversationWorkspaceResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .agent_conversation_workspace_repo
        .get_by_project_id(&project_id)
        .await
        .map_err(|e| e.to_string())
        .map(|workspaces| {
            workspaces
                .into_iter()
                .map(AgentConversationWorkspaceResponse::from)
                .collect()
        })
}

/// List durable publish events for a project-backed agent conversation workspace.
#[tauri::command]
pub async fn list_agent_conversation_workspace_publication_events(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AgentConversationWorkspacePublicationEventResponse>, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    state
        .agent_conversation_workspace_repo
        .list_publication_events(&conversation_id)
        .await
        .map_err(|e| e.to_string())
        .map(|events| {
            events
                .into_iter()
                .map(AgentConversationWorkspacePublicationEventResponse::from)
                .collect()
        })
}

/// Inspect whether the workspace's captured base commit is behind the current base ref.
#[tauri::command]
pub async fn get_agent_conversation_workspace_freshness(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<AgentConversationWorkspaceFreshnessResponse, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    let workspace = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Agent conversation workspace not found for conversation {}",
                conversation_id
            )
        })?;

    if workspace.mode != AgentConversationWorkspaceMode::Edit {
        return Err(
            "Only edit-agent conversation workspaces can be inspected for publish freshness"
                .to_string(),
        );
    }

    let project = state
        .project_repo
        .get_by_id(&workspace.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", workspace.project_id))?;
    let worktree_path = resolve_valid_agent_conversation_workspace_path(&project, &workspace)
        .await
        .map_err(|e| e.to_string())?;
    let status = inspect_publish_branch_freshness(
        &worktree_path,
        &workspace.base_ref,
        workspace.base_commit.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(AgentConversationWorkspaceFreshnessResponse::from_workspace_status(&workspace, status))
}

/// Update an edit-agent workspace branch from its captured base ref without publishing it.
#[tauri::command]
pub async fn update_agent_conversation_workspace_from_base(
    conversation_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    team_service: State<'_, std::sync::Arc<crate::application::TeamService>>,
    app: tauri::AppHandle,
) -> Result<UpdateAgentConversationWorkspaceFromBaseResponse, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    let mut workspace = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Agent conversation workspace not found for conversation {}",
                conversation_id
            )
        })?;

    if workspace.mode != AgentConversationWorkspaceMode::Edit {
        return Err(
            "Ideation-mode agent conversations are updated through the execution pipeline"
                .to_string(),
        );
    }
    if workspace.is_execution_owned() {
        return Err(
            "This agent conversation workspace is owned by an execution plan and cannot be directly updated"
                .to_string(),
        );
    }

    let conversation = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Conversation not found: {}", conversation_id))?;
    if conversation.context_type != ChatContextType::Project
        || conversation.context_id != workspace.project_id.as_str()
    {
        return Err(format!(
            "Conversation {} does not match agent workspace project {}",
            conversation.id, workspace.project_id
        ));
    }

    let repair_service = create_chat_service(
        &state,
        app,
        &execution_state,
        Some(team_service.inner().clone()),
    );
    let project = state
        .project_repo
        .get_by_id(&workspace.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", workspace.project_id))?;
    let worktree_path =
        match resolve_valid_agent_conversation_workspace_path(&project, &workspace).await {
            Ok(path) => path,
            Err(error) => {
                if error
                    .to_string()
                    .contains("Agent conversation workspace is missing")
                {
                    let _ = state
                        .agent_conversation_workspace_repo
                        .update_status(
                            &workspace.conversation_id,
                            crate::domain::entities::AgentConversationWorkspaceStatus::Missing,
                        )
                        .await;
                }
                return Err(error.to_string());
            }
        };

    mark_agent_workspace_publish_status(&state, &workspace, "refreshing")
        .await
        .map_err(|e| e.to_string())?;

    let freshness_conversation_id = workspace.conversation_id.as_str();
    let outcome = ensure_publish_branch_fresh(
        &worktree_path,
        &project,
        &workspace.branch_name,
        &workspace.base_ref,
        &freshness_conversation_id,
        None,
    )
    .await;
    let (updated, target_ref, base_commit) = match outcome {
        PublishBranchFreshnessOutcome::AlreadyFresh {
            base_commit,
            target_ref,
        } => (false, target_ref, base_commit),
        PublishBranchFreshnessOutcome::Updated {
            base_commit,
            target_ref,
        } => (true, target_ref, base_commit),
        PublishBranchFreshnessOutcome::NeedsAgent { message, .. }
        | PublishBranchFreshnessOutcome::OperationalError { message } => {
            mark_agent_workspace_publish_failure(
                &state,
                &workspace,
                &message,
                None,
                &repair_service,
            )
            .await;
            return Err(message);
        }
    };

    workspace.base_commit = Some(base_commit.clone());
    workspace = state
        .agent_conversation_workspace_repo
        .create_or_update(workspace)
        .await
        .map_err(|e| e.to_string())?;
    state
        .agent_conversation_workspace_repo
        .update_publication(
            &workspace.conversation_id,
            workspace.publication_pr_number,
            workspace.publication_pr_url.as_deref(),
            workspace.publication_pr_status.as_deref(),
            Some("refreshed"),
        )
        .await
        .map_err(|e| e.to_string())?;
    append_agent_workspace_publication_event(
        &state,
        &workspace.conversation_id,
        if updated {
            "updated_from_base"
        } else {
            "base_current"
        },
        "succeeded",
        if updated {
            "Workspace branch updated from base"
        } else {
            "Workspace branch is current with base"
        },
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    let refreshed = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&workspace.conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or(workspace);

    Ok(UpdateAgentConversationWorkspaceFromBaseResponse {
        workspace: AgentConversationWorkspaceResponse::from(refreshed),
        updated,
        target_ref,
        base_commit,
    })
}

/// Commit and publish a general edit agent conversation workspace.
#[tauri::command]
pub async fn publish_agent_conversation_workspace(
    conversation_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
    team_service: State<'_, std::sync::Arc<crate::application::TeamService>>,
    app: tauri::AppHandle,
) -> Result<PublishAgentConversationWorkspaceResponse, String> {
    let conversation_id = ChatConversationId::from_string(conversation_id);
    let mut workspace = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "Agent conversation workspace not found for conversation {}",
                conversation_id
            )
        })?;

    if workspace.mode != AgentConversationWorkspaceMode::Edit {
        return Err(
            "Ideation-mode agent conversations are published through the execution pipeline"
                .to_string(),
        );
    }
    if workspace.is_execution_owned() {
        return Err(
            "This agent conversation workspace is owned by an execution plan and cannot be directly published"
                .to_string(),
        );
    }

    let conversation = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Conversation not found: {}", conversation_id))?;
    if conversation.context_type != ChatContextType::Project
        || conversation.context_id != workspace.project_id.as_str()
    {
        return Err(format!(
            "Conversation {} does not match agent workspace project {}",
            conversation.id, workspace.project_id
        ));
    }

    let repair_service = create_chat_service(
        &state,
        app,
        &execution_state,
        Some(team_service.inner().clone()),
    );

    let project = state
        .project_repo
        .get_by_id(&workspace.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", workspace.project_id))?;
    let worktree_path =
        match resolve_valid_agent_conversation_workspace_path(&project, &workspace).await {
            Ok(path) => path,
            Err(error) => {
                if error
                    .to_string()
                    .contains("Agent conversation workspace is missing")
                {
                    let _ = state
                        .agent_conversation_workspace_repo
                        .update_status(
                            &workspace.conversation_id,
                            crate::domain::entities::AgentConversationWorkspaceStatus::Missing,
                        )
                        .await;
                }
                return Err(error.to_string());
            }
        };

    let github = match state.github_service.as_ref() {
        Some(github) => github,
        None => {
            let error = "GitHub integration is not available".to_string();
            mark_agent_workspace_publish_failure(&state, &workspace, &error, None, &repair_service)
                .await;
            return Err(error);
        }
    };

    mark_agent_workspace_publish_status(&state, &workspace, "checking")
        .await
        .map_err(|e| e.to_string())?;

    let has_uncommitted_changes = match GitService::has_uncommitted_changes(&worktree_path).await {
        Ok(has_changes) => has_changes,
        Err(error) => {
            let error = error.to_string();
            mark_agent_workspace_publish_failure(&state, &workspace, &error, None, &repair_service)
                .await;
            return Err(error);
        }
    };

    let commit_sha = if has_uncommitted_changes {
        mark_agent_workspace_publish_status(&state, &workspace, "committing")
            .await
            .map_err(|e| e.to_string())?;
        let message = build_agent_workspace_commit_message(&conversation);
        match GitService::commit_all_including_deletions(&worktree_path, &message).await {
            Ok(commit_sha) => commit_sha,
            Err(error) => {
                let error = error.to_string();
                mark_agent_workspace_publish_failure(
                    &state,
                    &workspace,
                    &error,
                    None,
                    &repair_service,
                )
                .await;
                return Err(error);
            }
        }
    } else {
        None
    };

    if let Err(error) =
        review_base_for_publish(workspace.base_commit.as_deref(), &workspace.base_ref)
    {
        mark_agent_workspace_publish_failure(&state, &workspace, &error, None, &repair_service)
            .await;
        return Err(error);
    }

    mark_agent_workspace_publish_status(&state, &workspace, "refreshing")
        .await
        .map_err(|e| e.to_string())?;

    let repo_path = std::path::Path::new(&project.working_directory);
    let freshness_conversation_id = workspace.conversation_id.as_str();
    let freshness_outcome = ensure_publish_branch_fresh(
        repo_path,
        &project,
        &workspace.branch_name,
        &workspace.base_ref,
        &freshness_conversation_id,
        None,
    )
    .await;
    let refreshed_base_commit = match freshness_outcome {
        PublishBranchFreshnessOutcome::AlreadyFresh { base_commit, .. }
        | PublishBranchFreshnessOutcome::Updated { base_commit, .. } => base_commit,
        PublishBranchFreshnessOutcome::NeedsAgent { message, .. } => {
            mark_agent_workspace_publish_failure(
                &state,
                &workspace,
                &message,
                None,
                &repair_service,
            )
            .await;
            return Err(message);
        }
        PublishBranchFreshnessOutcome::OperationalError { message } => {
            mark_agent_workspace_publish_failure(
                &state,
                &workspace,
                &message,
                None,
                &repair_service,
            )
            .await;
            return Err(message);
        }
    };

    if workspace.base_commit.as_deref() != Some(refreshed_base_commit.as_str()) {
        workspace.base_commit = Some(refreshed_base_commit);
        workspace = state
            .agent_conversation_workspace_repo
            .create_or_update(workspace)
            .await
            .map_err(|e| e.to_string())?;
    }

    let review_base =
        match review_base_for_publish(workspace.base_commit.as_deref(), &workspace.base_ref) {
            Ok(review_base) => review_base,
            Err(error) => {
                mark_agent_workspace_publish_failure(
                    &state,
                    &workspace,
                    &error,
                    None,
                    &repair_service,
                )
                .await;
                return Err(error);
            }
        };

    mark_agent_workspace_publish_status(&state, &workspace, "checking")
        .await
        .map_err(|e| e.to_string())?;

    let reviewable_commit_count =
        match count_publish_reviewable_commits(&worktree_path, &workspace.branch_name, review_base)
            .await
        {
            Ok(count) => count,
            Err(error) => {
                let error = error.to_string();
                mark_agent_workspace_publish_failure(
                    &state,
                    &workspace,
                    &error,
                    None,
                    &repair_service,
                )
                .await;
                return Err(error);
            }
        };
    if reviewable_commit_count == 0 {
        let _ = mark_agent_workspace_publish_status(&state, &workspace, "no_changes").await;
        return Err("No committed changes to publish on this agent branch".to_string());
    }

    mark_agent_workspace_publish_status(&state, &workspace, "pushing")
        .await
        .map_err(|e| e.to_string())?;

    if let Err(error) = push_publish_branch(github, &worktree_path, &workspace.branch_name).await {
        let error = error.to_string();
        mark_agent_workspace_publish_failure(&state, &workspace, &error, None, &repair_service)
            .await;
        return Err(error);
    }

    mark_agent_workspace_publish_status(&state, &workspace, "pushed")
        .await
        .map_err(|e| e.to_string())?;

    let publisher = AgentWorkspacePrPublisher::new(github);
    let pr_result = publisher
        .publish_draft_pr(&worktree_path, &conversation, &workspace)
        .await;
    let outcome = match pr_result {
        Ok(result) => result,
        Err(error) => {
            let error = error.to_string();
            mark_agent_workspace_publish_failure(
                &state,
                &workspace,
                &error,
                Some("failed"),
                &repair_service,
            )
            .await;
            return Err(error);
        }
    };

    state
        .agent_conversation_workspace_repo
        .update_publication(
            &workspace.conversation_id,
            Some(outcome.pr_number),
            Some(&outcome.pr_url),
            Some(outcome.pr_status),
            Some("pushed"),
        )
        .await
        .map_err(|e| e.to_string())?;
    append_agent_workspace_publication_event(
        &state,
        &workspace.conversation_id,
        "published",
        "succeeded",
        "Draft pull request is ready",
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    let review_chat_service: Arc<dyn ChatService> = Arc::new(repair_service);
    state.pr_poller_registry.start_agent_workspace_polling(
        workspace.conversation_id,
        outcome.pr_number,
        worktree_path.clone(),
        Arc::clone(&state.agent_conversation_workspace_repo),
        review_chat_service,
    );

    let refreshed = state
        .agent_conversation_workspace_repo
        .get_by_conversation_id(&workspace.conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .unwrap_or(workspace);

    Ok(PublishAgentConversationWorkspaceResponse {
        workspace: AgentConversationWorkspaceResponse::from(refreshed),
        commit_sha,
        pushed: true,
        created_pr: outcome.created_pr,
        pr_number: Some(outcome.pr_number),
        pr_url: Some(outcome.pr_url),
    })
}

async fn mark_agent_workspace_publish_status(
    state: &AppState,
    workspace: &AgentConversationWorkspace,
    push_status: &str,
) -> crate::error::AppResult<()> {
    state
        .agent_conversation_workspace_repo
        .update_publication(
            &workspace.conversation_id,
            workspace.publication_pr_number,
            workspace.publication_pr_url.as_deref(),
            workspace.publication_pr_status.as_deref(),
            Some(push_status),
        )
        .await?;
    append_agent_workspace_publication_event(
        state,
        &workspace.conversation_id,
        push_status,
        publication_event_status_for_push_status(push_status),
        publication_event_summary_for_push_status(push_status),
        None,
    )
    .await
}

#[doc(hidden)]
pub fn build_agent_workspace_publish_repair_message(
    error: &str,
    workspace: &AgentConversationWorkspace,
) -> String {
    let base = workspace
        .base_display_name
        .as_deref()
        .unwrap_or(workspace.base_ref.as_str());
    [
        "Commit & Publish failed for this edit workspace.".to_string(),
        String::new(),
        "Please fix the workspace so publishing can be retried.".to_string(),
        String::new(),
        format!("Error: {error}"),
        format!("Workspace branch: {}", workspace.branch_name),
        format!("Base: {base}"),
    ]
    .join("\n")
}

#[doc(hidden)]
pub async fn send_agent_workspace_publish_repair_message<S>(
    service: &S,
    workspace: &AgentConversationWorkspace,
    error: &str,
) -> Result<SendResult, ChatServiceError>
where
    S: ChatService + ?Sized,
{
    service
        .send_message(
            ChatContextType::Project,
            workspace.project_id.as_str(),
            &build_agent_workspace_publish_repair_message(error, workspace),
            SendMessageOptions {
                conversation_id_override: Some(workspace.conversation_id),
                agent_name_override: Some(
                    agent_name_for_workspace_mode(workspace.mode).to_string(),
                ),
                ..Default::default()
            },
        )
        .await
}

#[doc(hidden)]
pub async fn mark_agent_workspace_publish_failure<S>(
    state: &AppState,
    workspace: &AgentConversationWorkspace,
    error: &str,
    pr_status_override: Option<&str>,
    repair_service: &S,
) where
    S: ChatService + ?Sized,
{
    let push_status = publish_push_status_for_failure(error);
    let failure_class = classify_publish_failure(error);
    let classification = match failure_class {
        PublishFailureClass::AgentFixable => "agent_fixable",
        PublishFailureClass::Operational => "operational",
    };
    let _ = state
        .agent_conversation_workspace_repo
        .update_publication(
            &workspace.conversation_id,
            workspace.publication_pr_number,
            workspace.publication_pr_url.as_deref(),
            pr_status_override.or(workspace.publication_pr_status.as_deref()),
            Some(push_status),
        )
        .await;
    let _ = append_agent_workspace_publication_event(
        state,
        &workspace.conversation_id,
        push_status,
        "failed",
        error,
        Some(classification.to_string()),
    )
    .await;

    if !matches!(failure_class, PublishFailureClass::AgentFixable) {
        return;
    }

    match send_agent_workspace_publish_repair_message(repair_service, workspace, error).await {
        Ok(_) => {
            let _ = append_agent_workspace_publication_event(
                state,
                &workspace.conversation_id,
                "repair_sent",
                "succeeded",
                "Sent publish failure to workspace agent",
                Some("agent_fixable".to_string()),
            )
            .await;
        }
        Err(repair_error) => {
            tracing::warn!(
                conversation_id = %workspace.conversation_id,
                error = %repair_error,
                "Failed to send agent workspace publish repair message"
            );
            let _ = append_agent_workspace_publication_event(
                state,
                &workspace.conversation_id,
                "repair_sent",
                "failed",
                &format!("Failed to send publish failure to workspace agent: {repair_error}"),
                Some("operational".to_string()),
            )
            .await;
        }
    }
}

async fn append_agent_workspace_publication_event(
    state: &AppState,
    conversation_id: &ChatConversationId,
    step: &str,
    status: &str,
    summary: &str,
    classification: Option<String>,
) -> crate::error::AppResult<()> {
    state
        .agent_conversation_workspace_repo
        .append_publication_event(AgentConversationWorkspacePublicationEvent::new(
            *conversation_id,
            step,
            status,
            summary,
            classification,
        ))
        .await
}

fn publication_event_status_for_push_status(push_status: &str) -> &'static str {
    match push_status {
        "pushed" => "succeeded",
        "no_changes" => "skipped",
        "failed" | "needs_agent" => "failed",
        _ => "started",
    }
}

fn publication_event_summary_for_push_status(push_status: &str) -> &'static str {
    match push_status {
        "checking" => "Checking workspace changes",
        "committing" => "Committing workspace changes",
        "refreshing" => "Refreshing branch from base",
        "pushing" => "Pushing agent branch",
        "pushed" => "Agent branch pushed",
        "no_changes" => "No committed changes to publish",
        "needs_agent" => "Publish needs workspace agent repair",
        "failed" => "Publish failed",
        _ => "Publish status changed",
    }
}

/// Persist a child workflow milestone into a parent project-agent conversation.
///
/// This command is intentionally not an agent send path: it does not touch queue
/// or capacity state and only records bridge/status messages that the UI derives
/// from authoritative orchestration events. `event_key` is used for idempotency
/// so replayed event-table polls and live Tauri events can converge safely.
#[tauri::command]
pub async fn append_agent_bridge_message(
    input: AppendAgentBridgeMessageInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<AgentMessageResponse>, String> {
    let conversation_id = ChatConversationId::from_string(&input.conversation_id);
    let content = input.content.trim().to_string();
    if content.is_empty() {
        return Err("Bridge message content cannot be empty".to_string());
    }
    if input.event_key.trim().is_empty() {
        return Err("Bridge event key cannot be empty".to_string());
    }

    let conversation = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Conversation not found".to_string())?;

    if conversation.context_type != ChatContextType::Project {
        return Err("Bridge messages can only be appended to project conversations".to_string());
    }

    let existing_messages = state
        .chat_message_repo
        .get_by_conversation(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    for message in existing_messages {
        let Some(metadata) = message.metadata.as_deref() else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<JsonValue>(metadata) else {
            continue;
        };
        let existing_key = parsed
            .get("bridge_event_key")
            .or_else(|| parsed.get("bridgeEventKey"))
            .and_then(JsonValue::as_str);
        if existing_key == Some(input.event_key.as_str()) {
            return Ok(None);
        }
    }

    let mut metadata = JsonMap::new();
    metadata.insert(
        "source".to_string(),
        JsonValue::String("project_agent_ideation_bridge".to_string()),
    );
    metadata.insert(
        "bridge_event_key".to_string(),
        JsonValue::String(input.event_key.clone()),
    );
    metadata.insert(
        "bridge_event_type".to_string(),
        JsonValue::String(input.event_type.clone()),
    );
    metadata.insert(
        "source_session_id".to_string(),
        JsonValue::String(input.source_session_id.clone()),
    );
    if let Some(payload) = input.metadata {
        metadata.insert("payload".to_string(), payload);
    }

    let mut bridge_message = create_assistant_message(
        conversation.context_type,
        &conversation.context_id,
        &content,
        conversation_id.clone(),
        &[],
        &[],
    );
    bridge_message.metadata = Some(JsonValue::Object(metadata).to_string());

    let created = state
        .chat_message_repo
        .create(bridge_message)
        .await
        .map_err(|e| e.to_string())?;

    let metadata = created.metadata.clone();
    let created_response = AgentMessageResponse {
        id: created.id.as_str().to_string(),
        role: created.role.to_string(),
        content: created.content.clone(),
        metadata: metadata.clone(),
        tool_calls: None,
        content_blocks: None,
        attribution_source: created.attribution_source.clone(),
        provider_harness: created
            .provider_harness
            .as_ref()
            .map(|value| value.to_string()),
        provider_session_id: created.provider_session_id.clone(),
        upstream_provider: created.upstream_provider.clone(),
        provider_profile: created.provider_profile.clone(),
        logical_model: created.logical_model.clone(),
        effective_model_id: created.effective_model_id.clone(),
        logical_effort: created
            .logical_effort
            .as_ref()
            .map(|value| value.to_string()),
        effective_effort: created.effective_effort.clone(),
        input_tokens: created.input_tokens,
        output_tokens: created.output_tokens,
        cache_creation_tokens: created.cache_creation_tokens,
        cache_read_tokens: created.cache_read_tokens,
        estimated_usd: created.estimated_usd,
        created_at: created.created_at.to_rfc3339(),
    };

    let _ = app.emit(
        "agent:message_created",
        AgentMessageCreatedPayload {
            message_id: created.id.as_str().to_string(),
            conversation_id: conversation_id.as_str().to_string(),
            context_type: conversation.context_type.to_string(),
            context_id: conversation.context_id,
            role: created.role.to_string(),
            content: created.content,
            created_at: Some(created.created_at.to_rfc3339()),
            metadata,
        },
    );

    Ok(Some(created_response))
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

/// Get a tail-first page of conversation messages for fast conversation switching.
/// `offset` counts how many newest messages to skip before loading older history.
#[tauri::command]
pub async fn get_agent_conversation_messages_page(
    conversation_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Option<AgentConversationMessagesPageResponse>, String> {
    use crate::domain::entities::ChatConversationId;

    let conversation_id = ChatConversationId::from_string(&conversation_id);
    let limit = limit.unwrap_or(40).clamp(1, 200);
    let offset = offset.unwrap_or(0);

    let Some(conversation) = state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };

    let raw_messages = state
        .chat_message_repo
        .get_recent_by_conversation_paginated(&conversation_id, limit, offset)
        .await
        .map_err(|e| e.to_string())?;

    let mut messages = Vec::with_capacity(raw_messages.len());
    for message in raw_messages {
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

    let fetched_count = offset as i64 + messages.len() as i64;
    let total_message_count = conversation.message_count.max(0);
    let has_older = fetched_count < total_message_count;

    Ok(Some(AgentConversationMessagesPageResponse {
        conversation: AgentConversationResponse::from(conversation),
        messages,
        limit,
        offset,
        total_message_count,
        has_older,
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
    let (model_id, model_label) =
        if let Ok(Some(conv)) = state.chat_conversation_repo.get_by_id(&conv_id).await {
            let runtime_context_id = if conv.context_type == ChatContextType::Project {
                conv.id.as_str().to_string()
            } else {
                conv.context_id.clone()
            };
            let key = RunningAgentKey::new(conv.context_type.to_string(), runtime_context_id);
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
    pub title: Option<String>,
}

/// Input for update_agent_conversation_title command
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentConversationTitleInput {
    pub conversation_id: String,
    pub title: String,
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

    let mut conversation = match context_type {
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

    if let Some(title) = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        conversation.set_title(title.to_string());
    }

    state
        .chat_conversation_repo
        .create(conversation)
        .await
        .map(AgentConversationResponse::from)
        .map_err(|e| e.to_string())
}

/// Update an existing conversation title.
#[tauri::command]
pub async fn update_agent_conversation_title(
    input: UpdateAgentConversationTitleInput,
    state: State<'_, AppState>,
) -> Result<AgentConversationResponse, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("Conversation title cannot be empty".to_string());
    }

    let conversation_id = ChatConversationId::from_string(input.conversation_id);
    state
        .chat_conversation_repo
        .update_title(&conversation_id, title)
        .await
        .map_err(|e| e.to_string())?;

    state
        .chat_conversation_repo
        .get_by_id(&conversation_id)
        .await
        .map_err(|e| e.to_string())?
        .map(AgentConversationResponse::from)
        .ok_or_else(|| "Conversation not found".to_string())
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
            parsed
                .get("delegated_session_id")
                .and_then(|value| value.as_str()),
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

        assert_eq!(
            result.get("status").and_then(|value| value.as_str()),
            Some("completed")
        );
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
