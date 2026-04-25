use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use tracing::warn;

use crate::application::AppState;
use crate::commands::design_artifact_persistence::persist_design_generation_artifacts;
use crate::commands::design_feedback_commands::DesignStyleguideItemResponse;
use crate::commands::unified_chat_commands::AgentConversationResponse;
use crate::domain::entities::{
    ChatConversation, DesignApprovalStatus, DesignConfidence, DesignFeedbackStatus, DesignRun,
    DesignRunKind, DesignRunStatus, DesignSchemaVersion, DesignSchemaVersionId,
    DesignSchemaVersionStatus, DesignSourceKind, DesignSourceRef, DesignSourceRole,
    DesignStyleguideGroup, DesignStyleguideItem, DesignStyleguideItemId, DesignSystem,
    DesignSystemId, DesignSystemSource, DesignSystemSourceId, DesignSystemStatus, Project,
    ProjectId,
};
use crate::utils::design_source_manifest::build_design_source_manifest;
use crate::utils::design_storage_paths::DesignStoragePaths;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignSystemSourceInput {
    pub project_id: String,
    pub role: Option<String>,
    #[serde(default)]
    pub selected_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignSystemInput {
    pub primary_project_id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub selected_paths: Vec<String>,
    #[serde(default)]
    pub sources: Vec<CreateDesignSystemSourceInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDesignSystemStyleguideInput {
    pub design_system_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemResponse {
    pub id: String,
    pub primary_project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub current_schema_version_id: Option<String>,
    pub storage_root_ref: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemSourceResponse {
    pub id: String,
    pub design_system_id: String,
    pub project_id: String,
    pub role: String,
    pub selected_paths: Vec<String>,
    pub source_kind: String,
    pub git_commit: Option<String>,
    pub source_hashes: BTreeMap<String, String>,
    pub last_analyzed_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSystemDetailResponse {
    pub design_system: DesignSystemResponse,
    pub sources: Vec<DesignSystemSourceResponse>,
    pub conversation: Option<AgentConversationResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesignSystemResponse {
    pub design_system: DesignSystemResponse,
    pub sources: Vec<DesignSystemSourceResponse>,
    pub conversation: AgentConversationResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDesignSystemStyleguideResponse {
    pub design_system: DesignSystemResponse,
    pub schema_version_id: String,
    pub run_id: String,
    pub items: Vec<DesignStyleguideItemResponse>,
}

impl From<DesignSystem> for DesignSystemResponse {
    fn from(system: DesignSystem) -> Self {
        Self {
            id: system.id.as_str().to_string(),
            primary_project_id: system.primary_project_id.as_str().to_string(),
            name: system.name,
            description: system.description,
            status: enum_text(&system.status),
            current_schema_version_id: system
                .current_schema_version_id
                .map(|id| id.as_str().to_string()),
            storage_root_ref: system.storage_root_ref.as_str().to_string(),
            created_at: system.created_at.to_rfc3339(),
            updated_at: system.updated_at.to_rfc3339(),
            archived_at: system.archived_at.map(|value| value.to_rfc3339()),
        }
    }
}

impl From<DesignSystemSource> for DesignSystemSourceResponse {
    fn from(source: DesignSystemSource) -> Self {
        Self {
            id: source.id.as_str().to_string(),
            design_system_id: source.design_system_id.as_str().to_string(),
            project_id: source.project_id.as_str().to_string(),
            role: enum_text(&source.role),
            selected_paths: source.selected_paths,
            source_kind: enum_text(&source.source_kind),
            git_commit: source.git_commit,
            source_hashes: source.source_hashes,
            last_analyzed_at: source.last_analyzed_at.map(|value| value.to_rfc3339()),
        }
    }
}

#[tauri::command]
pub async fn create_design_system(
    input: CreateDesignSystemInput,
    state: State<'_, AppState>,
) -> Result<CreateDesignSystemResponse, String> {
    let primary_project_id = input.primary_project_id.clone();
    let selected_path_count = input.selected_paths.len();
    let additional_source_count = input.sources.len();
    let storage_paths = match design_storage_paths_from_state(&state) {
        Ok(paths) => paths,
        Err(error) => {
            warn!(
                %error,
                primary_project_id = %primary_project_id,
                selected_path_count,
                additional_source_count,
                "Failed to prepare design system storage"
            );
            return Err(error);
        }
    };

    match create_design_system_core(&state, &storage_paths, input).await {
        Ok(response) => Ok(response),
        Err(error) => {
            warn!(
                %error,
                primary_project_id = %primary_project_id,
                selected_path_count,
                additional_source_count,
                "Failed to create design system"
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn list_project_design_systems(
    project_id: String,
    include_archived: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<DesignSystemResponse>, String> {
    let project_id = parse_non_empty_project_id(&project_id, "project_id")?;
    state
        .design_system_repo
        .list_by_project(&project_id, include_archived.unwrap_or(false))
        .await
        .map(|systems| {
            systems
                .into_iter()
                .map(DesignSystemResponse::from)
                .collect()
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_design_system(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<DesignSystemDetailResponse>, String> {
    let design_system_id = parse_non_empty_design_system_id(&id)?;
    let Some(system) = state
        .design_system_repo
        .get_by_id(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };

    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    let conversation = state
        .chat_conversation_repo
        .get_active_for_context(
            crate::domain::entities::ChatContextType::Design,
            design_system_id.as_str(),
        )
        .await
        .map_err(|error| error.to_string())?;

    Ok(Some(DesignSystemDetailResponse {
        design_system: DesignSystemResponse::from(system),
        sources: sources
            .into_iter()
            .map(DesignSystemSourceResponse::from)
            .collect(),
        conversation: conversation.map(AgentConversationResponse::from),
    }))
}

#[tauri::command]
pub async fn archive_design_system(
    id: String,
    state: State<'_, AppState>,
) -> Result<DesignSystemResponse, String> {
    let design_system_id = parse_non_empty_design_system_id(&id)?;
    state
        .design_system_repo
        .archive(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .design_system_repo
        .get_by_id(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
        .map(DesignSystemResponse::from)
        .ok_or_else(|| format!("Design system not found: {}", design_system_id.as_str()))
}

#[tauri::command]
pub async fn generate_design_system_styleguide(
    input: GenerateDesignSystemStyleguideInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<GenerateDesignSystemStyleguideResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    let response = generate_design_system_styleguide_core(&state, &storage_paths, input).await?;
    let _ = app.emit("design:schema_published", &response);
    Ok(response)
}

#[doc(hidden)]
pub async fn create_design_system_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: CreateDesignSystemInput,
) -> Result<CreateDesignSystemResponse, String> {
    let primary_project_id =
        parse_non_empty_project_id(&input.primary_project_id, "primary_project_id")?;
    let primary_project = load_project(state, &primary_project_id).await?;

    let name = normalize_name(&input.name)?;
    let description = input
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());

    let design_system_id = DesignSystemId::new();
    let storage_root_ref = storage_paths.storage_ref_for_design_system(&design_system_id);
    let sources = build_sources(
        state,
        &design_system_id,
        &primary_project,
        input.selected_paths,
        input.sources,
    )
    .await?;
    storage_paths
        .ensure_design_system_root(&storage_root_ref)
        .map_err(|error| error.to_string())?;

    let now = Utc::now();
    let design_system = DesignSystem {
        id: design_system_id.clone(),
        primary_project_id,
        name: name.clone(),
        description,
        status: DesignSystemStatus::Draft,
        current_schema_version_id: None,
        storage_root_ref,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };

    let created_system = state
        .design_system_repo
        .create(design_system)
        .await
        .map_err(|error| error.to_string())?;
    state
        .design_system_source_repo
        .replace_for_design_system(&design_system_id, sources.clone())
        .await
        .map_err(|error| error.to_string())?;

    let mut conversation = ChatConversation::new_design(design_system_id.clone());
    conversation.set_title(format!("Design: {name}"));
    let conversation = state
        .chat_conversation_repo
        .create(conversation)
        .await
        .map_err(|error| error.to_string())?;

    Ok(CreateDesignSystemResponse {
        design_system: DesignSystemResponse::from(created_system),
        sources: sources
            .into_iter()
            .map(DesignSystemSourceResponse::from)
            .collect(),
        conversation: AgentConversationResponse::from(conversation),
    })
}

#[doc(hidden)]
pub async fn generate_design_system_styleguide_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: GenerateDesignSystemStyleguideInput,
) -> Result<GenerateDesignSystemStyleguideResponse, String> {
    let design_system_id = parse_non_empty_design_system_id(&input.design_system_id)?;
    let mut system = state
        .design_system_repo
        .get_by_id(&design_system_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Design system not found: {}", design_system_id.as_str()))?;
    if system.archived_at.is_some() || system.status == DesignSystemStatus::Archived {
        return Err("Archived design systems cannot publish new styleguides".to_string());
    }

    let sources = state
        .design_system_source_repo
        .list_by_design_system(&design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    if sources.is_empty() {
        return Err("Design system has no source manifests to publish".to_string());
    }

    let now = Utc::now();
    let mut run = DesignRun::queued(
        design_system_id.clone(),
        DesignRunKind::Create,
        "Generated initial source-grounded styleguide",
    );
    run.status = DesignRunStatus::Running;
    run.started_at = Some(now);
    let mut run = state
        .design_run_repo
        .create(run)
        .await
        .map_err(|error| error.to_string())?;

    let schema_version_id = DesignSchemaVersionId::new();
    let version = next_schema_version_label(state, &design_system_id).await?;
    let mut items = build_initial_styleguide_items(&system, &schema_version_id, &sources, now)?;
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

    run.status = DesignRunStatus::Completed;
    run.completed_at = Some(now);
    run.output_artifact_ids = artifacts.output_artifact_ids;
    state
        .design_run_repo
        .update(&run)
        .await
        .map_err(|error| error.to_string())?;

    Ok(GenerateDesignSystemStyleguideResponse {
        design_system: DesignSystemResponse::from(system),
        schema_version_id: schema_version.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        items: items
            .into_iter()
            .map(DesignStyleguideItemResponse::from)
            .collect(),
    })
}

fn design_storage_paths_from_state(state: &AppState) -> Result<DesignStoragePaths, String> {
    let app_handle = state
        .app_handle
        .as_ref()
        .ok_or_else(|| "Design storage requires a Tauri app handle".to_string())?;
    let app_data_dir = app_handle.path().app_data_dir().map_err(|error| {
        format!("Failed to resolve RalphX app data directory for design storage: {error}")
    })?;
    DesignStoragePaths::new(app_data_dir).map_err(|error| error.to_string())
}

async fn build_sources(
    state: &AppState,
    design_system_id: &DesignSystemId,
    primary_project: &Project,
    primary_selected_paths: Vec<String>,
    additional_sources: Vec<CreateDesignSystemSourceInput>,
) -> Result<Vec<DesignSystemSource>, String> {
    let primary_selected_paths = normalize_selected_paths(primary_selected_paths)?;
    let primary_source_hashes =
        source_hashes_for_project(primary_project, &primary_selected_paths)?;
    let mut sources = vec![DesignSystemSource {
        id: DesignSystemSourceId::new(),
        design_system_id: design_system_id.clone(),
        project_id: primary_project.id.clone(),
        role: DesignSourceRole::Primary,
        selected_paths: primary_selected_paths,
        source_kind: DesignSourceKind::ProjectCheckout,
        git_commit: None,
        source_hashes: primary_source_hashes,
        last_analyzed_at: None,
    }];
    let mut seen_projects = HashSet::from([primary_project.id.as_str().to_string()]);

    for source in additional_sources {
        let project_id = parse_non_empty_project_id(&source.project_id, "source.project_id")?;
        if !seen_projects.insert(project_id.as_str().to_string()) {
            return Err(format!(
                "Duplicate design source project: {}",
                project_id.as_str()
            ));
        }
        let project = load_project(state, &project_id).await?;
        let selected_paths = normalize_selected_paths(source.selected_paths)?;
        let source_hashes = source_hashes_for_project(&project, &selected_paths)?;

        sources.push(DesignSystemSource {
            id: DesignSystemSourceId::new(),
            design_system_id: design_system_id.clone(),
            project_id,
            role: parse_additional_source_role(source.role.as_deref())?,
            selected_paths,
            source_kind: DesignSourceKind::ProjectCheckout,
            git_commit: None,
            source_hashes,
            last_analyzed_at: None,
        });
    }

    Ok(sources)
}

async fn next_schema_version_label(
    state: &AppState,
    design_system_id: &DesignSystemId,
) -> Result<String, String> {
    let existing_versions = state
        .design_schema_repo
        .list_versions(design_system_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(format!("0.{}.0", existing_versions.len() + 1))
}

struct InitialStyleguideItem<'a> {
    item_id: &'a str,
    group: DesignStyleguideGroup,
    label: &'a str,
    summary: &'a str,
    keywords: &'a [&'a str],
}

fn build_initial_styleguide_items(
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    sources: &[DesignSystemSource],
    now: DateTime<Utc>,
) -> Result<Vec<DesignStyleguideItem>, String> {
    let source_refs = source_refs_from_manifests(sources);
    if source_refs.is_empty() {
        return Err("Design source manifests did not produce any file references".to_string());
    }

    let definitions = [
        InitialStyleguideItem {
            item_id: "ui_kit.workspace_surfaces",
            group: DesignStyleguideGroup::UiKit,
            label: "Workspace surfaces",
            summary: "Reviewable layout and pane patterns inferred from source-backed app surfaces.",
            keywords: &["app", "view", "layout", "page", "screen", "workspace"],
        },
        InitialStyleguideItem {
            item_id: "type.typography_scale",
            group: DesignStyleguideGroup::Type,
            label: "Typography scale",
            summary: "Text hierarchy, label density, and code-font usage inferred from style and UI files.",
            keywords: &["font", "typography", "text", "css", "style", "theme"],
        },
        InitialStyleguideItem {
            item_id: "colors.primary_palette",
            group: DesignStyleguideGroup::Colors,
            label: "Primary palette",
            summary: "Primary, hover, soft, border, and focus color roles grounded in style sources.",
            keywords: &["color", "css", "style", "theme", "tailwind", "token", "palette"],
        },
        InitialStyleguideItem {
            item_id: "spacing.radii_elevation",
            group: DesignStyleguideGroup::Spacing,
            label: "Spacing, radii, and elevation",
            summary: "Panel spacing, control radius, borders, focus rings, and elevation rules for review.",
            keywords: &["spacing", "radius", "shadow", "border", "ring", "layout", "css"],
        },
        InitialStyleguideItem {
            item_id: "components.core_controls",
            group: DesignStyleguideGroup::Components,
            label: "Core controls",
            summary: "Button, input, composer, and compact row patterns found in reusable components.",
            keywords: &["button", "input", "composer", "control", "component", "ui"],
        },
        InitialStyleguideItem {
            item_id: "brand.visual_identity",
            group: DesignStyleguideGroup::Brand,
            label: "Visual identity assets",
            summary: "Logo, icon, asset, and brand-adjacent source references available for curation.",
            keywords: &["logo", "icon", "asset", "brand", "public", "svg", "png"],
        },
    ];

    Ok(definitions
        .iter()
        .map(|definition| {
            let (item_refs, has_keyword_match) =
                matching_source_refs(&source_refs, definition.keywords);
            DesignStyleguideItem {
                id: DesignStyleguideItemId::new(),
                design_system_id: system.id.clone(),
                schema_version_id: schema_version_id.clone(),
                item_id: definition.item_id.to_string(),
                group: definition.group,
                label: definition.label.to_string(),
                summary: definition.summary.to_string(),
                preview_artifact_id: Some(format!(
                    "design-preview:{}:{}",
                    system.id.as_str(),
                    definition.item_id.replace('.', "-")
                )),
                source_refs: item_refs,
                confidence: confidence_for_match(has_keyword_match),
                approval_status: DesignApprovalStatus::NeedsReview,
                feedback_status: DesignFeedbackStatus::None,
                updated_at: now,
            }
        })
        .collect())
}

fn source_refs_from_manifests(sources: &[DesignSystemSource]) -> Vec<DesignSourceRef> {
    sources
        .iter()
        .flat_map(|source| {
            source.source_hashes.keys().map(|path| DesignSourceRef {
                project_id: source.project_id.clone(),
                path: path.clone(),
                line: None,
            })
        })
        .collect()
}

fn matching_source_refs(
    source_refs: &[DesignSourceRef],
    keywords: &[&str],
) -> (Vec<DesignSourceRef>, bool) {
    let matched: Vec<_> = source_refs
        .iter()
        .filter(|source_ref| path_matches_keywords(&source_ref.path, keywords))
        .take(3)
        .cloned()
        .collect();
    if !matched.is_empty() {
        return (matched, true);
    }

    (
        source_refs.iter().take(3).cloned().collect::<Vec<_>>(),
        false,
    )
}

fn path_matches_keywords(path: &str, keywords: &[&str]) -> bool {
    let path = path.to_ascii_lowercase();
    keywords.iter().any(|keyword| path.contains(keyword))
}

fn confidence_for_match(has_keyword_match: bool) -> DesignConfidence {
    if has_keyword_match {
        DesignConfidence::High
    } else {
        DesignConfidence::Low
    }
}

async fn load_project(state: &AppState, project_id: &ProjectId) -> Result<Project, String> {
    state
        .project_repo
        .get_by_id(project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))
}

fn source_hashes_for_project(
    project: &Project,
    selected_paths: &[String],
) -> Result<BTreeMap<String, String>, String> {
    build_design_source_manifest(
        project.id.clone(),
        Path::new(&project.working_directory),
        selected_paths,
    )
    .map(|manifest| manifest.source_hashes())
    .map_err(|error| error.to_string())
}

fn parse_non_empty_project_id(value: &str, field_name: &str) -> Result<ProjectId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name} is required"));
    }
    Ok(ProjectId::from_string(value.to_string()))
}

fn parse_non_empty_design_system_id(value: &str) -> Result<DesignSystemId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("design system id is required".to_string());
    }
    Ok(DesignSystemId::from_string(value.to_string()))
}

fn normalize_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Design system name is required".to_string());
    }
    Ok(name.to_string())
}

fn parse_additional_source_role(role: Option<&str>) -> Result<DesignSourceRole, String> {
    match role.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("secondary") => Ok(DesignSourceRole::Secondary),
        Some("reference") => Ok(DesignSourceRole::Reference),
        Some("primary") => Err("Additional design sources cannot use primary role".to_string()),
        Some(other) => Err(format!("Invalid design source role: {other}")),
    }
}

fn normalize_selected_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();

    for raw_path in paths {
        let path = normalize_selected_path(&raw_path)?;
        if let Some(path) = path {
            if seen.insert(path.clone()) {
                normalized.push(path);
            }
        }
    }

    Ok(normalized)
}

fn normalize_selected_path(raw_path: &str) -> Result<Option<String>, String> {
    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        return Ok(None);
    }

    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Err("Design source paths must be relative to the source project".to_string());
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
                );
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(safe.to_string_lossy().to_string()))
}

fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        ArtifactContent, ArtifactId, ArtifactType, ChatContextType, DesignRunStatus,
        DesignSchemaVersionStatus, Project,
    };
    use serde_json::Value as JsonValue;

    fn write_project_file(root: &Path, relative_path: &str, content: &str) {
        let path = root.join(
            normalize_selected_path(relative_path)
                .expect("valid relative test path")
                .expect("non-empty relative test path"),
        );
        let parent = path.parent().expect("test path parent");
        std::fs::create_dir_all(parent).expect("create parent");
        std::fs::write(path, content).expect("write file");
    }

    async fn artifact_file_path(
        state: &AppState,
        artifact_id: &str,
        expected_type: ArtifactType,
    ) -> PathBuf {
        let artifact = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(artifact_id.to_string()))
            .await
            .unwrap()
            .expect("artifact");
        assert_eq!(artifact.artifact_type, expected_type);
        match artifact.content {
            ArtifactContent::File { path } => PathBuf::from(path),
            ArtifactContent::Inline { .. } => panic!("expected file artifact"),
        }
    }

    #[tokio::test]
    async fn create_design_system_core_creates_draft_sources_and_conversation() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let project_root = tempfile::tempdir().expect("project tempdir");
        write_project_file(project_root.path(), "frontend/src/App.tsx", "app\n");
        let project = Project::new(
            "RalphX".to_string(),
            project_root.path().to_string_lossy().to_string(),
        );
        state.project_repo.create(project.clone()).await.unwrap();

        let response = create_design_system_core(
            &state,
            &storage_paths,
            CreateDesignSystemInput {
                primary_project_id: project.id.as_str().to_string(),
                name: " Product UI ".to_string(),
                description: Some("  Core product design system ".to_string()),
                selected_paths: vec!["frontend/src".to_string(), "./frontend/src".to_string()],
                sources: Vec::new(),
            },
        )
        .await
        .expect("create design system");

        assert_eq!(response.design_system.name, "Product UI");
        assert_eq!(
            response.design_system.description.as_deref(),
            Some("Core product design system")
        );
        assert_eq!(response.design_system.status, "draft");
        assert_eq!(response.sources.len(), 1);
        assert_eq!(response.sources[0].role, "primary");
        assert_eq!(response.sources[0].selected_paths, vec!["frontend/src"]);
        assert_eq!(response.sources[0].source_hashes.len(), 1);
        assert!(response.sources[0]
            .source_hashes
            .contains_key("frontend/src/App.tsx"));
        assert_eq!(response.conversation.context_type, "design");
        assert_eq!(response.conversation.context_id, response.design_system.id);

        let design_system_id = DesignSystemId::from_string(response.design_system.id.clone());
        let stored_system = state
            .design_system_repo
            .get_by_id(&design_system_id)
            .await
            .unwrap()
            .expect("stored system");
        assert_eq!(
            stored_system.storage_root_ref.as_str(),
            response.design_system.storage_root_ref
        );
        let conversations = state
            .chat_conversation_repo
            .get_by_context(ChatContextType::Design, design_system_id.as_str())
            .await
            .unwrap();
        assert_eq!(conversations.len(), 1);
        assert!(state
            .design_run_repo
            .list_by_design_system(&design_system_id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn generate_design_system_styleguide_core_publishes_schema_items_and_run() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let project_root = tempfile::tempdir().expect("project tempdir");
        write_project_file(
            project_root.path(),
            "frontend/src/components/ui/button.tsx",
            "export function Button() {}\n",
        );
        write_project_file(
            project_root.path(),
            "frontend/src/styles/theme.css",
            ":root { --accent-primary: #ff6b35; }\n",
        );
        write_project_file(
            project_root.path(),
            "frontend/src/App.tsx",
            "export function App() {}\n",
        );
        write_project_file(project_root.path(), "public/logo.svg", "<svg />\n");
        let project = Project::new(
            "RalphX".to_string(),
            project_root.path().to_string_lossy().to_string(),
        );
        state.project_repo.create(project.clone()).await.unwrap();

        let draft = create_design_system_core(
            &state,
            &storage_paths,
            CreateDesignSystemInput {
                primary_project_id: project.id.as_str().to_string(),
                name: "Product UI".to_string(),
                description: None,
                selected_paths: vec!["frontend/src".to_string(), "public".to_string()],
                sources: Vec::new(),
            },
        )
        .await
        .expect("create design system");
        let design_system_id = DesignSystemId::from_string(draft.design_system.id.clone());

        let response = generate_design_system_styleguide_core(
            &state,
            &storage_paths,
            GenerateDesignSystemStyleguideInput {
                design_system_id: design_system_id.as_str().to_string(),
            },
        )
        .await
        .expect("generate styleguide");

        assert_eq!(response.design_system.status, "ready");
        assert_eq!(
            response.design_system.current_schema_version_id.as_deref(),
            Some(response.schema_version_id.as_str())
        );
        assert_eq!(response.items.len(), 6);
        assert!(response
            .items
            .iter()
            .any(|item| item.item_id == "colors.primary_palette"));
        assert!(response
            .items
            .iter()
            .all(|item| !item.source_refs.is_empty()));
        for source_ref in response
            .items
            .iter()
            .flat_map(|item| item.source_refs.iter())
        {
            assert_eq!(source_ref.project_id, project.id);
            assert!(!Path::new(&source_ref.path).is_absolute());
        }

        let current_schema = state
            .design_schema_repo
            .get_current_for_design_system(&design_system_id)
            .await
            .unwrap()
            .expect("current schema");
        assert_eq!(current_schema.id.as_str(), response.schema_version_id);
        assert_eq!(current_schema.version, "0.1.0");
        assert_eq!(current_schema.status, DesignSchemaVersionStatus::Verified);
        assert_eq!(
            current_schema
                .created_by_run_id
                .as_ref()
                .map(|id| id.as_str()),
            Some(response.run_id.as_str())
        );

        let storage_root = temp.path().canonicalize().unwrap();
        let schema_path = artifact_file_path(
            &state,
            &current_schema.schema_artifact_id,
            ArtifactType::Specification,
        )
        .await;
        assert!(schema_path.starts_with(&storage_root));
        assert!(!schema_path.starts_with(project_root.path()));
        let schema_json: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(&schema_path).expect("schema json"))
                .expect("parse schema json");
        assert_eq!(schema_json["schema_version"], "1.0");
        assert_eq!(
            schema_json["design_system"]["id"].as_str(),
            Some(design_system_id.as_str())
        );
        assert!(schema_json["sources"][0]["source_hashes"]
            .as_object()
            .unwrap()
            .contains_key("frontend/src/components/ui/button.tsx"));

        let manifest_path = artifact_file_path(
            &state,
            &current_schema.manifest_artifact_id,
            ArtifactType::Findings,
        )
        .await;
        assert!(manifest_path.starts_with(&storage_root));
        let manifest_json: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).expect("manifest json"))
                .expect("parse manifest json");
        assert_eq!(manifest_json["total_file_count"].as_u64(), Some(4));

        let styleguide_path = artifact_file_path(
            &state,
            &current_schema.styleguide_artifact_id,
            ArtifactType::DesignDoc,
        )
        .await;
        assert!(styleguide_path.starts_with(&storage_root));
        let styleguide_json: JsonValue = serde_json::from_str(
            &std::fs::read_to_string(&styleguide_path).expect("styleguide json"),
        )
        .expect("parse styleguide json");
        assert_eq!(styleguide_json["groups"].as_array().unwrap().len(), 6);

        for item in &response.items {
            let preview_artifact_id = item
                .preview_artifact_id
                .as_deref()
                .expect("preview artifact id");
            let preview_path =
                artifact_file_path(&state, preview_artifact_id, ArtifactType::DesignDoc).await;
            assert!(preview_path.starts_with(&storage_root));
            let preview_json: JsonValue = serde_json::from_str(
                &std::fs::read_to_string(&preview_path).expect("preview json"),
            )
            .expect("parse preview json");
            assert_eq!(
                preview_json["item_id"].as_str(),
                Some(item.item_id.as_str())
            );
        }

        let runs = state
            .design_run_repo
            .list_by_design_system(&design_system_id)
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, DesignRunStatus::Completed);
        assert_eq!(runs[0].output_artifact_ids.len(), 3 + response.items.len());
        assert!(runs[0]
            .output_artifact_ids
            .contains(&current_schema.schema_artifact_id));
        assert!(runs[0]
            .output_artifact_ids
            .contains(&current_schema.manifest_artifact_id));
        assert!(runs[0]
            .output_artifact_ids
            .contains(&current_schema.styleguide_artifact_id));
        assert!(!project_root.path().join("design-systems").exists());
    }

    #[tokio::test]
    async fn create_design_system_core_rejects_unsafe_source_paths() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let project_root = tempfile::tempdir().expect("project tempdir");
        let project = Project::new(
            "RalphX".to_string(),
            project_root.path().to_string_lossy().to_string(),
        );
        state.project_repo.create(project.clone()).await.unwrap();

        let error = create_design_system_core(
            &state,
            &storage_paths,
            CreateDesignSystemInput {
                primary_project_id: project.id.as_str().to_string(),
                name: "Product UI".to_string(),
                description: None,
                selected_paths: vec!["../outside".to_string()],
                sources: Vec::new(),
            },
        )
        .await
        .expect_err("unsafe path should fail");

        assert!(error.contains("Design source paths cannot contain"));
        assert!(state
            .design_system_repo
            .list_by_project(&project.id, true)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn create_design_system_core_rejects_missing_source_project() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");

        let error = create_design_system_core(
            &state,
            &storage_paths,
            CreateDesignSystemInput {
                primary_project_id: ProjectId::new().as_str().to_string(),
                name: "Product UI".to_string(),
                description: None,
                selected_paths: Vec::new(),
                sources: Vec::new(),
            },
        )
        .await
        .expect_err("missing project should fail");

        assert!(error.contains("Project not found"));
    }
}
