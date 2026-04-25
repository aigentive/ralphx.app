use std::collections::BTreeMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use super::*;
use crate::commands::design_artifact_commands::{
    generate_design_artifact_core, GenerateDesignArtifactInput, GenerateDesignArtifactResponse,
};
use crate::commands::design_commands::{
    DesignSystemDetailResponse, DesignSystemResponse, DesignSystemSourceResponse,
};
use crate::commands::design_feedback_commands::{
    create_design_styleguide_feedback_core_with_options, list_design_styleguide_items_core,
    CreateDesignStyleguideFeedbackInput, CreateDesignStyleguideFeedbackOptions,
    CreateDesignStyleguideFeedbackResponse, DesignStyleguideItemResponse,
    ListDesignStyleguideItemsInput,
};
use crate::domain::entities::{
    ArtifactContent, ArtifactId, ChatContextType, DesignApprovalStatus, DesignFeedbackStatus,
    DesignSchemaVersion, DesignSchemaVersionId, DesignStyleguideItem, DesignSystemId,
};
use crate::utils::design_storage_paths::DesignStoragePaths;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSchemaQuery {
    #[serde(alias = "schema_version_id")]
    pub schema_version_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDesignStyleguideItemRequest {
    #[serde(alias = "item_id")]
    pub item_id: String,
    #[serde(alias = "approval_status")]
    pub approval_status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordDesignStyleguideFeedbackRequest {
    #[serde(alias = "item_id")]
    pub item_id: String,
    pub feedback: String,
    #[serde(alias = "conversation_id")]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignArtifactRequest {
    #[serde(alias = "artifact_kind")]
    pub artifact_kind: String,
    pub name: String,
    pub brief: Option<String>,
    #[serde(alias = "source_item_id")]
    pub source_item_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSourceManifestResponse {
    pub design_system: DesignSystemResponse,
    pub sources: Vec<DesignSystemSourceResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignStyleguideToolResponse {
    pub design_system_id: String,
    pub schema_version_id: Option<String>,
    pub items: Vec<DesignStyleguideItemResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignArtifactSummaryResponse {
    pub artifact_id: String,
    pub name: String,
    pub artifact_type: String,
    pub content_kind: String,
    pub version: u32,
    pub created_by: String,
    pub created_at: String,
    pub derived_from: Vec<String>,
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignArtifactsResponse {
    pub design_system_id: String,
    pub schema_version_id: Option<String>,
    pub artifacts: Vec<DesignArtifactSummaryResponse>,
}

pub async fn get_design_system_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
) -> Result<Json<DesignSystemDetailResponse>, (StatusCode, String)> {
    get_design_system_for_tool_core(&state.app_state, &design_system_id)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn get_design_source_manifest_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
) -> Result<Json<DesignSourceManifestResponse>, (StatusCode, String)> {
    get_design_source_manifest_for_tool_core(&state.app_state, &design_system_id)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn get_design_styleguide_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Query(query): Query<DesignSchemaQuery>,
) -> Result<Json<DesignStyleguideToolResponse>, (StatusCode, String)> {
    get_design_styleguide_for_tool_core(&state.app_state, &design_system_id, query)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn update_design_styleguide_item_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<UpdateDesignStyleguideItemRequest>,
) -> Result<Json<DesignStyleguideItemResponse>, (StatusCode, String)> {
    let item =
        update_design_styleguide_item_for_tool_core(&state.app_state, &design_system_id, req)
            .await
            .map_err(map_design_tool_error)?;

    if let Some(handle) = &state.app_state.app_handle {
        let event_name = if item.approval_status == "approved" {
            "design:styleguide_item_approved"
        } else {
            "design:styleguide_item_updated"
        };
        let _ = handle.emit(event_name, &item);
    }

    Ok(Json(item))
}

pub async fn record_design_styleguide_feedback_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<RecordDesignStyleguideFeedbackRequest>,
) -> Result<Json<CreateDesignStyleguideFeedbackResponse>, (StatusCode, String)> {
    let response =
        record_design_styleguide_feedback_for_tool_core(&state.app_state, &design_system_id, req)
            .await
            .map_err(map_design_tool_error)?;

    if let Some(handle) = &state.app_state.app_handle {
        let _ = handle.emit(
            "design:styleguide_item_feedback_created",
            &response.feedback,
        );
        if let Some(message) = response.message.as_ref() {
            let _ = handle.emit(
                "agent:message_created",
                crate::application::AgentMessageCreatedPayload {
                    message_id: message.id.clone(),
                    conversation_id: response.feedback.conversation_id.clone(),
                    context_type: ChatContextType::Design.to_string(),
                    context_id: response.feedback.design_system_id.clone(),
                    role: message.role.clone(),
                    content: message.content.clone(),
                    created_at: Some(message.created_at.clone()),
                    metadata: message.metadata.clone(),
                },
            );
        }
    }

    Ok(Json(response))
}

pub async fn list_design_artifacts_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Query(query): Query<DesignSchemaQuery>,
) -> Result<Json<DesignArtifactsResponse>, (StatusCode, String)> {
    list_design_artifacts_for_tool_core(&state.app_state, &design_system_id, query)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn create_design_artifact_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<CreateDesignArtifactRequest>,
) -> Result<Json<GenerateDesignArtifactResponse>, (StatusCode, String)> {
    let storage_paths =
        design_storage_paths_for_http(&state.app_state).map_err(map_design_tool_error)?;
    let response = generate_design_artifact_core(
        &state.app_state,
        &storage_paths,
        GenerateDesignArtifactInput {
            design_system_id,
            artifact_kind: req.artifact_kind,
            name: req.name,
            brief: req.brief,
            source_item_id: req.source_item_id,
        },
    )
    .await
    .map_err(map_design_tool_error)?;

    if let Some(handle) = &state.app_state.app_handle {
        let _ = handle.emit("design:artifact_created", &response);
    }

    Ok(Json(response))
}

async fn get_design_system_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
) -> Result<DesignSystemDetailResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
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

    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    let conversation = state
        .chat_conversation_repo
        .get_active_for_context(ChatContextType::Design, design_system_id.as_str())
        .await
        .map_err(|error| error.to_string())?;

    Ok(DesignSystemDetailResponse {
        design_system: DesignSystemResponse::from(system),
        sources: sources
            .into_iter()
            .map(DesignSystemSourceResponse::from)
            .collect(),
        conversation: conversation
            .map(crate::commands::unified_chat_commands::AgentConversationResponse::from),
    })
}

async fn get_design_source_manifest_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
) -> Result<DesignSourceManifestResponse, String> {
    let detail = get_design_system_for_tool_core(state, raw_design_system_id).await?;
    Ok(DesignSourceManifestResponse {
        design_system: detail.design_system,
        sources: detail.sources,
    })
}

