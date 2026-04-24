use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tauri::{Manager, State};

use crate::application::AppState;
use crate::commands::design_commands::{DesignSystemResponse, DesignSystemSourceResponse};
use crate::commands::design_feedback_commands::DesignStyleguideItemResponse;
use crate::commands::unified_chat_commands::AgentConversationResponse;
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactId, ArtifactType, ChatConversation, DesignApprovalStatus,
    DesignConfidence, DesignFeedbackStatus, DesignRun, DesignRunKind, DesignRunStatus,
    DesignSchemaVersion, DesignSchemaVersionId, DesignSchemaVersionStatus, DesignSourceKind,
    DesignSourceRole, DesignStyleguideGroup, DesignStyleguideItem, DesignStyleguideItemId,
    DesignSystem, DesignSystemId, DesignSystemSource, DesignSystemSourceId, DesignSystemStatus,
    Project, ProjectId,
};
use crate::utils::design_storage_paths::DesignStoragePaths;

const DESIGN_ARTIFACT_CREATOR: &str = "ralphx-design";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDesignStyleguideViewModelInput {
    pub design_system_id: String,
    pub schema_version_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDesignStyleguidePreviewInput {
    pub design_system_id: String,
    pub preview_artifact_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDesignSystemPackageInput {
    pub design_system_id: String,
    pub include_full_provenance: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDesignSystemPackageInput {
    pub package_artifact_id: String,
    pub attach_project_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignArtifactJsonResponse {
    pub design_system_id: String,
    pub schema_version_id: String,
    pub artifact_id: String,
    pub artifact_type: String,
    pub content: JsonValue,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDesignSystemPackageResponse {
    pub design_system_id: String,
    pub schema_version_id: String,
    pub artifact_id: String,
    pub redacted: bool,
    pub exported_at: String,
    pub content: JsonValue,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDesignSystemPackageResponse {
    pub design_system: DesignSystemResponse,
    pub sources: Vec<DesignSystemSourceResponse>,
    pub conversation: AgentConversationResponse,
    pub schema_version_id: String,
    pub run_id: String,
    pub package_artifact_id: String,
    pub items: Vec<DesignStyleguideItemResponse>,
}

#[tauri::command]
pub async fn get_design_styleguide_view_model(
    input: GetDesignStyleguideViewModelInput,
    state: State<'_, AppState>,
) -> Result<Option<DesignArtifactJsonResponse>, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    get_design_styleguide_view_model_core(&state, &storage_paths, input).await
}

#[tauri::command]
pub async fn get_design_styleguide_preview(
    input: GetDesignStyleguidePreviewInput,
    state: State<'_, AppState>,
) -> Result<DesignArtifactJsonResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    get_design_styleguide_preview_core(&state, &storage_paths, input).await
}

#[tauri::command]
pub async fn export_design_system_package(
    input: ExportDesignSystemPackageInput,
    state: State<'_, AppState>,
) -> Result<ExportDesignSystemPackageResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    export_design_system_package_core(&state, &storage_paths, input).await
}

#[tauri::command]
pub async fn import_design_system_package(
    input: ImportDesignSystemPackageInput,
    state: State<'_, AppState>,
) -> Result<ImportDesignSystemPackageResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    import_design_system_package_core(&state, &storage_paths, input).await
}

#[doc(hidden)]
pub async fn get_design_styleguide_view_model_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: GetDesignStyleguideViewModelInput,
) -> Result<Option<DesignArtifactJsonResponse>, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let system = load_design_system(state, &design_system_id).await?;
    let Some(schema_version) =
        resolve_schema_version(state, &system, input.schema_version_id.as_deref()).await?
    else {
        return Ok(None);
    };

    let content = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &schema_version.styleguide_artifact_id,
        ArtifactType::DesignDoc,
    )
    .await?;

    Ok(Some(DesignArtifactJsonResponse {
        design_system_id: system.id.as_str().to_string(),
        schema_version_id: schema_version.id.as_str().to_string(),
        artifact_id: schema_version.styleguide_artifact_id,
        artifact_type: ArtifactType::DesignDoc.to_string(),
        content,
    }))
}

