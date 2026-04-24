use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::application::AppState;
use crate::commands::unified_chat_commands::AgentConversationResponse;
use crate::domain::entities::{
    ChatConversation, DesignSourceKind, DesignSourceRole, DesignSystem, DesignSystemId,
    DesignSystemSource, DesignSystemSourceId, DesignSystemStatus, ProjectId,
};
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
    let storage_paths = design_storage_paths_from_state(&state)?;
    create_design_system_core(&state, &storage_paths, input).await
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

#[doc(hidden)]
pub async fn create_design_system_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: CreateDesignSystemInput,
) -> Result<CreateDesignSystemResponse, String> {
    let primary_project_id =
        parse_non_empty_project_id(&input.primary_project_id, "primary_project_id")?;
    ensure_project_exists(state, &primary_project_id).await?;

    let name = normalize_name(&input.name)?;
    let description = input
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());

    let design_system_id = DesignSystemId::new();
    let storage_root_ref = storage_paths.storage_ref_for_design_system(&design_system_id);
    storage_paths
        .ensure_design_system_root(&storage_root_ref)
        .map_err(|error| error.to_string())?;

    let now = Utc::now();
    let design_system = DesignSystem {
        id: design_system_id.clone(),
        primary_project_id: primary_project_id.clone(),
        name: name.clone(),
        description,
        status: DesignSystemStatus::Draft,
        current_schema_version_id: None,
        storage_root_ref,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    let sources = build_sources(
        state,
        &design_system_id,
        &primary_project_id,
        input.selected_paths,
        input.sources,
    )
    .await?;

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
    primary_project_id: &ProjectId,
    primary_selected_paths: Vec<String>,
    additional_sources: Vec<CreateDesignSystemSourceInput>,
) -> Result<Vec<DesignSystemSource>, String> {
    let mut sources = vec![DesignSystemSource {
        id: DesignSystemSourceId::new(),
        design_system_id: design_system_id.clone(),
        project_id: primary_project_id.clone(),
        role: DesignSourceRole::Primary,
        selected_paths: normalize_selected_paths(primary_selected_paths)?,
        source_kind: DesignSourceKind::ProjectCheckout,
        git_commit: None,
        source_hashes: BTreeMap::new(),
        last_analyzed_at: None,
    }];
    let mut seen_projects = HashSet::from([primary_project_id.as_str().to_string()]);

    for source in additional_sources {
        let project_id = parse_non_empty_project_id(&source.project_id, "source.project_id")?;
        if !seen_projects.insert(project_id.as_str().to_string()) {
            return Err(format!(
                "Duplicate design source project: {}",
                project_id.as_str()
            ));
        }
        ensure_project_exists(state, &project_id).await?;

        sources.push(DesignSystemSource {
            id: DesignSystemSourceId::new(),
            design_system_id: design_system_id.clone(),
            project_id,
            role: parse_additional_source_role(source.role.as_deref())?,
            selected_paths: normalize_selected_paths(source.selected_paths)?,
            source_kind: DesignSourceKind::ProjectCheckout,
            git_commit: None,
            source_hashes: BTreeMap::new(),
            last_analyzed_at: None,
        });
    }

    Ok(sources)
}

async fn ensure_project_exists(state: &AppState, project_id: &ProjectId) -> Result<(), String> {
    state
        .project_repo
        .get_by_id(project_id)
        .await
        .map_err(|error| error.to_string())?
        .map(|_| ())
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))
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
    use crate::domain::entities::{ChatContextType, Project};

    #[tokio::test]
    async fn create_design_system_core_creates_draft_sources_and_conversation() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let project = Project::new("RalphX".to_string(), "/tmp/ralphx".to_string());
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
    async fn create_design_system_core_rejects_unsafe_source_paths() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let project = Project::new("RalphX".to_string(), "/tmp/ralphx".to_string());
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