async fn get_design_styleguide_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    query: DesignSchemaQuery,
) -> Result<DesignStyleguideToolResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let schema_version_id = query.schema_version_id.clone();
    let items = list_design_styleguide_items_core(
        state,
        ListDesignStyleguideItemsInput {
            design_system_id: design_system_id.as_str().to_string(),
            schema_version_id,
        },
    )
    .await?;

    let resolved_schema_version_id = items
        .first()
        .map(|item| item.schema_version_id.clone())
        .or(query.schema_version_id);

    Ok(DesignStyleguideToolResponse {
        design_system_id: design_system_id.as_str().to_string(),
        schema_version_id: resolved_schema_version_id,
        items,
    })
}

async fn update_design_styleguide_item_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    req: UpdateDesignStyleguideItemRequest,
) -> Result<DesignStyleguideItemResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let item_id = normalize_required_string(&req.item_id, "item_id")?;
    let approval_status = parse_approval_status(&req.approval_status)?;
    let item = load_styleguide_item(state, &design_system_id, &item_id).await?;

    if approval_status == DesignApprovalStatus::Approved {
        state
            .design_styleguide_repo
            .approve_item(&item.id)
            .await
            .map_err(|error| error.to_string())?;
    } else {
        let mut item = item;
        item.approval_status = approval_status;
        if approval_status == DesignApprovalStatus::NeedsWork {
            item.feedback_status = DesignFeedbackStatus::Open;
        }
        item.updated_at = Utc::now();
        state
            .design_styleguide_repo
            .update_item(&item)
            .await
            .map_err(|error| error.to_string())?;
    }

    state
        .design_styleguide_repo
        .get_item(&design_system_id, &item_id)
        .await
        .map_err(|error| error.to_string())?
        .map(DesignStyleguideItemResponse::from)
        .ok_or_else(|| format!("Design styleguide item not found: {item_id}"))
}