#[doc(hidden)]
pub async fn get_design_styleguide_preview_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: GetDesignStyleguidePreviewInput,
) -> Result<DesignArtifactJsonResponse, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let preview_artifact_id =
        parse_required_string(&input.preview_artifact_id, "preview_artifact_id")?;
    let system = load_design_system(state, &design_system_id).await?;
    let schema_version_id = system
        .current_schema_version_id
        .as_ref()
        .ok_or_else(|| "Design system has no published styleguide".to_string())?;

    let item = load_item_for_preview_artifact(
        state,
        &design_system_id,
        schema_version_id,
        &preview_artifact_id,
    )
    .await?;
    let content = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &preview_artifact_id,
        ArtifactType::DesignDoc,
    )
    .await?;

    if let Some(item_id) = content.get("item_id").and_then(JsonValue::as_str) {
        if item_id != item.item_id {
            return Err(
                "Design preview artifact does not match the linked styleguide item".to_string(),
            );
        }
    }

    Ok(DesignArtifactJsonResponse {
        design_system_id: system.id.as_str().to_string(),
        schema_version_id: schema_version_id.as_str().to_string(),
        artifact_id: preview_artifact_id,
        artifact_type: ArtifactType::DesignDoc.to_string(),
        content,
    })
}

#[doc(hidden)]
pub async fn export_design_system_package_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: ExportDesignSystemPackageInput,
) -> Result<ExportDesignSystemPackageResponse, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let system = load_design_system(state, &design_system_id).await?;
    let schema_version = resolve_schema_version(state, &system, None)
        .await?
        .ok_or_else(|| "Design system has no published schema to export".to_string())?;
    let include_full_provenance = input.include_full_provenance.unwrap_or(false);
    let redacted = !include_full_provenance;
    let exported_at = Utc::now();

    let mut schema = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &schema_version.schema_artifact_id,
        ArtifactType::Specification,
    )
    .await?;
    let mut source_audit = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &schema_version.manifest_artifact_id,
        ArtifactType::Findings,
    )
    .await?;
    let mut styleguide = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &schema_version.styleguide_artifact_id,
        ArtifactType::DesignDoc,
    )
    .await?;

    if redacted {
        redact_source_provenance(&mut schema);
        redact_source_provenance(&mut source_audit);
        redact_source_provenance(&mut styleguide);
    }

    let package = json!({
        "package_version": "1.0",
        "exported_at": exported_at.to_rfc3339(),
        "redacted": redacted,
        "design_system": {
            "id": system.id.as_str(),
            "name": system.name.as_str(),
            "schema_version_id": schema_version.id.as_str(),
            "version": schema_version.version.as_str(),
        },
        "schema": schema,
        "source_audit": source_audit,
        "styleguide": styleguide,
    });

    let storage_root = storage_paths
        .ensure_design_system_root(&system.storage_root_ref)
        .map_err(|error| error.to_string())?;
    let version_component = storage_paths.schema_version_component(&schema_version.id);
    let package_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("exports")
                .join(version_component)
                .join("design-system-package.json"),
            &package,
        )
        .map_err(|error| error.to_string())?;
    let artifact = Artifact::new_file(
        format!("Design export: {} {}", system.name, schema_version.version),
        ArtifactType::DesignDoc,
        package_path.to_string_lossy().to_string(),
        DESIGN_ARTIFACT_CREATOR,
    );
    let artifact = state
        .artifact_repo
        .create(artifact)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ExportDesignSystemPackageResponse {
        design_system_id: system.id.as_str().to_string(),
        schema_version_id: schema_version.id.as_str().to_string(),
        artifact_id: artifact.id.as_str().to_string(),
        redacted,
        exported_at: exported_at.to_rfc3339(),
        content: package,
    })
}

