use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path as FsPath, PathBuf};

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
use crate::commands::design_artifact_persistence::persist_design_generation_artifacts;
use crate::commands::design_commands::{
    next_schema_version_label, DesignRunEventPayload, DesignSystemDetailResponse,
    DesignSystemResponse, DesignSystemSourceResponse,
};
use crate::commands::design_feedback_commands::{
    create_design_styleguide_feedback_core_with_options, list_design_styleguide_items_core,
    CreateDesignStyleguideFeedbackInput, CreateDesignStyleguideFeedbackOptions,
    CreateDesignStyleguideFeedbackResponse, DesignStyleguideItemResponse,
    ListDesignStyleguideItemsInput,
};
use crate::domain::entities::{
    ArtifactContent, ArtifactId, ChatContextType, DesignApprovalStatus, DesignConfidence,
    DesignFeedbackStatus, DesignRun, DesignRunId, DesignRunKind, DesignRunStatus,
    DesignSchemaVersion, DesignSchemaVersionId, DesignSchemaVersionStatus, DesignSourceRef,
    DesignStyleguideGroup, DesignStyleguideItem, DesignStyleguideItemId, DesignSystemId,
    DesignSystemStatus, Project, ProjectId,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDesignSourceFilesQuery {
    #[serde(alias = "project_id")]
    pub project_id: Option<String>,
    #[serde(alias = "max_files")]
    pub max_files: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDesignSourceFileRequest {
    #[serde(alias = "project_id")]
    pub project_id: Option<String>,
    pub path: String,
    #[serde(alias = "start_line")]
    pub start_line: Option<usize>,
    #[serde(alias = "end_line")]
    pub end_line: Option<usize>,
    #[serde(alias = "max_bytes")]
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDesignSourceFilesRequest {
    #[serde(alias = "project_id")]
    pub project_id: Option<String>,
    pub pattern: String,
    #[serde(alias = "case_sensitive")]
    pub case_sensitive: Option<bool>,
    #[serde(alias = "max_results")]
    pub max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDesignSourceRefInput {
    #[serde(alias = "project_id")]
    pub project_id: String,
    pub path: String,
    pub line: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDesignStyleguideItemInput {
    #[serde(alias = "item_id")]
    pub item_id: String,
    pub group: String,
    pub label: String,
    pub summary: String,
    #[serde(default, alias = "source_refs")]
    pub source_refs: Vec<PublishDesignSourceRefInput>,
    pub confidence: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDesignSchemaVersionRequest {
    pub version: Option<String>,
    #[serde(default)]
    pub items: Vec<PublishDesignStyleguideItemInput>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSourceFileSummary {
    pub project_id: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSourceFilesResponse {
    pub design_system_id: String,
    pub files: Vec<DesignSourceFileSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadDesignSourceFileResponse {
    pub design_system_id: String,
    pub project_id: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub truncated: bool,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSourceSearchMatch {
    pub project_id: String,
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDesignSourceFilesResponse {
    pub design_system_id: String,
    pub matches: Vec<DesignSourceSearchMatch>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDesignSchemaVersionResponse {
    pub design_system: DesignSystemResponse,
    pub schema_version_id: String,
    pub run_id: String,
    pub items: Vec<DesignStyleguideItemResponse>,
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

pub async fn list_design_source_files_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Query(query): Query<ListDesignSourceFilesQuery>,
) -> Result<Json<DesignSourceFilesResponse>, (StatusCode, String)> {
    list_design_source_files_for_tool_core(&state.app_state, &design_system_id, query)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn read_design_source_file_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<ReadDesignSourceFileRequest>,
) -> Result<Json<ReadDesignSourceFileResponse>, (StatusCode, String)> {
    read_design_source_file_for_tool_core(&state.app_state, &design_system_id, req)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn search_design_source_files_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<SearchDesignSourceFilesRequest>,
) -> Result<Json<SearchDesignSourceFilesResponse>, (StatusCode, String)> {
    search_design_source_files_for_tool_core(&state.app_state, &design_system_id, req)
        .await
        .map(Json)
        .map_err(map_design_tool_error)
}

pub async fn publish_design_schema_version_for_tool(
    State(state): State<HttpServerState>,
    Path(design_system_id): Path<String>,
    Json(req): Json<PublishDesignSchemaVersionRequest>,
) -> Result<Json<PublishDesignSchemaVersionResponse>, (StatusCode, String)> {
    let storage_paths =
        design_storage_paths_for_http(&state.app_state).map_err(map_design_tool_error)?;
    let response = publish_design_schema_version_for_tool_core(
        &state.app_state,
        &storage_paths,
        &design_system_id,
        req,
    )
    .await
    .map_err(map_design_tool_error)?;

    if let Some(handle) = &state.app_state.app_handle {
        let _ = handle.emit("design:schema_published", &response);
        let _ = handle.emit("design:system_updated", &response.design_system);
        if let Ok(Some(run)) = state
            .app_state
            .design_run_repo
            .get_by_id(&DesignRunId::from_string(response.run_id.clone()))
            .await
        {
            let _ = handle.emit("design:run_completed", DesignRunEventPayload::from(&run));
        }
    }

    Ok(Json(response))
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

async fn list_design_source_files_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    query: ListDesignSourceFilesQuery,
) -> Result<DesignSourceFilesResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let project_filter = query
        .project_id
        .as_deref()
        .map(|value| parse_project_id(value, "project_id"))
        .transpose()?;
    let max_files = query.max_files.unwrap_or(500).clamp(1, 2_000);
    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut files = Vec::new();
    for source in sources {
        if project_filter
            .as_ref()
            .is_some_and(|id| *id != source.project_id)
        {
            continue;
        }
        for (path, sha256) in source.source_hashes {
            files.push(DesignSourceFileSummary {
                project_id: source.project_id.as_str().to_string(),
                path,
                sha256,
            });
            if files.len() >= max_files {
                return Ok(DesignSourceFilesResponse {
                    design_system_id: design_system_id.as_str().to_string(),
                    files,
                });
            }
        }
    }
    files.sort_by(|left, right| {
        left.project_id
            .cmp(&right.project_id)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(DesignSourceFilesResponse {
        design_system_id: design_system_id.as_str().to_string(),
        files,
    })
}

async fn read_design_source_file_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    req: ReadDesignSourceFileRequest,
) -> Result<ReadDesignSourceFileResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let source_file = resolve_manifest_source_file(
        state,
        &design_system_id,
        req.project_id.as_deref(),
        &req.path,
    )
    .await?;
    let max_bytes = req.max_bytes.unwrap_or(64 * 1024).clamp(1, 256 * 1024);
    let metadata = std::fs::metadata(&source_file.path)
        .map_err(|error| format!("Failed to read selected design source metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Selected design source path is not a file".to_string());
    }
    let truncated = metadata.len() > max_bytes;
    let bytes = std::fs::read(&source_file.path)
        .map_err(|error| format!("Failed to read selected design source file: {error}"))?;
    let bytes = &bytes[..bytes.len().min(max_bytes as usize)];
    let content = String::from_utf8_lossy(bytes);
    let lines = content.lines().collect::<Vec<_>>();
    let start_line = req.start_line.unwrap_or(1).max(1);
    let end_line = req.end_line.unwrap_or(lines.len()).max(start_line);
    let selected = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index + 1;
            (line_number >= start_line && line_number <= end_line).then_some(*line)
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ReadDesignSourceFileResponse {
        design_system_id: design_system_id.as_str().to_string(),
        project_id: source_file.project_id.as_str().to_string(),
        path: source_file.relative_path,
        start_line,
        end_line: end_line.min(lines.len().max(start_line)),
        truncated,
        content: selected,
    })
}

async fn search_design_source_files_for_tool_core(
    state: &crate::application::AppState,
    raw_design_system_id: &str,
    req: SearchDesignSourceFilesRequest,
) -> Result<SearchDesignSourceFilesResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let pattern = normalize_required_string(&req.pattern, "pattern")?;
    let project_filter = req
        .project_id
        .as_deref()
        .map(|value| parse_project_id(value, "project_id"))
        .transpose()?;
    let case_sensitive = req.case_sensitive.unwrap_or(false);
    let max_results = req.max_results.unwrap_or(100).clamp(1, 1_000);
    let needle = if case_sensitive {
        pattern.clone()
    } else {
        pattern.to_ascii_lowercase()
    };
    let mut matches = Vec::new();
    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    for source in sources {
        if project_filter
            .as_ref()
            .is_some_and(|id| *id != source.project_id)
        {
            continue;
        }
        for relative_path in source.source_hashes.keys() {
            let source_file = resolve_manifest_source_file(
                state,
                &design_system_id,
                Some(source.project_id.as_str()),
                relative_path,
            )
            .await?;
            let metadata = std::fs::metadata(&source_file.path).map_err(|error| {
                format!("Failed to read selected design source metadata: {error}")
            })?;
            if !metadata.is_file() || metadata.len() > 512 * 1024 {
                continue;
            }
            let content = std::fs::read_to_string(&source_file.path).unwrap_or_default();
            for (index, line) in content.lines().enumerate() {
                let haystack = if case_sensitive {
                    line.to_string()
                } else {
                    line.to_ascii_lowercase()
                };
                if haystack.contains(&needle) {
                    matches.push(DesignSourceSearchMatch {
                        project_id: source_file.project_id.as_str().to_string(),
                        path: source_file.relative_path.clone(),
                        line: index + 1,
                        text: line.trim().to_string(),
                    });
                    if matches.len() >= max_results {
                        return Ok(SearchDesignSourceFilesResponse {
                            design_system_id: design_system_id.as_str().to_string(),
                            matches,
                            truncated: true,
                        });
                    }
                }
            }
        }
    }

    Ok(SearchDesignSourceFilesResponse {
        design_system_id: design_system_id.as_str().to_string(),
        matches,
        truncated: false,
    })
}

pub(crate) async fn publish_design_schema_version_for_tool_core(
    state: &crate::application::AppState,
    storage_paths: &DesignStoragePaths,
    raw_design_system_id: &str,
    req: PublishDesignSchemaVersionRequest,
) -> Result<PublishDesignSchemaVersionResponse, String> {
    let design_system_id = parse_design_system_id(raw_design_system_id)?;
    let mut system = state
        .design_system_repo
        .get_by_id(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Design system not found: {}", design_system_id.as_str()))?;
    if system.status == DesignSystemStatus::Archived || system.archived_at.is_some() {
        return Err("Archived design systems cannot publish new schema versions".to_string());
    }
    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    if sources.is_empty() {
        return Err("Design system has no source manifest to publish against".to_string());
    }
    let schema_version_id = DesignSchemaVersionId::new();
    let version = match req
        .version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(version) => version.to_string(),
        None => next_schema_version_label(state, &design_system_id).await?,
    };
    let now = Utc::now();
    let mut items = styleguide_items_from_publish_request(
        &design_system_id,
        &schema_version_id,
        &sources,
        req.items,
        now,
    )?;
    let artifacts = persist_design_generation_artifacts(
        state,
        storage_paths,
        &system,
        &schema_version_id,
        &version,
        &sources,
        &mut items,
        now,
    )
    .await?;
    let mut run = latest_active_design_run(state, &design_system_id)
        .await?
        .unwrap_or_else(|| {
            let mut run = DesignRun::queued(
                design_system_id.clone(),
                DesignRunKind::Update,
                "Published design schema version from steward output",
            );
            run.started_at = Some(now);
            run
        });
    run.status = DesignRunStatus::Completed;
    run.completed_at = Some(now);
    run.output_artifact_ids = artifacts.output_artifact_ids.clone();
    let run = if state
        .design_run_repo
        .get_by_id(&run.id)
        .await
        .map_err(|error| error.to_string())?
        .is_some()
    {
        state
            .design_run_repo
            .update(&run)
            .await
            .map_err(|error| error.to_string())?;
        run
    } else {
        state
            .design_run_repo
            .create(run)
            .await
            .map_err(|error| error.to_string())?
    };
    let schema_version = DesignSchemaVersion {
        id: schema_version_id.clone(),
        design_system_id: design_system_id.clone(),
        version,
        schema_artifact_id: artifacts.schema_artifact_id,
        manifest_artifact_id: artifacts.manifest_artifact_id,
        styleguide_artifact_id: artifacts.styleguide_artifact_id,
        status: DesignSchemaVersionStatus::Verified,
        created_by_run_id: Some(run.id.clone()),
        created_at: now,
    };
    let schema_version = state
        .design_schema_repo
        .create_version(schema_version)
        .await
        .map_err(|error| error.to_string())?;
    state
        .design_styleguide_repo
        .replace_items_for_schema_version(&schema_version.id, items.clone())
        .await
        .map_err(|error| error.to_string())?;

    system.status = DesignSystemStatus::Ready;
    system.current_schema_version_id = Some(schema_version.id.clone());
    system.updated_at = now;
    state
        .design_system_repo
        .update(&system)
        .await
        .map_err(|error| error.to_string())?;

    Ok(PublishDesignSchemaVersionResponse {
        design_system: DesignSystemResponse::from(system),
        schema_version_id: schema_version.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        items: items
            .into_iter()
            .map(DesignStyleguideItemResponse::from)
            .collect(),
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

struct ResolvedDesignSourceFile {
    project_id: ProjectId,
    relative_path: String,
    path: PathBuf,
}

async fn resolve_manifest_source_file(
    state: &crate::application::AppState,
    design_system_id: &DesignSystemId,
    raw_project_id: Option<&str>,
    raw_path: &str,
) -> Result<ResolvedDesignSourceFile, String> {
    let relative_path = normalize_source_ref_path(raw_path)?;
    let project_filter = raw_project_id
        .map(|value| parse_project_id(value, "project_id"))
        .transpose()?;
    let sources = state
        .design_system_source_repo
        .list_by_design_system(design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    let matches = sources
        .into_iter()
        .filter(|source| {
            project_filter
                .as_ref()
                .is_none_or(|id| *id == source.project_id)
        })
        .filter(|source| source.source_hashes.contains_key(&relative_path))
        .collect::<Vec<_>>();
    let source = match matches.as_slice() {
        [source] => source,
        [] => return Err("Selected path is not in the design source manifest".to_string()),
        _ => {
            return Err(
                "Selected path is ambiguous across design source projects; provide project_id"
                    .to_string(),
            )
        }
    };
    let project = load_project_for_design_source(state, &source.project_id).await?;
    let root = canonical_project_root(&project)?;
    let safe_relative = safe_relative_path(&relative_path)?;
    let path = root
        .join(safe_relative)
        .canonicalize()
        .map_err(|error| format!("Failed to canonicalize selected design source file: {error}"))?;
    ensure_under(&path, &root, "selected design source file")?;
    Ok(ResolvedDesignSourceFile {
        project_id: source.project_id.clone(),
        relative_path,
        path,
    })
}

fn styleguide_items_from_publish_request(
    design_system_id: &DesignSystemId,
    schema_version_id: &DesignSchemaVersionId,
    sources: &[crate::domain::entities::DesignSystemSource],
    item_inputs: Vec<PublishDesignStyleguideItemInput>,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<DesignStyleguideItem>, String> {
    if item_inputs.is_empty() {
        return Err(
            "publish_design_schema_version requires at least one styleguide item".to_string(),
        );
    }
    let manifest_index = manifest_source_index(sources);
    item_inputs
        .into_iter()
        .map(|input| {
            let item_id = normalize_required_string(&input.item_id, "item_id")?;
            let group = parse_styleguide_group(&input.group)?;
            let confidence = input
                .confidence
                .as_deref()
                .map(parse_confidence)
                .transpose()?
                .unwrap_or(DesignConfidence::Medium);
            let source_refs = input
                .source_refs
                .into_iter()
                .map(|source_ref| validate_publish_source_ref(source_ref, &manifest_index))
                .collect::<Result<Vec<_>, _>>()?;
            if source_refs.is_empty() && confidence != DesignConfidence::Low {
                return Err(format!(
                    "Styleguide item {item_id} needs source_refs or low confidence"
                ));
            }
            Ok(DesignStyleguideItem {
                id: DesignStyleguideItemId::new(),
                design_system_id: design_system_id.clone(),
                schema_version_id: schema_version_id.clone(),
                item_id,
                group,
                label: normalize_required_string(&input.label, "label")?,
                summary: input.summary.trim().to_string(),
                preview_artifact_id: None,
                source_refs,
                confidence,
                approval_status: DesignApprovalStatus::NeedsReview,
                feedback_status: DesignFeedbackStatus::None,
                updated_at: now,
            })
        })
        .collect()
}

fn manifest_source_index(
    sources: &[crate::domain::entities::DesignSystemSource],
) -> HashSet<(String, String)> {
    sources
        .iter()
        .flat_map(|source| {
            source.source_hashes.keys().map(|path| {
                (
                    source.project_id.as_str().to_string(),
                    path.as_str().to_string(),
                )
            })
        })
        .collect()
}

fn validate_publish_source_ref(
    source_ref: PublishDesignSourceRefInput,
    manifest_index: &HashSet<(String, String)>,
) -> Result<DesignSourceRef, String> {
    let project_id = parse_project_id(&source_ref.project_id, "source_ref.project_id")?;
    let path = normalize_source_ref_path(&source_ref.path)?;
    if !manifest_index.contains(&(project_id.as_str().to_string(), path.clone())) {
        return Err(format!(
            "Source ref {}:{} is not in the selected design source manifest",
            project_id.as_str(),
            path
        ));
    }
    Ok(DesignSourceRef {
        project_id,
        path,
        line: source_ref.line,
    })
}

async fn latest_active_design_run(
    state: &crate::application::AppState,
    design_system_id: &DesignSystemId,
) -> Result<Option<DesignRun>, String> {
    let runs = state
        .design_run_repo
        .list_by_design_system(design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(runs.into_iter().find(|run| {
        matches!(
            run.status,
            DesignRunStatus::Queued | DesignRunStatus::Running
        ) && matches!(
            run.kind,
            DesignRunKind::Create | DesignRunKind::Update | DesignRunKind::ItemFeedback
        )
    }))
}

async fn load_project_for_design_source(
    state: &crate::application::AppState,
    project_id: &ProjectId,
) -> Result<Project, String> {
    state
        .project_repo
        .get_by_id(project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))
}

fn canonical_project_root(project: &Project) -> Result<PathBuf, String> {
    let root = FsPath::new(&project.working_directory);
    if !root.is_absolute() {
        return Err("Design source project root must be absolute".to_string());
    }
    let root = root
        .canonicalize()
        .map_err(|error| format!("Failed to canonicalize design source project root: {error}"))?;
    if !root.is_dir() {
        return Err("Design source project root must be a directory".to_string());
    }
    Ok(root)
}

fn safe_relative_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = FsPath::new(raw_path);
    if path.is_absolute() {
        return Err("Design source paths must be relative".to_string());
    }
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(
                    "Design source paths cannot contain parent, root, or prefix components"
                        .to_string(),
                )
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err("Design source path is required".to_string());
    }
    Ok(safe)
}

fn normalize_source_ref_path(raw_path: &str) -> Result<String, String> {
    let safe = safe_relative_path(raw_path)?;
    Ok(safe
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/"))
}

fn ensure_under(path: &FsPath, root: &FsPath, label: &str) -> Result<(), String> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(format!("{label} escaped design source project root"))
    }
}

fn parse_design_system_id(raw: &str) -> Result<DesignSystemId, String> {
    Ok(DesignSystemId::from_string(normalize_required_string(
        raw,
        "design_system_id",
    )?))
}

fn parse_project_id(raw: &str, field: &str) -> Result<ProjectId, String> {
    Ok(ProjectId::from_string(normalize_required_string(
        raw, field,
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

fn parse_styleguide_group(raw: &str) -> Result<DesignStyleguideGroup, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ui_kit" | "ui-kit" | "ui kit" => Ok(DesignStyleguideGroup::UiKit),
        "type" | "typography" => Ok(DesignStyleguideGroup::Type),
        "colors" | "color" => Ok(DesignStyleguideGroup::Colors),
        "spacing" => Ok(DesignStyleguideGroup::Spacing),
        "components" | "component" => Ok(DesignStyleguideGroup::Components),
        "brand" => Ok(DesignStyleguideGroup::Brand),
        _ => Err("group must be ui_kit, type, colors, spacing, components, or brand".to_string()),
    }
}

fn parse_confidence(raw: &str) -> Result<DesignConfidence, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" => Ok(DesignConfidence::High),
        "medium" => Ok(DesignConfidence::Medium),
        "low" => Ok(DesignConfidence::Low),
        _ => Err("confidence must be high, medium, or low".to_string()),
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