async fn record_design_styleguide_feedback_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    req: RecordDesignStyleguideFeedbackRequest,
) -> Result<CreateDesignStyleguideFeedbackResponse, String> {
    create_design_styleguide_feedback_core_with_options(
        state,
        CreateDesignStyleguideFeedbackInput {
            design_system_id: parse_design_system_id(raw_design_system_id)?
                .as_str()
                .to_string(),
            item_id: req.item_id,
            feedback: req.feedback,
            conversation_id: req.conversation_id,
        },
        CreateDesignStyleguideFeedbackOptions::record_only(),
    )
    .await
}

async fn list_design_artifacts_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    query: DesignSchemaQuery,
) -> Result<DesignArtifactsResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let Some(_system) = state
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

    let schema_version =
        resolve_schema_version(state, &design_system_id, query.schema_version_id).await?;
    let mut artifact_sources = BTreeMap::<String, String>::new();

    if let Some(schema_version) = schema_version.as_ref() {
        artifact_sources.insert(
            schema_version.schema_artifact_id.clone(),
            "current_schema".to_string(),
        );
        artifact_sources.insert(
            schema_version.manifest_artifact_id.clone(),
            "current_source_audit".to_string(),
        );
        artifact_sources.insert(
            schema_version.styleguide_artifact_id.clone(),
            "current_styleguide".to_string(),
        );
    }

    for run in state
        .design_run_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
    {
        for artifact_id in run.output_artifact_ids {
            artifact_sources
                .entry(artifact_id)
                .or_insert_with(|| format!("run_output:{}", run.id.as_str()));
        }
    }

    let mut artifacts = Vec::new();
    for (artifact_id, source) in artifact_sources {
        let Some(artifact) = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(artifact_id.clone()))
            .await
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        artifacts.push(DesignArtifactSummaryResponse {
            artifact_id,
            name: artifact.name,
            artifact_type: artifact.artifact_type.to_string(),
            content_kind: match artifact.content {
                ArtifactContent::Inline { .. } => "inline".to_string(),
                ArtifactContent::File { .. } => "file".to_string(),
            },
            version: artifact.metadata.version,
            created_by: artifact.metadata.created_by,
            created_at: artifact.metadata.created_at.to_rfc3339(),
            derived_from: artifact
                .derived_from
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            source,
        });
    }

    Ok(DesignArtifactsResponse {
        design_system_id: design_system_id.as_str().to_string(),
        schema_version_id: schema_version.map(|version| version.id.as_str().to_string()),
        artifacts,
    })
}

async fn resolve_schema_version(
    state: &crate::application::AppState,
    design_system_id: &DesignSystemId,
    raw_schema_version_id: Option<String>,
) -> Result<Option<DesignSchemaVersion>, String> {
    if let Some(raw_schema_version_id) = raw_schema_version_id {
        let schema_version_id = DesignSchemaVersionId::from_string(normalize_required_string(
            &raw_schema_version_id,
            "schema_version_id",
        )?);
        let schema_version = state
            .design_schema_repo
            .get_version(&schema_version_id)
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Design schema version not found: {raw_schema_version_id}"))?;
        if schema_version.design_system_id != *design_system_id {
            return Err("Design schema version does not belong to this design system".to_string());
        }
        return Ok(Some(schema_version));
    }

    state
        .design_schema_repo
        .get_current_for_design_system(design_system_id)
        .await
        .map_err(|error| error.to_string())
}

async fn load_styleguide_item(
    state: &crate::application::AppState,
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

fn parse_design_system_id(raw: &str) -> Result<DesignSystemId, String> {
    Ok(DesignSystemId::from_string(normalize_required_string(
        raw,
        "design_system_id",
    )?))
}

fn normalize_required_string(raw: &str, field: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    Ok(value.to_string())
}

fn parse_approval_status(raw: &str) -> Result<DesignApprovalStatus, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "needs_review" => Ok(DesignApprovalStatus::NeedsReview),
        "approved" => Ok(DesignApprovalStatus::Approved),
        "needs_work" => Ok(DesignApprovalStatus::NeedsWork),
        _ => Err("approval_status must be needs_review, approved, or needs_work".to_string()),
    }
}