#[doc(hidden)]
pub async fn import_design_system_package_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: ImportDesignSystemPackageInput,
) -> Result<ImportDesignSystemPackageResponse, String> {
    let package_artifact_id =
        parse_required_string(&input.package_artifact_id, "package_artifact_id")?;
    let attach_project_id = parse_project_id(&input.attach_project_id, "attach_project_id")?;
    let project = load_project(state, &attach_project_id).await?;
    let package_artifact_id = ArtifactId::from_string(package_artifact_id);
    let package =
        read_design_package_artifact_json(state, storage_paths, &package_artifact_id).await?;
    validate_design_package(&package)?;

    let imported_name = normalize_import_name(
        input.name.as_deref(),
        package
            .get("design_system")
            .and_then(|value| value.get("name"))
            .and_then(JsonValue::as_str),
        &project,
    )?;
    let now = Utc::now();
    let design_system_id = DesignSystemId::new();
    let schema_version_id = DesignSchemaVersionId::new();
    let storage_root_ref = storage_paths.storage_ref_for_design_system(&design_system_id);
    let version = imported_version_label(&package);

    let storage_root = storage_paths
        .ensure_design_system_root(&storage_root_ref)
        .map_err(|error| error.to_string())?;
    let version_component = storage_paths.schema_version_component(&schema_version_id);

    let mut schema = package
        .get("schema")
        .cloned()
        .ok_or_else(|| "Design package is missing schema".to_string())?;
    let mut source_audit = package
        .get("source_audit")
        .cloned()
        .ok_or_else(|| "Design package is missing source_audit".to_string())?;
    let mut styleguide = package
        .get("styleguide")
        .cloned()
        .ok_or_else(|| "Design package is missing styleguide".to_string())?;
    rewrite_imported_design_identity(
        &mut schema,
        &design_system_id,
        &schema_version_id,
        &imported_name,
        &version,
    );
    rewrite_imported_design_identity(
        &mut source_audit,
        &design_system_id,
        &schema_version_id,
        &imported_name,
        &version,
    );
    rewrite_imported_design_identity(
        &mut styleguide,
        &design_system_id,
        &schema_version_id,
        &imported_name,
        &version,
    );
    strip_imported_preview_artifact_refs(&mut styleguide);

    let items =
        styleguide_items_from_package(&styleguide, &design_system_id, &schema_version_id, now)?;
    let sources = vec![DesignSystemSource {
        id: DesignSystemSourceId::new(),
        design_system_id: design_system_id.clone(),
        project_id: attach_project_id.clone(),
        role: DesignSourceRole::Primary,
        selected_paths: Vec::new(),
        source_kind: DesignSourceKind::ManualNote,
        git_commit: None,
        source_hashes: BTreeMap::new(),
        last_analyzed_at: Some(now),
    }];

    let schema_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("imports")
                .join(&version_component)
                .join("design-system.schema.json"),
            &schema,
        )
        .map_err(|error| error.to_string())?;
    let schema_artifact_id = create_design_file_artifact(
        state,
        format!("Imported design schema: {imported_name} {version}"),
        ArtifactType::Specification,
        &schema_path,
        Some(package_artifact_id.clone()),
    )
    .await?;

    let manifest_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("imports")
                .join(&version_component)
                .join("source-audit.json"),
            &source_audit,
        )
        .map_err(|error| error.to_string())?;
    let manifest_artifact_id = create_design_file_artifact(
        state,
        format!("Imported design source audit: {imported_name} {version}"),
        ArtifactType::Findings,
        &manifest_path,
        Some(package_artifact_id.clone()),
    )
    .await?;

    let styleguide_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("imports")
                .join(&version_component)
                .join("styleguide-view-model.json"),
            &styleguide,
        )
        .map_err(|error| error.to_string())?;
    let styleguide_artifact_id = create_design_file_artifact(
        state,
        format!("Imported design styleguide: {imported_name} {version}"),
        ArtifactType::DesignDoc,
        &styleguide_path,
        Some(package_artifact_id.clone()),
    )
    .await?;

    let design_system = DesignSystem {
        id: design_system_id.clone(),
        primary_project_id: attach_project_id,
        name: imported_name.clone(),
        description: Some("Imported design system package".to_string()),
        status: DesignSystemStatus::Ready,
        current_schema_version_id: Some(schema_version_id.clone()),
        storage_root_ref,
        created_at: now,
        updated_at: now,
        archived_at: None,
    };
    let design_system = state
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
    conversation.set_title(format!("Design: {imported_name}"));
    let conversation = state
        .chat_conversation_repo
        .create(conversation)
        .await
        .map_err(|error| error.to_string())?;

    let mut run = DesignRun::queued(
        design_system_id.clone(),
        DesignRunKind::Import,
        format!(
            "Imported package artifact {} into {}",
            package_artifact_id.as_str(),
            project.name
        ),
    );
    run.status = DesignRunStatus::Running;
    run.started_at = Some(now);
    run.conversation_id = Some(conversation.id);
    let mut run = state
        .design_run_repo
        .create(run)
        .await
        .map_err(|error| error.to_string())?;

    let schema_version = DesignSchemaVersion {
        id: schema_version_id.clone(),
        design_system_id: design_system_id.clone(),
        version,
        schema_artifact_id: schema_artifact_id.clone(),
        manifest_artifact_id: manifest_artifact_id.clone(),
        styleguide_artifact_id: styleguide_artifact_id.clone(),
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

    run.status = DesignRunStatus::Completed;
    run.completed_at = Some(now);
    run.output_artifact_ids = vec![
        schema_artifact_id,
        manifest_artifact_id,
        styleguide_artifact_id,
    ];
    state
        .design_run_repo
        .update(&run)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ImportDesignSystemPackageResponse {
        design_system: DesignSystemResponse::from(design_system),
        sources: sources
            .into_iter()
            .map(DesignSystemSourceResponse::from)
            .collect(),
        conversation: AgentConversationResponse::from(conversation),
        schema_version_id: schema_version.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        package_artifact_id: package_artifact_id.as_str().to_string(),
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

async fn load_design_system(
    state: &AppState,
    design_system_id: &DesignSystemId,
) -> Result<DesignSystem, String> {
    state
        .design_system_repo
        .get_by_id(design_system_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Design system not found: {}", design_system_id.as_str()))
}

async fn resolve_schema_version(
    state: &AppState,
    system: &DesignSystem,
    schema_version_id: Option<&str>,
) -> Result<Option<DesignSchemaVersion>, String> {
    if let Some(schema_version_id) = schema_version_id {
        let schema_version_id = parse_schema_version_id(schema_version_id)?;
        let Some(schema_version) = state
            .design_schema_repo
            .get_version(&schema_version_id)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Err(format!(
                "Design schema version not found: {}",
                schema_version_id.as_str()
            ));
        };
        if schema_version.design_system_id != system.id {
            return Err(
                "Design schema version does not belong to the selected design system".to_string(),
            );
        }
        return Ok(Some(schema_version));
    }

    let Some(schema_version_id) = system.current_schema_version_id.as_ref() else {
        return Ok(None);
    };
    state
        .design_schema_repo
        .get_version(schema_version_id)
        .await
        .map_err(|error| error.to_string())
}

async fn load_item_for_preview_artifact(
    state: &AppState,
    design_system_id: &DesignSystemId,
    schema_version_id: &DesignSchemaVersionId,
    preview_artifact_id: &str,
) -> Result<DesignStyleguideItem, String> {
    state
        .design_styleguide_repo
        .list_items(design_system_id, Some(schema_version_id))
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|item| item.preview_artifact_id.as_deref() == Some(preview_artifact_id))
        .ok_or_else(|| {
            "Design preview artifact is not linked to the current styleguide".to_string()
        })
}

async fn read_design_file_artifact_json(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    system: &DesignSystem,
    artifact_id: &str,
    expected_type: ArtifactType,
) -> Result<JsonValue, String> {
    let artifact_id = ArtifactId::from_string(artifact_id.to_string());
    let artifact = state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Design artifact not found: {}", artifact_id.as_str()))?;

    if artifact.artifact_type != expected_type {
        return Err(format!(
            "Design artifact {} has type {}, expected {}",
            artifact.id.as_str(),
            artifact.artifact_type,
            expected_type
        ));
    }
    if artifact.metadata.created_by != DESIGN_ARTIFACT_CREATOR {
        return Err("Design artifact was not created by RalphX Design".to_string());
    }

    let path = match artifact.content {
        ArtifactContent::File { path } => PathBuf::from(path),
        ArtifactContent::Inline { .. } => {
            return Err("Design artifact must be backed by a JSON file".to_string());
        }
    };
    let storage_root = storage_paths
        .ensure_design_system_root(&system.storage_root_ref)
        .map_err(|error| error.to_string())?;
    storage_paths
        .read_json_file::<JsonValue>(&storage_root, Path::new(&path))
        .map_err(|error| error.to_string())
}

async fn read_design_package_artifact_json(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    artifact_id: &ArtifactId,
) -> Result<JsonValue, String> {
    let artifact = state
        .artifact_repo
        .get_by_id(artifact_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "Design package artifact not found: {}",
                artifact_id.as_str()
            )
        })?;

    if artifact.artifact_type != ArtifactType::DesignDoc {
        return Err(format!(
            "Design package artifact {} has type {}, expected {}",
            artifact.id.as_str(),
            artifact.artifact_type,
            ArtifactType::DesignDoc
        ));
    }
    if artifact.metadata.created_by != DESIGN_ARTIFACT_CREATOR {
        return Err("Design package artifact was not created by RalphX Design".to_string());
    }

    let path = match artifact.content {
        ArtifactContent::File { path } => PathBuf::from(path),
        ArtifactContent::Inline { .. } => {
            return Err("Design package artifact must be backed by a JSON file".to_string());
        }
    };
    storage_paths
        .read_json_file_under_design_storage_root::<JsonValue>(&path)
        .map_err(|error| error.to_string())
}

