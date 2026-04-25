use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tauri::{Emitter, State};

use crate::application::chat_service::create_user_message;
use crate::application::{AgentMessageCreatedPayload, AppState};
use crate::commands::unified_chat_commands::AgentMessageResponse;
use crate::domain::entities::{
    ChatContextType, ChatConversation, ChatConversationId, ChatMessage, DesignApprovalStatus,
    DesignFeedbackStatus, DesignSchemaVersionId, DesignSourceRef, DesignStyleguideFeedback,
    DesignStyleguideFeedbackId, DesignStyleguideItem, DesignSystemId,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveDesignStyleguideItemInput {
    pub design_system_id: String,
    pub item_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignStyleguideFeedbackInput {
    pub design_system_id: String,
    pub item_id: String,
    pub feedback: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveDesignStyleguideFeedbackInput {
    pub feedback_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDesignStyleguideItemsInput {
    pub design_system_id: String,
    pub schema_version_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignStyleguideItemResponse {
    pub id: String,
    pub design_system_id: String,
    pub schema_version_id: String,
    pub item_id: String,
    pub group: String,
    pub label: String,
    pub summary: String,
    pub preview_artifact_id: Option<String>,
    pub source_refs: Vec<DesignSourceRef>,
    pub confidence: String,
    pub approval_status: String,
    pub feedback_status: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignStyleguideFeedbackResponse {
    pub id: String,
    pub design_system_id: String,
    pub schema_version_id: String,
    pub item_id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub preview_artifact_id: Option<String>,
    pub source_refs: Vec<DesignSourceRef>,
    pub feedback: String,
    pub status: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignStyleguideFeedbackResponse {
    pub feedback: DesignStyleguideFeedbackResponse,
    pub item: DesignStyleguideItemResponse,
    pub message: AgentMessageResponse,
}

impl From<DesignStyleguideItem> for DesignStyleguideItemResponse {
    fn from(item: DesignStyleguideItem) -> Self {
        Self {
            id: item.id.as_str().to_string(),
            design_system_id: item.design_system_id.as_str().to_string(),
            schema_version_id: item.schema_version_id.as_str().to_string(),
            item_id: item.item_id,
            group: enum_text(&item.group),
            label: item.label,
            summary: item.summary,
            preview_artifact_id: item.preview_artifact_id,
            source_refs: item.source_refs,
            confidence: enum_text(&item.confidence),
            approval_status: enum_text(&item.approval_status),
            feedback_status: enum_text(&item.feedback_status),
            updated_at: item.updated_at.to_rfc3339(),
        }
    }
}

impl From<DesignStyleguideFeedback> for DesignStyleguideFeedbackResponse {
    fn from(feedback: DesignStyleguideFeedback) -> Self {
        Self {
            id: feedback.id.as_str().to_string(),
            design_system_id: feedback.design_system_id.as_str().to_string(),
            schema_version_id: feedback.schema_version_id.as_str().to_string(),
            item_id: feedback.item_id,
            conversation_id: feedback.conversation_id.as_str(),
            message_id: feedback.message_id.map(|id| id.as_str().to_string()),
            preview_artifact_id: feedback.preview_artifact_id,
            source_refs: feedback.source_refs,
            feedback: feedback.feedback,
            status: enum_text(&feedback.status),
            created_at: feedback.created_at.to_rfc3339(),
            resolved_at: feedback.resolved_at.map(|value| value.to_rfc3339()),
        }
    }
}

#[tauri::command]
pub async fn approve_design_styleguide_item(
    input: ApproveDesignStyleguideItemInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DesignStyleguideItemResponse, String> {
    let item = approve_design_styleguide_item_core(&state, input).await?;
    let _ = app.emit("design:styleguide_item_approved", &item);
    Ok(item)
}

#[tauri::command]
pub async fn create_design_styleguide_feedback(
    input: CreateDesignStyleguideFeedbackInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CreateDesignStyleguideFeedbackResponse, String> {
    let response = create_design_styleguide_feedback_core(&state, input).await?;
    let _ = app.emit(
        "design:styleguide_item_feedback_created",
        &response.feedback,
    );
    let _ = app.emit(
        "agent:message_created",
        AgentMessageCreatedPayload {
            message_id: response.message.id.clone(),
            conversation_id: response.feedback.conversation_id.clone(),
            context_type: ChatContextType::Design.to_string(),
            context_id: response.feedback.design_system_id.clone(),
            role: response.message.role.clone(),
            content: response.message.content.clone(),
            created_at: Some(response.message.created_at.clone()),
            metadata: response.message.metadata.clone(),
        },
    );
    Ok(response)
}

#[tauri::command]
pub async fn resolve_design_styleguide_feedback(
    input: ResolveDesignStyleguideFeedbackInput,
    state: State<'_, AppState>,
) -> Result<DesignStyleguideFeedbackResponse, String> {
    resolve_design_styleguide_feedback_core(&state, input).await
}

#[tauri::command]
pub async fn list_design_styleguide_items(
    input: ListDesignStyleguideItemsInput,
    state: State<'_, AppState>,
) -> Result<Vec<DesignStyleguideItemResponse>, String> {
    list_design_styleguide_items_core(&state, input).await
}

#[doc(hidden)]
pub async fn list_design_styleguide_items_core(
    state: &AppState,
    input: ListDesignStyleguideItemsInput,
) -> Result<Vec<DesignStyleguideItemResponse>, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let Some(system) = state
        .design_system_repo
        .get_by_id(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Err(format!(
            "Design system not found: {}",
            design_system_id.as_str()
        ));
    };

    let requested_schema_version_id = input
        .schema_version_id
        .as_deref()
        .map(parse_schema_version_id)
        .transpose()?;
    let schema_version_id = requested_schema_version_id
        .as_ref()
        .or(system.current_schema_version_id.as_ref());
    let Some(schema_version_id) = schema_version_id else {
        return Ok(Vec::new());
    };

    state
        .design_styleguide_repo
        .list_items(&design_system_id, Some(schema_version_id))
        .await
        .map(|items| {
            items
                .into_iter()
                .map(DesignStyleguideItemResponse::from)
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[doc(hidden)]
pub async fn approve_design_styleguide_item_core(
    state: &AppState,
    input: ApproveDesignStyleguideItemInput,
) -> Result<DesignStyleguideItemResponse, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let item_id = normalize_item_id(&input.item_id)?;
    let item = load_styleguide_item(state, &design_system_id, &item_id).await?;

    state
        .design_styleguide_repo
        .approve_item(&item.id)
        .await
        .map_err(|error| error.to_string())?;

    state
        .design_styleguide_repo
        .get_item(&design_system_id, &item_id)
        .await
        .map_err(|error| error.to_string())?
        .map(DesignStyleguideItemResponse::from)
        .ok_or_else(|| format!("Design styleguide item not found: {item_id}"))
}

#[doc(hidden)]
pub async fn create_design_styleguide_feedback_core(
    state: &AppState,
    input: CreateDesignStyleguideFeedbackInput,
) -> Result<CreateDesignStyleguideFeedbackResponse, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let item_id = normalize_item_id(&input.item_id)?;
    let feedback_text = normalize_feedback(&input.feedback)?;
    let conversation =
        resolve_design_conversation(state, &design_system_id, input.conversation_id.as_deref())
            .await?;
    let mut item = load_styleguide_item(state, &design_system_id, &item_id).await?;

    let feedback = DesignStyleguideFeedback {
        id: DesignStyleguideFeedbackId::new(),
        design_system_id: design_system_id.clone(),
        schema_version_id: item.schema_version_id.clone(),
        item_id: item.item_id.clone(),
        conversation_id: conversation.id,
        message_id: None,
        preview_artifact_id: item.preview_artifact_id.clone(),
        source_refs: item.source_refs.clone(),
        feedback: feedback_text.clone(),
        status: DesignFeedbackStatus::Open,
        created_at: Utc::now(),
        resolved_at: None,
    };
    let mut feedback = state
        .design_styleguide_feedback_repo
        .create(feedback)
        .await
        .map_err(|error| error.to_string())?;

    item.approval_status = DesignApprovalStatus::NeedsWork;
    item.feedback_status = DesignFeedbackStatus::Open;
    item.updated_at = Utc::now();
    state
        .design_styleguide_repo
        .update_item(&item)
        .await
        .map_err(|error| error.to_string())?;

    let metadata = feedback_message_metadata(&feedback, &item);
    let message_content = format!("Feedback on {}: {}", item.label, feedback_text);
    let message = create_user_message(
        ChatContextType::Design,
        design_system_id.as_str(),
        &message_content,
        conversation.id,
        Some(metadata.to_string()),
        None,
    );
    let message = state
        .chat_message_repo
        .create(message)
        .await
        .map_err(|error| error.to_string())?;

    feedback.message_id = Some(message.id.clone());
    state
        .design_styleguide_feedback_repo
        .update(&feedback)
        .await
        .map_err(|error| error.to_string())?;
    update_conversation_stats(state, &conversation.id, message.created_at).await?;

    Ok(CreateDesignStyleguideFeedbackResponse {
        feedback: DesignStyleguideFeedbackResponse::from(feedback),
        item: DesignStyleguideItemResponse::from(item),
        message: agent_message_response(message),
    })
}

#[doc(hidden)]
pub async fn resolve_design_styleguide_feedback_core(
    state: &AppState,
    input: ResolveDesignStyleguideFeedbackInput,
) -> Result<DesignStyleguideFeedbackResponse, String> {
    let feedback_id = parse_feedback_id(&input.feedback_id)?;
    let mut feedback = state
        .design_styleguide_feedback_repo
        .get_by_id(&feedback_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "Design styleguide feedback not found: {}",
                feedback_id.as_str()
            )
        })?;

    feedback.status = DesignFeedbackStatus::Resolved;
    feedback.resolved_at = Some(Utc::now());
    state
        .design_styleguide_feedback_repo
        .update(&feedback)
        .await
        .map_err(|error| error.to_string())?;

    let open_feedback = state
        .design_styleguide_feedback_repo
        .list_open_by_design_system(&feedback.design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    if !open_feedback
        .iter()
        .any(|candidate| candidate.item_id == feedback.item_id)
    {
        if let Some(mut item) = state
            .design_styleguide_repo
            .get_item(&feedback.design_system_id, &feedback.item_id)
            .await
            .map_err(|error| error.to_string())?
        {
            item.feedback_status = DesignFeedbackStatus::Resolved;
            item.updated_at = Utc::now();
            state
                .design_styleguide_repo
                .update_item(&item)
                .await
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(DesignStyleguideFeedbackResponse::from(feedback))
}

async fn load_styleguide_item(
    state: &AppState,
    design_system_id: &DesignSystemId,
    item_id: &str,
) -> Result<DesignStyleguideItem, String> {
    state
        .design_styleguide_repo
        .get_item(design_system_id, item_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Design styleguide item not found: {item_id}"))
}

async fn resolve_design_conversation(
    state: &AppState,
    design_system_id: &DesignSystemId,
    conversation_id: Option<&str>,
) -> Result<ChatConversation, String> {
    let conversation = if let Some(conversation_id) = conversation_id {
        let conversation_id = ChatConversationId::from_string(conversation_id);
        state
            .chat_conversation_repo
            .get_by_id(&conversation_id)
            .await
            .map_err(|error| error.to_string())?
    } else {
        state
            .chat_conversation_repo
            .get_active_for_context(ChatContextType::Design, design_system_id.as_str())
            .await
            .map_err(|error| error.to_string())?
    }
    .ok_or_else(|| "Design conversation not found".to_string())?;

    if conversation.context_type != ChatContextType::Design
        || conversation.context_id != design_system_id.as_str()
    {
        return Err("Feedback must target the selected design conversation".to_string());
    }
    Ok(conversation)
}

async fn update_conversation_stats(
    state: &AppState,
    conversation_id: &ChatConversationId,
    last_message_at: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let message_count = state
        .chat_message_repo
        .get_by_conversation(conversation_id)
        .await
        .map_err(|error| error.to_string())?
        .len() as i64;
    state
        .chat_conversation_repo
        .update_message_stats(conversation_id, message_count, last_message_at)
        .await
        .map_err(|error| error.to_string())
}

fn feedback_message_metadata(
    feedback: &DesignStyleguideFeedback,
    item: &DesignStyleguideItem,
) -> JsonValue {
    json!({
        "source": "design_styleguide_feedback",
        "event_type": "design:styleguide_item_feedback_created",
        "feedback_id": feedback.id.as_str(),
        "design_system_id": feedback.design_system_id.as_str(),
        "schema_version_id": feedback.schema_version_id.as_str(),
        "item_id": feedback.item_id,
        "styleguide_item_id": item.id.as_str(),
        "preview_artifact_id": feedback.preview_artifact_id,
        "source_refs": feedback.source_refs,
    })
}

fn agent_message_response(message: ChatMessage) -> AgentMessageResponse {
    AgentMessageResponse {
        id: message.id.as_str().to_string(),
        role: message.role.to_string(),
        content: message.content,
        metadata: message.metadata,
        tool_calls: parse_json_field(message.tool_calls),
        content_blocks: parse_json_field(message.content_blocks),
        attribution_source: message.attribution_source,
        provider_harness: message
            .provider_harness
            .as_ref()
            .map(|value| value.to_string()),
        provider_session_id: message.provider_session_id,
        upstream_provider: message.upstream_provider,
        provider_profile: message.provider_profile,
        logical_model: message.logical_model,
        effective_model_id: message.effective_model_id,
        logical_effort: message
            .logical_effort
            .as_ref()
            .map(|value| value.to_string()),
        effective_effort: message.effective_effort,
        input_tokens: message.input_tokens,
        output_tokens: message.output_tokens,
        cache_creation_tokens: message.cache_creation_tokens,
        cache_read_tokens: message.cache_read_tokens,
        estimated_usd: message.estimated_usd,
        created_at: message.created_at.to_rfc3339(),
    }
}

fn parse_json_field(value: Option<String>) -> Option<JsonValue> {
    value.and_then(|value| serde_json::from_str(&value).ok())
}

fn parse_design_system_id(value: &str) -> Result<DesignSystemId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("design_system_id is required".to_string());
    }
    Ok(DesignSystemId::from_string(value.to_string()))
}

fn parse_feedback_id(value: &str) -> Result<DesignStyleguideFeedbackId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("feedback_id is required".to_string());
    }
    Ok(DesignStyleguideFeedbackId::from_string(value.to_string()))
}

fn parse_schema_version_id(value: &str) -> Result<DesignSchemaVersionId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("schema_version_id is required".to_string());
    }
    Ok(DesignSchemaVersionId::from_string(value.to_string()))
}

fn normalize_item_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("item_id is required".to_string());
    }
    Ok(value.to_string())
}

fn normalize_feedback(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Feedback cannot be empty".to_string());
    }
    Ok(value.to_string())
}

fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::entities::{
        ChatConversation, DesignConfidence, DesignSchemaVersionId, DesignStorageRootRef,
        DesignStyleguideGroup, DesignStyleguideItemId, DesignSystem, DesignSystemStatus, ProjectId,
    };

    fn styleguide_item(design_system_id: DesignSystemId) -> DesignStyleguideItem {
        DesignStyleguideItem {
            id: DesignStyleguideItemId::new(),
            design_system_id,
            schema_version_id: DesignSchemaVersionId::new(),
            item_id: "button.primary".to_string(),
            group: DesignStyleguideGroup::Components,
            label: "Primary button".to_string(),
            summary: "Main action button".to_string(),
            preview_artifact_id: Some("preview-button".to_string()),
            source_refs: vec![DesignSourceRef {
                project_id: ProjectId::from_string("project-1".to_string()),
                path: "frontend/src/Button.tsx".to_string(),
                line: Some(12),
            }],
            confidence: DesignConfidence::High,
            approval_status: DesignApprovalStatus::NeedsReview,
            feedback_status: DesignFeedbackStatus::None,
            updated_at: Utc::now(),
        }
    }

    fn styleguide_item_for_schema(
        design_system_id: DesignSystemId,
        schema_version_id: DesignSchemaVersionId,
        item_id: &str,
    ) -> DesignStyleguideItem {
        let mut item = styleguide_item(design_system_id);
        item.schema_version_id = schema_version_id;
        item.item_id = item_id.to_string();
        item.label = item_id.to_string();
        item
    }

    async fn seed_design_system(
        state: &AppState,
        design_system_id: DesignSystemId,
        current_schema_version_id: Option<DesignSchemaVersionId>,
    ) {
        let now = Utc::now();
        state
            .design_system_repo
            .create(DesignSystem {
                id: design_system_id,
                primary_project_id: ProjectId::from_string("project-1".to_string()),
                name: "Product UI".to_string(),
                description: None,
                status: DesignSystemStatus::Ready,
                current_schema_version_id,
                storage_root_ref: DesignStorageRootRef::from_hash_component("design-root"),
                created_at: now,
                updated_at: now,
                archived_at: None,
            })
            .await
            .unwrap();
    }

    async fn seed_design_conversation(state: &AppState, design_system_id: &DesignSystemId) {
        state
            .chat_conversation_repo
            .create(ChatConversation::new_design(design_system_id.clone()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_styleguide_items_uses_current_schema_version_by_default() {
        let state = AppState::new_test();
        let design_system_id = DesignSystemId::new();
        let current_schema_version_id = DesignSchemaVersionId::new();
        let stale_schema_version_id = DesignSchemaVersionId::new();
        seed_design_system(
            &state,
            design_system_id.clone(),
            Some(current_schema_version_id.clone()),
        )
        .await;

        let current_item = styleguide_item_for_schema(
            design_system_id.clone(),
            current_schema_version_id.clone(),
            "button.primary",
        );
        let stale_item = styleguide_item_for_schema(
            design_system_id.clone(),
            stale_schema_version_id.clone(),
            "button.legacy",
        );
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&current_schema_version_id, vec![current_item])
            .await
            .unwrap();
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&stale_schema_version_id, vec![stale_item])
            .await
            .unwrap();

        let response = list_design_styleguide_items_core(
            &state,
            ListDesignStyleguideItemsInput {
                design_system_id: design_system_id.as_str().to_string(),
                schema_version_id: None,
            },
        )
        .await
        .expect("list styleguide items");

        assert_eq!(response.len(), 1);
        assert_eq!(response[0].item_id, "button.primary");
        assert_eq!(response[0].schema_version_id, current_schema_version_id.as_str());
    }

    #[tokio::test]
    async fn feedback_bridge_updates_item_and_appends_design_chat_message() {
        let state = AppState::new_test();
        let design_system_id = DesignSystemId::new();
        let item = styleguide_item(design_system_id.clone());
        let schema_version_id = item.schema_version_id.clone();
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&schema_version_id, vec![item])
            .await
            .unwrap();
        seed_design_conversation(&state, &design_system_id).await;

        let response = create_design_styleguide_feedback_core(
            &state,
            CreateDesignStyleguideFeedbackInput {
                design_system_id: design_system_id.as_str().to_string(),
                item_id: "button.primary".to_string(),
                feedback: " Increase focus ring contrast. ".to_string(),
                conversation_id: None,
            },
        )
        .await
        .expect("feedback bridge");

        assert_eq!(response.feedback.status, "open");
        assert_eq!(
            response.feedback.message_id.as_deref(),
            Some(response.message.id.as_str())
        );
        assert_eq!(response.item.approval_status, "needs_work");
        assert_eq!(response.item.feedback_status, "open");
        assert_eq!(response.message.role, "user");
        assert!(response
            .message
            .content
            .contains("Increase focus ring contrast"));

        let messages = state
            .chat_message_repo
            .get_by_conversation(&ChatConversationId::from_string(
                response.feedback.conversation_id.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
        let metadata =
            serde_json::from_str::<JsonValue>(messages[0].metadata.as_deref().expect("metadata"))
                .expect("metadata json");
        assert_eq!(metadata["source"], "design_styleguide_feedback");
        assert_eq!(metadata["item_id"], "button.primary");
    }

    #[tokio::test]
    async fn approving_styleguide_item_updates_only_item_state() {
        let state = AppState::new_test();
        let design_system_id = DesignSystemId::new();
        let item = styleguide_item(design_system_id.clone());
        let schema_version_id = item.schema_version_id.clone();
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&schema_version_id, vec![item])
            .await
            .unwrap();

        let response = approve_design_styleguide_item_core(
            &state,
            ApproveDesignStyleguideItemInput {
                design_system_id: design_system_id.as_str().to_string(),
                item_id: "button.primary".to_string(),
            },
        )
        .await
        .expect("approve item");

        assert_eq!(response.approval_status, "approved");
        assert_eq!(response.feedback_status, "resolved");
        assert!(state
            .chat_message_repo
            .get_by_conversation(&ChatConversationId::new())
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn feedback_bridge_rejects_wrong_conversation_context() {
        let state = AppState::new_test();
        let design_system_id = DesignSystemId::new();
        let other_design_system_id = DesignSystemId::new();
        let item = styleguide_item(design_system_id.clone());
        let schema_version_id = item.schema_version_id.clone();
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&schema_version_id, vec![item])
            .await
            .unwrap();
        let other_conversation = state
            .chat_conversation_repo
            .create(ChatConversation::new_design(other_design_system_id))
            .await
            .unwrap();

        let error = create_design_styleguide_feedback_core(
            &state,
            CreateDesignStyleguideFeedbackInput {
                design_system_id: design_system_id.as_str().to_string(),
                item_id: "button.primary".to_string(),
                feedback: "Needs a calmer hover state".to_string(),
                conversation_id: Some(other_conversation.id.as_str()),
            },
        )
        .await
        .expect_err("wrong conversation should fail");

        assert!(error.contains("selected design conversation"));
    }
}