fn design_storage_paths_for_http(
    state: &crate::application::AppState,
) -> Result<DesignStoragePaths, String> {
    let app_handle = state
        .app_handle
        .as_ref()
        .ok_or_else(|| "Design storage requires a Tauri app handle".to_string())?;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|error| {
        format!("Failed to resolve RalphX app data directory for design storage: {error}")
    })?;
    DesignStoragePaths::new(app_data_dir).map_err(|error| error.to_string())
}

fn map_design_tool_error(error: String) -> (StatusCode, String) {
    if error.to_ascii_lowercase().contains("not found") {
        (StatusCode::NOT_FOUND, error)
    } else {
        (StatusCode::BAD_REQUEST, error)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{
        Artifact, ArtifactType, ChatConversation, DesignConfidence, DesignRun, DesignRunKind,
        DesignRunStatus, DesignSchemaVersionStatus, DesignSourceKind, DesignSourceRole,
        DesignStorageRootRef, DesignStyleguideGroup, DesignStyleguideItemId, DesignSystem,
        DesignSystemSource, DesignSystemSourceId, Project, ProjectId,
    };

    fn design_system(project_id: ProjectId) -> DesignSystem {
        DesignSystem::new(
            project_id,
            "Atlas",
            DesignStorageRootRef::from_hash_component("design-test"),
        )
    }

    fn styleguide_item(
        design_system_id: DesignSystemId,
        schema_version_id: DesignSchemaVersionId,
    ) -> DesignStyleguideItem {
        DesignStyleguideItem {
            id: DesignStyleguideItemId::new(),
            design_system_id,
            schema_version_id,
            item_id: "components-button".to_string(),
            group: DesignStyleguideGroup::Components,
            label: "Button".to_string(),
            summary: "Primary action style".to_string(),
            preview_artifact_id: None,
            source_refs: Vec::new(),
            confidence: DesignConfidence::High,
            approval_status: DesignApprovalStatus::NeedsReview,
            feedback_status: DesignFeedbackStatus::None,
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn design_tool_updates_styleguide_item_status() {
        let state = AppState::new_test();
        let project = state
            .project_repo
            .create(Project::new("App".to_string(), "/tmp/app".to_string()))
            .await
            .expect("create project");
        let mut system = design_system(project.id.clone());
        let schema_version_id = DesignSchemaVersionId::new();
        system.current_schema_version_id = Some(schema_version_id.clone());
        let system = state
            .design_system_repo
            .create(system)
            .await
            .expect("create design system");
        let item = styleguide_item(system.id.clone(), schema_version_id.clone());
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&schema_version_id, vec![item])
            .await
            .expect("store item");

        let response = update_design_styleguide_item_for_tool_core(
            &state,
            system.id.as_str(),
            UpdateDesignStyleguideItemRequest {
                item_id: "components-button".to_string(),
                approval_status: "needs_work".to_string(),
            },
        )
        .await
        .expect("update item");

        assert_eq!(response.approval_status, "needs_work");
        assert_eq!(response.feedback_status, "open");
    }

    #[tokio::test]
    async fn design_tool_records_feedback_without_appending_chat_message() {
        let state = AppState::new_test();
        let project = state
            .project_repo
            .create(Project::new("App".to_string(), "/tmp/app".to_string()))
            .await
            .expect("create project");
        let mut system = design_system(project.id.clone());
        let schema_version_id = DesignSchemaVersionId::new();
        system.current_schema_version_id = Some(schema_version_id.clone());
        let system = state
            .design_system_repo
            .create(system)
            .await
            .expect("create design system");
        let item = styleguide_item(system.id.clone(), schema_version_id);
        let item_schema_version_id = item.schema_version_id.clone();
        state
            .design_styleguide_repo
            .replace_items_for_schema_version(&item_schema_version_id, vec![item])
            .await
            .expect("store item");
        let conversation = state
            .chat_conversation_repo
            .create(ChatConversation::new_design(system.id.clone()))
            .await
            .expect("create design conversation");

        let response = record_design_styleguide_feedback_for_tool_core(
            &state,
            system.id.as_str(),
            RecordDesignStyleguideFeedbackRequest {
                item_id: "components-button".to_string(),
                feedback: "The steward found source caveats to track.".to_string(),
                conversation_id: Some(conversation.id.as_str()),
            },
        )
        .await
        .expect("record feedback");

        assert_eq!(response.feedback.status, "open");
        assert!(response.feedback.message_id.is_none());
        assert!(response.message.is_none());
        assert_eq!(response.item.approval_status, "needs_work");
        assert_eq!(response.item.feedback_status, "open");
        let messages = state
            .chat_message_repo
            .get_by_conversation(&conversation.id)
            .await
            .expect("list chat messages");
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn design_tool_lists_current_schema_artifacts_without_file_paths() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();
        let mut system = design_system(project_id.clone());
        let schema_version_id = DesignSchemaVersionId::new();
        system.current_schema_version_id = Some(schema_version_id.clone());
        let system = state
            .design_system_repo
            .create(system)
            .await
            .expect("create design system");
        state
            .design_system_source_repo
            .replace_for_design_system(
                &system.id,
                vec![DesignSystemSource {
                    id: DesignSystemSourceId::new(),
                    design_system_id: system.id.clone(),
                    project_id,
                    role: DesignSourceRole::Primary,
                    selected_paths: vec!["src".to_string()],
                    source_kind: DesignSourceKind::ProjectCheckout,
                    git_commit: None,
                    source_hashes: BTreeMap::new(),
                    last_analyzed_at: Some(Utc::now()),
                }],
            )
            .await
            .expect("store source");
        let schema_artifact = state
            .artifact_repo
            .create(Artifact::new_file(
                "Schema",
                ArtifactType::Specification,
                "/tmp/app-data/schema.json",
                "ralphx-design",
            ))
            .await
            .expect("create schema artifact");
        let manifest_artifact = state
            .artifact_repo
            .create(Artifact::new_file(
                "Source audit",
                ArtifactType::Findings,
                "/tmp/app-data/source-audit.json",
                "ralphx-design",
            ))
            .await
            .expect("create manifest artifact");
        let styleguide_artifact = state
            .artifact_repo
            .create(Artifact::new_file(
                "Styleguide",
                ArtifactType::DesignDoc,
                "/tmp/app-data/styleguide.json",
                "ralphx-design",
            ))
            .await
            .expect("create styleguide artifact");
        state
            .design_schema_repo
            .create_version(DesignSchemaVersion {
                id: schema_version_id.clone(),
                design_system_id: system.id.clone(),
                version: "v1".to_string(),
                schema_artifact_id: schema_artifact.id.as_str().to_string(),
                manifest_artifact_id: manifest_artifact.id.as_str().to_string(),
                styleguide_artifact_id: styleguide_artifact.id.as_str().to_string(),
                status: DesignSchemaVersionStatus::Verified,
                created_by_run_id: None,
                created_at: Utc::now(),
            })
            .await
            .expect("create schema version");
        let extra_artifact = state
            .artifact_repo
            .create(Artifact::new_inline(
                "Design note",
                ArtifactType::DesignDoc,
                "note",
                "ralphx-design-steward",
            ))
            .await
            .expect("create extra artifact");
        let mut run = DesignRun::queued(system.id.clone(), DesignRunKind::Audit, "audit");
        run.status = DesignRunStatus::Completed;
        run.output_artifact_ids = vec![extra_artifact.id.as_str().to_string()];
        state.design_run_repo.create(run).await.expect("create run");

        let response = list_design_artifacts_for_tool_core(
            &state,
            system.id.as_str(),
            DesignSchemaQuery {
                schema_version_id: None,
            },
        )
        .await
        .expect("list artifacts");

        assert_eq!(
            response.schema_version_id,
            Some(schema_version_id.as_str().to_string())
        );
        assert_eq!(response.artifacts.len(), 4);
        assert!(response
            .artifacts
            .iter()
            .all(|artifact| artifact.content_kind == "file" || artifact.content_kind == "inline"));
        assert!(!serde_json::to_string(&response)
            .expect("serialize")
            .contains("/tmp/app-data/schema.json"));
    }
}