async fn create_design_file_artifact(
    state: &AppState,
    name: String,
    artifact_type: ArtifactType,
    path: &Path,
    derived_from: Option<ArtifactId>,
) -> Result<String, String> {
    let mut artifact = Artifact::new_file(
        name,
        artifact_type,
        path.to_string_lossy().to_string(),
        DESIGN_ARTIFACT_CREATOR,
    );
    if let Some(parent) = derived_from {
        artifact = artifact.derived_from_artifact(parent);
    }
    state
        .artifact_repo
        .create(artifact)
        .await
        .map(|artifact| artifact.id.as_str().to_string())
        .map_err(|error| error.to_string())
}

async fn load_project(state: &AppState, project_id: &ProjectId) -> Result<Project, String> {
    state
        .project_repo
        .get_by_id(project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))
}

fn validate_design_package(package: &JsonValue) -> Result<(), String> {
    if package.get("package_version").and_then(JsonValue::as_str) != Some("1.0") {
        return Err("Design package has unsupported package_version".to_string());
    }
    for key in ["design_system", "schema", "source_audit", "styleguide"] {
        if !package.get(key).is_some_and(JsonValue::is_object) {
            return Err(format!("Design package is missing {key}"));
        }
    }
    let groups = package
        .get("styleguide")
        .and_then(|styleguide| styleguide.get("groups"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "Design package styleguide is missing groups".to_string())?;
    if groups.is_empty() {
        return Err("Design package styleguide has no groups".to_string());
    }
    Ok(())
}

fn normalize_import_name(
    requested_name: Option<&str>,
    package_name: Option<&str>,
    project: &Project,
) -> Result<String, String> {
    let fallback = package_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("{value} Import"))
        .unwrap_or_else(|| format!("{} Design System Import", project.name));
    let name = requested_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&fallback);
    if name.is_empty() {
        return Err("Imported design system name is required".to_string());
    }
    Ok(name.to_string())
}

fn imported_version_label(package: &JsonValue) -> String {
    package
        .get("design_system")
        .and_then(|value| value.get("version"))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("imported")
        .to_string()
}

fn rewrite_imported_design_identity(
    value: &mut JsonValue,
    design_system_id: &DesignSystemId,
    schema_version_id: &DesignSchemaVersionId,
    name: &str,
    version: &str,
) {
    match value {
        JsonValue::Object(map) => {
            if map.contains_key("design_system_id") {
                map.insert(
                    "design_system_id".to_string(),
                    JsonValue::String(design_system_id.as_str().to_string()),
                );
            }
            if map.contains_key("schema_version_id") {
                map.insert(
                    "schema_version_id".to_string(),
                    JsonValue::String(schema_version_id.as_str().to_string()),
                );
            }
            if let Some(JsonValue::Object(design_system)) = map.get_mut("design_system") {
                design_system.insert(
                    "id".to_string(),
                    JsonValue::String(design_system_id.as_str().to_string()),
                );
                design_system.insert("name".to_string(), JsonValue::String(name.to_string()));
                design_system.insert(
                    "schema_version_id".to_string(),
                    JsonValue::String(schema_version_id.as_str().to_string()),
                );
                design_system.insert(
                    "version".to_string(),
                    JsonValue::String(version.to_string()),
                );
            }
            for child in map.values_mut() {
                rewrite_imported_design_identity(
                    child,
                    design_system_id,
                    schema_version_id,
                    name,
                    version,
                );
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                rewrite_imported_design_identity(
                    child,
                    design_system_id,
                    schema_version_id,
                    name,
                    version,
                );
            }
        }
        _ => {}
    }
}

fn strip_imported_preview_artifact_refs(value: &mut JsonValue) {
    match value {
        JsonValue::Object(map) => {
            map.remove("preview_artifact_id");
            for child in map.values_mut() {
                strip_imported_preview_artifact_refs(child);
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                strip_imported_preview_artifact_refs(child);
            }
        }
        _ => {}
    }
}

fn styleguide_items_from_package(
    styleguide: &JsonValue,
    design_system_id: &DesignSystemId,
    schema_version_id: &DesignSchemaVersionId,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<DesignStyleguideItem>, String> {
    let groups = styleguide
        .get("groups")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "Design package styleguide is missing groups".to_string())?;
    let mut items = Vec::new();
    let mut seen_item_ids = HashSet::new();

    for (group_index, group_value) in groups.iter().enumerate() {
        let fallback_group = group_value
            .get("id")
            .and_then(JsonValue::as_str)
            .and_then(parse_styleguide_group)
            .ok_or_else(|| format!("Design package styleguide group {group_index} is invalid"))?;
        let group_items = group_value
            .get("items")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| {
                format!("Design package styleguide group {group_index} is missing items")
            })?;

        for (item_index, item_value) in group_items.iter().enumerate() {
            let item_id = imported_item_id(item_value, group_index, item_index, &mut seen_item_ids);
            let group = item_value
                .get("group")
                .and_then(JsonValue::as_str)
                .and_then(parse_styleguide_group)
                .unwrap_or(fallback_group);
            let label = item_value
                .get("label")
                .and_then(JsonValue::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&item_id)
                .to_string();
            let summary = item_value
                .get("summary")
                .and_then(JsonValue::as_str)
                .unwrap_or_default()
                .to_string();
            let confidence = item_value
                .get("confidence")
                .and_then(JsonValue::as_str)
                .and_then(parse_confidence)
                .unwrap_or(DesignConfidence::Medium);
            let approval_status = item_value
                .get("approval_status")
                .and_then(JsonValue::as_str)
                .and_then(parse_approval_status)
                .unwrap_or(DesignApprovalStatus::NeedsReview);
            let feedback_status = item_value
                .get("feedback_status")
                .and_then(JsonValue::as_str)
                .and_then(parse_feedback_status)
                .unwrap_or(DesignFeedbackStatus::None);

            items.push(DesignStyleguideItem {
                id: DesignStyleguideItemId::new(),
                design_system_id: design_system_id.clone(),
                schema_version_id: schema_version_id.clone(),
                item_id,
                group,
                label,
                summary,
                preview_artifact_id: None,
                source_refs: Vec::new(),
                confidence,
                approval_status,
                feedback_status,
                updated_at: now,
            });
        }
    }

    if items.is_empty() {
        return Err("Design package styleguide has no items".to_string());
    }
    Ok(items)
}

fn imported_item_id(
    item: &JsonValue,
    group_index: usize,
    item_index: usize,
    seen_item_ids: &mut HashSet<String>,
) -> String {
    let base = item
        .get("id")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("imported.{group_index}.{item_index}"));
    if seen_item_ids.insert(base.clone()) {
        return base;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{base}.{suffix}");
        if seen_item_ids.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn parse_styleguide_group(value: &str) -> Option<DesignStyleguideGroup> {
    match value.trim() {
        "ui_kit" => Some(DesignStyleguideGroup::UiKit),
        "type" => Some(DesignStyleguideGroup::Type),
        "colors" => Some(DesignStyleguideGroup::Colors),
        "spacing" => Some(DesignStyleguideGroup::Spacing),
        "components" => Some(DesignStyleguideGroup::Components),
        "brand" => Some(DesignStyleguideGroup::Brand),
        _ => None,
    }
}

fn parse_confidence(value: &str) -> Option<DesignConfidence> {
    match value.trim() {
        "high" => Some(DesignConfidence::High),
        "medium" => Some(DesignConfidence::Medium),
        "low" => Some(DesignConfidence::Low),
        _ => None,
    }
}

fn parse_approval_status(value: &str) -> Option<DesignApprovalStatus> {
    match value.trim() {
        "needs_review" => Some(DesignApprovalStatus::NeedsReview),
        "approved" => Some(DesignApprovalStatus::Approved),
        "needs_work" => Some(DesignApprovalStatus::NeedsWork),
        _ => None,
    }
}

fn parse_feedback_status(value: &str) -> Option<DesignFeedbackStatus> {
    match value.trim() {
        "none" => Some(DesignFeedbackStatus::None),
        "open" => Some(DesignFeedbackStatus::Open),
        "in_progress" => Some(DesignFeedbackStatus::InProgress),
        "resolved" => Some(DesignFeedbackStatus::Resolved),
        "dismissed" => Some(DesignFeedbackStatus::Dismissed),
        _ => None,
    }
}

fn parse_design_system_id(value: &str) -> Result<DesignSystemId, String> {
    let value = parse_required_string(value, "design_system_id")?;
    Ok(DesignSystemId::from_string(value))
}

fn parse_project_id(value: &str, field: &str) -> Result<ProjectId, String> {
    let value = parse_required_string(value, field)?;
    Ok(ProjectId::from_string(value))
}

fn parse_schema_version_id(value: &str) -> Result<DesignSchemaVersionId, String> {
    let value = parse_required_string(value, "schema_version_id")?;
    Ok(DesignSchemaVersionId::from_string(value))
}

fn parse_required_string(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    Ok(value.to_string())
}

fn redact_source_provenance(value: &mut JsonValue) {
    match value {
        JsonValue::Object(map) => {
            for key in [
                "source_refs",
                "source_hashes",
                "selected_paths",
                "git_commit",
                "last_analyzed_at",
            ] {
                map.remove(key);
            }
            for value in map.values_mut() {
                redact_source_provenance(value);
            }
        }
        JsonValue::Array(values) => {
            for value in values {
                redact_source_provenance(value);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::design_commands::{
        create_design_system_core, generate_design_system_styleguide_core, CreateDesignSystemInput,
        GenerateDesignSystemStyleguideInput,
    };
    use crate::domain::entities::{Artifact, ArtifactContent, Project};

    fn write_project_file(root: &Path, relative_path: &str, content: &str) {
        let path = root.join(relative_path);
        let parent = path.parent().expect("test path parent");
        std::fs::create_dir_all(parent).expect("create parent");
        std::fs::write(path, content).expect("write file");
    }

    async fn create_generated_design_system(
        state: &AppState,
        storage_paths: &DesignStoragePaths,
    ) -> (DesignSystemId, String) {
        let project_root = tempfile::tempdir().expect("project tempdir");
        write_project_file(
            project_root.path(),
            "frontend/src/components/ui/button.tsx",
            "export function Button() {}\n",
        );
        let project = Project::new(
            "RalphX".to_string(),
            project_root.path().to_string_lossy().to_string(),
        );
        state.project_repo.create(project.clone()).await.unwrap();

        let draft = create_design_system_core(
            state,
            storage_paths,
            CreateDesignSystemInput {
                primary_project_id: project.id.as_str().to_string(),
                name: "Product UI".to_string(),
                description: None,
                selected_paths: vec!["frontend/src".to_string()],
                sources: Vec::new(),
            },
        )
        .await
        .expect("create design system");
        let design_system_id = DesignSystemId::from_string(draft.design_system.id);
        let generated = generate_design_system_styleguide_core(
            state,
            storage_paths,
            GenerateDesignSystemStyleguideInput {
                design_system_id: design_system_id.as_str().to_string(),
            },
        )
        .await
        .expect("generate styleguide");
        let preview_artifact_id = generated.items[0]
            .preview_artifact_id
            .clone()
            .expect("preview artifact id");

        (design_system_id, preview_artifact_id)
    }

    #[tokio::test]
    async fn reads_persisted_styleguide_view_model_and_preview_json() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let (design_system_id, preview_artifact_id) =
            create_generated_design_system(&state, &storage_paths).await;

        let view_model = get_design_styleguide_view_model_core(
            &state,
            &storage_paths,
            GetDesignStyleguideViewModelInput {
                design_system_id: design_system_id.as_str().to_string(),
                schema_version_id: None,
            },
        )
        .await
        .expect("view model")
        .expect("published view model");
        assert_eq!(view_model.design_system_id, design_system_id.as_str());
        assert_eq!(view_model.content["groups"].as_array().unwrap().len(), 6);

        let preview = get_design_styleguide_preview_core(
            &state,
            &storage_paths,
            GetDesignStyleguidePreviewInput {
                design_system_id: design_system_id.as_str().to_string(),
                preview_artifact_id,
            },
        )
        .await
        .expect("preview");
        assert_eq!(
            preview.content["design_system_id"].as_str(),
            Some(design_system_id.as_str())
        );
        assert!(preview.content["preview_kind"].as_str().is_some());
    }

    #[tokio::test]
    async fn preview_read_rejects_linked_file_outside_design_storage() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let (design_system_id, preview_artifact_id) =
            create_generated_design_system(&state, &storage_paths).await;
        let mut item = state
            .design_styleguide_repo
            .list_items(&design_system_id, None)
            .await
            .unwrap()
            .into_iter()
            .find(|item| item.preview_artifact_id.as_deref() == Some(preview_artifact_id.as_str()))
            .expect("styleguide item");

        let outside = tempfile::tempdir().expect("outside tempdir");
        let outside_file = outside.path().join("preview.json");
        std::fs::write(&outside_file, "{\"item_id\":\"components.buttons\"}")
            .expect("outside preview");
        let outside_artifact = Artifact::new_file(
            "Outside preview",
            ArtifactType::DesignDoc,
            outside_file.to_string_lossy().to_string(),
            DESIGN_ARTIFACT_CREATOR,
        );
        let outside_artifact_id = outside_artifact.id.as_str().to_string();
        state
            .artifact_repo
            .create(outside_artifact)
            .await
            .expect("outside artifact");
        item.preview_artifact_id = Some(outside_artifact_id.clone());
        state
            .design_styleguide_repo
            .update_item(&item)
            .await
            .expect("update linked preview");

        let error = get_design_styleguide_preview_core(
            &state,
            &storage_paths,
            GetDesignStyleguidePreviewInput {
                design_system_id: design_system_id.as_str().to_string(),
                preview_artifact_id: outside_artifact_id,
            },
        )
        .await
        .expect_err("outside preview should fail");

        assert!(error.contains("escaped RalphX-owned design storage"));
    }

    #[tokio::test]
    async fn export_package_writes_redacted_design_artifact_inside_storage() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let (design_system_id, _) = create_generated_design_system(&state, &storage_paths).await;

        let response = export_design_system_package_core(
            &state,
            &storage_paths,
            ExportDesignSystemPackageInput {
                design_system_id: design_system_id.as_str().to_string(),
                include_full_provenance: None,
            },
        )
        .await
        .expect("export package");

        assert!(response.redacted);
        assert_eq!(
            response.content["design_system"]["id"].as_str(),
            Some(design_system_id.as_str())
        );
        assert!(response.content.to_string().contains("\"redacted\":true"));
        assert!(!response.content.to_string().contains("source_hashes"));
        assert!(!response.content.to_string().contains("selected_paths"));

        let artifact = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(response.artifact_id))
            .await
            .unwrap()
            .expect("export artifact");
        let path = match artifact.content {
            ArtifactContent::File { path } => PathBuf::from(path),
            ArtifactContent::Inline { .. } => panic!("expected file artifact"),
        };
        assert!(path.starts_with(temp.path().canonicalize().unwrap()));
        let package_json: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(path).expect("package json"))
                .expect("parse package json");
        assert_eq!(package_json["redacted"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn import_package_creates_ready_design_system_from_export_artifact() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let (design_system_id, _) = create_generated_design_system(&state, &storage_paths).await;
        let attach_root = tempfile::tempdir().expect("attach project tempdir");
        let attach_project = Project::new(
            "Imported Host".to_string(),
            attach_root.path().to_string_lossy().to_string(),
        );
        state
            .project_repo
            .create(attach_project.clone())
            .await
            .expect("attach project");
        let export = export_design_system_package_core(
            &state,
            &storage_paths,
            ExportDesignSystemPackageInput {
                design_system_id: design_system_id.as_str().to_string(),
                include_full_provenance: None,
            },
        )
        .await
        .expect("export package");

        let imported = import_design_system_package_core(
            &state,
            &storage_paths,
            ImportDesignSystemPackageInput {
                package_artifact_id: export.artifact_id.clone(),
                attach_project_id: attach_project.id.as_str().to_string(),
                name: Some("Imported Product UI".to_string()),
            },
        )
        .await
        .expect("import package");

        assert_eq!(imported.design_system.name, "Imported Product UI");
        assert_eq!(imported.design_system.status, "ready");
        assert_eq!(
            imported.design_system.primary_project_id,
            attach_project.id.as_str()
        );
        assert_eq!(
            imported.design_system.current_schema_version_id.as_deref(),
            Some(imported.schema_version_id.as_str())
        );
        assert_eq!(imported.sources.len(), 1);
        assert_eq!(imported.sources[0].source_kind, "manual_note");
        assert_eq!(imported.sources[0].source_hashes.len(), 0);
        assert_eq!(imported.items.len(), 6);
        assert!(imported
            .items
            .iter()
            .all(|item| item.preview_artifact_id.is_none() && item.source_refs.is_empty()));
        assert_eq!(imported.conversation.context_type, "design");
        assert_eq!(imported.conversation.context_id, imported.design_system.id);

        let imported_system_id = DesignSystemId::from_string(imported.design_system.id.clone());
        let schema_version = state
            .design_schema_repo
            .get_current_for_design_system(&imported_system_id)
            .await
            .unwrap()
            .expect("imported schema");
        let schema_artifact = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(
                schema_version.schema_artifact_id.clone(),
            ))
            .await
            .unwrap()
            .expect("schema artifact");
        assert_eq!(
            schema_artifact.derived_from,
            vec![ArtifactId::from_string(export.artifact_id.clone())]
        );
        let schema_path = match schema_artifact.content {
            ArtifactContent::File { path } => PathBuf::from(path),
            ArtifactContent::Inline { .. } => panic!("expected schema file"),
        };
        assert!(schema_path.starts_with(temp.path().canonicalize().unwrap()));
        let schema_json: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(schema_path).expect("schema json"))
                .expect("parse schema");
        assert_eq!(
            schema_json["design_system"]["id"].as_str(),
            Some(imported.design_system.id.as_str())
        );

        let runs = state
            .design_run_repo
            .list_by_design_system(&imported_system_id)
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].kind, DesignRunKind::Import);
        assert_eq!(runs[0].status, DesignRunStatus::Completed);
        assert_eq!(runs[0].output_artifact_ids.len(), 3);
    }

    #[tokio::test]
    async fn import_package_rejects_file_artifact_outside_design_storage() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = storage_paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        storage_paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");
        let attach_project = Project::new(
            "Imported Host".to_string(),
            outside.path().to_string_lossy().to_string(),
        );
        state
            .project_repo
            .create(attach_project.clone())
            .await
            .expect("attach project");
        let outside_package = outside.path().join("design-system-package.json");
        std::fs::write(
            &outside_package,
            serde_json::to_vec(&json!({
                "package_version": "1.0",
                "design_system": { "name": "Outside" },
                "schema": {},
                "source_audit": {},
                "styleguide": { "groups": [{ "id": "components", "items": [] }] }
            }))
            .expect("package json"),
        )
        .expect("write outside package");
        let artifact = Artifact::new_file(
            "Outside design package",
            ArtifactType::DesignDoc,
            outside_package.to_string_lossy().to_string(),
            DESIGN_ARTIFACT_CREATOR,
        );
        let artifact_id = artifact.id.as_str().to_string();
        state
            .artifact_repo
            .create(artifact)
            .await
            .expect("outside artifact");

        let error = import_design_system_package_core(
            &state,
            &storage_paths,
            ImportDesignSystemPackageInput {
                package_artifact_id: artifact_id,
                attach_project_id: attach_project.id.as_str().to_string(),
                name: None,
            },
        )
        .await
        .expect_err("outside package should fail");

        assert!(error.contains("escaped RalphX-owned design storage"));
    }
}
