use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tauri::{Emitter, Manager, State};

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
const DESIGN_PACKAGE_VERSION: &str = "1.0";

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
    pub destination_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDesignSystemPackageInput {
    pub package_artifact_id: Option<String>,
    pub package_path: Option<String>,
    pub attach_project_id: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDesignArtifactInput {
    pub design_system_id: String,
    pub artifact_kind: String,
    pub name: String,
    pub brief: Option<String>,
    pub source_item_id: Option<String>,
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
    pub run_id: String,
    pub artifact_id: String,
    pub redacted: bool,
    pub exported_at: String,
    pub content: JsonValue,
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDesignSystemPackageResponse {
    pub design_system: DesignSystemResponse,
    pub sources: Vec<DesignSystemSourceResponse>,
    pub conversation: AgentConversationResponse,
    pub schema_version_id: String,
    pub run_id: String,
    pub package_artifact_id: Option<String>,
    pub package_path: Option<String>,
    pub items: Vec<DesignStyleguideItemResponse>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenerateDesignArtifactResponse {
    pub design_system_id: String,
    pub schema_version_id: String,
    pub run_id: String,
    pub artifact_id: String,
    pub preview_artifact_id: String,
    pub artifact_kind: String,
    pub name: String,
    pub created_at: String,
    pub content: JsonValue,
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
    app: tauri::AppHandle,
) -> Result<ExportDesignSystemPackageResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    let response = export_design_system_package_core(&state, &storage_paths, input).await?;
    let _ = app.emit("design:export_completed", &response);
    if let Ok(Some(run)) = state
        .design_run_repo
        .get_by_id(&crate::domain::entities::DesignRunId::from_string(
            response.run_id.clone(),
        ))
        .await
    {
        let _ = app.emit(
            "design:run_completed",
            crate::commands::design_commands::DesignRunEventPayload::from(&run),
        );
    }
    Ok(response)
}

#[tauri::command]
pub async fn import_design_system_package(
    input: ImportDesignSystemPackageInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ImportDesignSystemPackageResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    let response = import_design_system_package_core(&state, &storage_paths, input).await?;
    let _ = app.emit("design:import_completed", &response);
    if let Ok(Some(run)) = state
        .design_run_repo
        .get_by_id(&crate::domain::entities::DesignRunId::from_string(
            response.run_id.clone(),
        ))
        .await
    {
        let _ = app.emit(
            "design:run_completed",
            crate::commands::design_commands::DesignRunEventPayload::from(&run),
        );
    }
    Ok(response)
}

#[tauri::command]
pub async fn generate_design_artifact(
    input: GenerateDesignArtifactInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<GenerateDesignArtifactResponse, String> {
    let storage_paths = design_storage_paths_from_state(&state)?;
    let response = generate_design_artifact_core(&state, &storage_paths, input).await?;
    let _ = app.emit("design:artifact_created", &response);
    Ok(response)
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
        "package_version": DESIGN_PACKAGE_VERSION,
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
    let package_bytes = serialize_design_package_zip(&package)?;
    let package_path = storage_paths
        .write_file(
            &storage_root,
            PathBuf::from("exports")
                .join(version_component)
                .join("design-system-export.zip"),
            &package_bytes,
        )
        .map_err(|error| error.to_string())?;
    let exported_file_path = input
        .destination_path
        .as_deref()
        .map(|path| write_user_selected_design_package(path, &package))
        .transpose()?;
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
    let mut run = DesignRun::queued(
        system.id.clone(),
        DesignRunKind::Export,
        format!(
            "Exported design package for {} {}",
            system.name, schema_version.version
        ),
    );
    run.status = DesignRunStatus::Completed;
    run.started_at = Some(exported_at);
    run.completed_at = Some(exported_at);
    run.output_artifact_ids = vec![artifact.id.as_str().to_string()];
    let run = state
        .design_run_repo
        .create(run)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ExportDesignSystemPackageResponse {
        design_system_id: system.id.as_str().to_string(),
        schema_version_id: schema_version.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        artifact_id: artifact.id.as_str().to_string(),
        redacted,
        exported_at: exported_at.to_rfc3339(),
        content: package,
        file_path: exported_file_path,
    })
}

#[doc(hidden)]
pub async fn import_design_system_package_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: ImportDesignSystemPackageInput,
) -> Result<ImportDesignSystemPackageResponse, String> {
    let attach_project_id = parse_project_id(&input.attach_project_id, "attach_project_id")?;
    let project = load_project(state, &attach_project_id).await?;
    let (package, package_artifact_id, package_path) = match (
        input.package_artifact_id.as_deref(),
        input.package_path.as_deref(),
    ) {
        (Some(raw_artifact_id), None) if !raw_artifact_id.trim().is_empty() => {
            let package_artifact_id = ArtifactId::from_string(parse_required_string(
                raw_artifact_id,
                "package_artifact_id",
            )?);
            let package =
                read_design_package_artifact_json(state, storage_paths, &package_artifact_id)
                    .await?;
            (package, Some(package_artifact_id), None)
        }
        (None, Some(raw_package_path)) if !raw_package_path.trim().is_empty() => {
            let path = validate_user_selected_design_package_path(raw_package_path, true)?;
            let package = read_user_selected_design_package(&path)?;
            (package, None, Some(path.to_string_lossy().to_string()))
        }
        _ => {
            return Err(
                "Import requires exactly one of package_artifact_id or package_path".to_string(),
            )
        }
    };
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
        package_artifact_id.clone(),
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
        package_artifact_id.clone(),
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
        package_artifact_id.clone(),
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
            "Imported package {} into {}",
            package_import_source_label(package_artifact_id.as_ref(), package_path.as_deref()),
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
        package_artifact_id: package_artifact_id.map(|id| id.as_str().to_string()),
        package_path,
        items: items
            .into_iter()
            .map(DesignStyleguideItemResponse::from)
            .collect(),
    })
}

#[doc(hidden)]
pub async fn generate_design_artifact_core(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    input: GenerateDesignArtifactInput,
) -> Result<GenerateDesignArtifactResponse, String> {
    let design_system_id = parse_design_system_id(&input.design_system_id)?;
    let artifact_kind = GeneratedDesignArtifactKind::parse(&input.artifact_kind)?;
    let name = parse_required_string(&input.name, "name")?;
    let brief = input
        .brief
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let source_item_id = input
        .source_item_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let system = load_design_system(state, &design_system_id).await?;
    if system.archived_at.is_some() || system.status == DesignSystemStatus::Archived {
        return Err("Archived design systems cannot generate design artifacts".to_string());
    }
    let schema_version = resolve_schema_version(state, &system, None)
        .await?
        .ok_or_else(|| "Design system has no published schema to generate from".to_string())?;
    let schema = read_design_file_artifact_json(
        state,
        storage_paths,
        &system,
        &schema_version.schema_artifact_id,
        ArtifactType::Specification,
    )
    .await?;
    let styleguide_items = state
        .design_styleguide_repo
        .list_items(&design_system_id, Some(&schema_version.id))
        .await
        .map_err(|error| error.to_string())?;
    let source_item =
        select_source_styleguide_item(&styleguide_items, artifact_kind, source_item_id.as_deref())?;

    let now = Utc::now();
    let mut run = DesignRun::queued(
        design_system_id.clone(),
        artifact_kind.run_kind(),
        format!(
            "Generate {} artifact: {}",
            artifact_kind.as_str(),
            name.as_str()
        ),
    );
    run.status = DesignRunStatus::Running;
    run.started_at = Some(now);
    let mut run = state
        .design_run_repo
        .create(run)
        .await
        .map_err(|error| error.to_string())?;

    let artifact_content = build_generated_artifact_json(
        &system,
        &schema_version,
        &schema,
        source_item,
        artifact_kind,
        &name,
        brief.as_deref(),
        now,
    );
    let storage_root = storage_paths
        .ensure_design_system_root(&system.storage_root_ref)
        .map_err(|error| error.to_string())?;
    let version_component = storage_paths.schema_version_component(&schema_version.id);
    let generated_component = storage_paths.styleguide_item_component(&format!(
        "{}:{}:{}",
        artifact_kind.as_str(),
        run.id.as_str(),
        name.as_str()
    ));
    let artifact_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("generated")
                .join(&version_component)
                .join(artifact_kind.as_str())
                .join(format!("{generated_component}.json")),
            &artifact_content,
        )
        .map_err(|error| error.to_string())?;
    let artifact_id = create_design_file_artifact(
        state,
        format!("Design {}: {}", artifact_kind.as_str(), name.as_str()),
        ArtifactType::DesignDoc,
        &artifact_path,
        Some(ArtifactId::from_string(
            schema_version.schema_artifact_id.clone(),
        )),
    )
    .await?;

    let preview_content = build_generated_artifact_preview_json(
        &system,
        &schema_version,
        artifact_kind,
        &name,
        &artifact_id,
        source_item,
        now,
    );
    let preview_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("previews")
                .join("generated")
                .join(&version_component)
                .join(format!("{generated_component}.json")),
            &preview_content,
        )
        .map_err(|error| error.to_string())?;
    let preview_artifact_id = create_design_file_artifact(
        state,
        format!(
            "Design {} preview: {}",
            artifact_kind.as_str(),
            name.as_str()
        ),
        ArtifactType::DesignDoc,
        &preview_path,
        Some(ArtifactId::from_string(artifact_id.clone())),
    )
    .await?;

    run.status = DesignRunStatus::Completed;
    run.completed_at = Some(now);
    run.output_artifact_ids = vec![artifact_id.clone(), preview_artifact_id.clone()];
    state
        .design_run_repo
        .update(&run)
        .await
        .map_err(|error| error.to_string())?;

    Ok(GenerateDesignArtifactResponse {
        design_system_id: design_system_id.as_str().to_string(),
        schema_version_id: schema_version.id.as_str().to_string(),
        run_id: run.id.as_str().to_string(),
        artifact_id,
        preview_artifact_id,
        artifact_kind: artifact_kind.as_str().to_string(),
        name,
        created_at: now.to_rfc3339(),
        content: artifact_content,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedDesignArtifactKind {
    Screen,
    Component,
}

impl GeneratedDesignArtifactKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "screen" | "generate_screen" => Ok(Self::Screen),
            "component" | "generate_component" => Ok(Self::Component),
            other => Err(format!(
                "Invalid design artifact kind: {other}. Expected screen or component"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Screen => "screen",
            Self::Component => "component",
        }
    }

    fn run_kind(self) -> DesignRunKind {
        match self {
            Self::Screen => DesignRunKind::GenerateScreen,
            Self::Component => DesignRunKind::GenerateComponent,
        }
    }

    fn preferred_group(self) -> DesignStyleguideGroup {
        match self {
            Self::Screen => DesignStyleguideGroup::UiKit,
            Self::Component => DesignStyleguideGroup::Components,
        }
    }

    fn preview_kind(self) -> &'static str {
        match self {
            Self::Screen => "screen_artifact_preview",
            Self::Component => "component_artifact_preview",
        }
    }
}

fn select_source_styleguide_item<'a>(
    items: &'a [DesignStyleguideItem],
    kind: GeneratedDesignArtifactKind,
    source_item_id: Option<&str>,
) -> Result<&'a DesignStyleguideItem, String> {
    if let Some(source_item_id) = source_item_id {
        return items
            .iter()
            .find(|item| item.item_id == source_item_id)
            .ok_or_else(|| {
                format!(
                    "Design styleguide item not found for artifact generation: {source_item_id}"
                )
            });
    }

    items
        .iter()
        .find(|item| item.group == kind.preferred_group())
        .or_else(|| items.first())
        .ok_or_else(|| "Design system has no styleguide items to generate from".to_string())
}

fn build_generated_artifact_json(
    system: &DesignSystem,
    schema_version: &DesignSchemaVersion,
    schema: &JsonValue,
    source_item: &DesignStyleguideItem,
    kind: GeneratedDesignArtifactKind,
    name: &str,
    brief: Option<&str>,
    generated_at: chrono::DateTime<Utc>,
) -> JsonValue {
    json!({
        "design_system_id": system.id.as_str(),
        "schema_version_id": schema_version.id.as_str(),
        "schema_artifact_id": schema_version.schema_artifact_id.as_str(),
        "styleguide_artifact_id": schema_version.styleguide_artifact_id.as_str(),
        "kind": kind.as_str(),
        "name": name,
        "brief": brief,
        "generated_at": generated_at.to_rfc3339(),
        "source_item": {
            "item_id": source_item.item_id.as_str(),
            "group": enum_text(&source_item.group),
            "label": source_item.label.as_str(),
            "summary": source_item.summary.as_str(),
            "preview_artifact_id": source_item.preview_artifact_id.as_deref(),
            "confidence": enum_text(&source_item.confidence),
        },
        "source_refs": &source_item.source_refs,
        "schema_refs": {
            "component_count": schema.get("components").and_then(JsonValue::as_array).map(Vec::len).unwrap_or(0),
            "screen_pattern_count": schema.get("screen_patterns").and_then(JsonValue::as_array).map(Vec::len).unwrap_or(0),
            "token_groups": {
                "colors": schema.pointer("/tokens/colors").and_then(JsonValue::as_array).map(Vec::len).unwrap_or(0),
                "typography": schema.pointer("/tokens/typography").and_then(JsonValue::as_array).map(Vec::len).unwrap_or(0),
                "spacing": schema.pointer("/tokens/spacing").and_then(JsonValue::as_array).map(Vec::len).unwrap_or(0),
            }
        },
        "artifact": {
            "storage": "ralphx_owned",
            "project_write_status": "not_written",
            "review_status": "needs_review",
            "handoff_status": "not_started",
        },
        "spec": generated_artifact_spec(kind, name, brief, source_item),
    })
}

fn build_generated_artifact_preview_json(
    system: &DesignSystem,
    schema_version: &DesignSchemaVersion,
    kind: GeneratedDesignArtifactKind,
    name: &str,
    artifact_id: &str,
    source_item: &DesignStyleguideItem,
    generated_at: chrono::DateTime<Utc>,
) -> JsonValue {
    json!({
        "design_system_id": system.id.as_str(),
        "schema_version_id": schema_version.id.as_str(),
        "artifact_id": artifact_id,
        "item_id": source_item.item_id.as_str(),
        "group": enum_text(&source_item.group),
        "label": name,
        "summary": format!("{} artifact generated from {}", kind.as_str(), source_item.label),
        "preview_kind": kind.preview_kind(),
        "confidence": enum_text(&source_item.confidence),
        "source_refs": &source_item.source_refs,
        "generated_at": generated_at.to_rfc3339(),
    })
}

fn generated_artifact_spec(
    kind: GeneratedDesignArtifactKind,
    name: &str,
    brief: Option<&str>,
    source_item: &DesignStyleguideItem,
) -> JsonValue {
    match kind {
        GeneratedDesignArtifactKind::Screen => json!({
            "screen_name": name,
            "intent": brief.unwrap_or("Generate a screen aligned to the current design schema."),
            "layout_pattern": source_item.item_id.as_str(),
            "regions": [
                "primary content",
                "supporting controls",
                "reviewable state"
            ],
            "states": ["empty", "loading", "ready", "error"],
            "accessibility": [
                "Preserve keyboard navigation.",
                "Use semantic regions and visible focus states.",
                "Keep generated copy concise and source-aligned."
            ],
        }),
        GeneratedDesignArtifactKind::Component => json!({
            "component_name": name,
            "intent": brief.unwrap_or("Generate a component aligned to the current design schema."),
            "component_pattern": source_item.item_id.as_str(),
            "variants": ["primary", "secondary", "disabled"],
            "states": ["default", "hover", "focus", "loading"],
            "accessibility": [
                "Expose accessible names for interactive controls.",
                "Keep visible focus treatment aligned with the styleguide.",
                "Do not rely on color alone for state."
            ],
        }),
    }
}

fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
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
            return Err("Design package artifact must be backed by a file".to_string());
        }
    };
    let bytes = storage_paths
        .read_file_under_design_storage_root(&path)
        .map_err(|error| error.to_string())?;
    parse_design_package_bytes(&bytes, path.extension().and_then(|value| value.to_str()))
}

fn validate_user_selected_design_package_path(
    raw_path: &str,
    require_existing_file: bool,
) -> Result<PathBuf, String> {
    let raw_path = parse_required_string(raw_path, "package_path")?;
    let path = PathBuf::from(raw_path);
    if !path.is_absolute() {
        return Err("Design package paths must be absolute user-selected paths".to_string());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Design package paths cannot contain parent directory components".to_string());
    }
    let extension = path.extension().and_then(|value| value.to_str());
    let extension_is_supported = if require_existing_file {
        matches!(extension, Some("zip" | "json"))
    } else {
        extension == Some("zip")
    };
    if !extension_is_supported {
        return Err(if require_existing_file {
            "Design package path must use a .zip or legacy .json extension".to_string()
        } else {
            "Design package export path must use a .zip extension".to_string()
        });
    }

    if require_existing_file {
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("Failed to resolve design package path: {error}"))?;
        let metadata = std::fs::metadata(&canonical)
            .map_err(|error| format!("Failed to read design package metadata: {error}"))?;
        if !metadata.is_file() {
            return Err("Selected design package path is not a file".to_string());
        }
        return Ok(canonical);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| "Design package path must include a file name".to_string())?
        .to_owned();
    let parent = path
        .parent()
        .ok_or_else(|| "Design package path must include a parent directory".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("Failed to resolve design package destination: {error}"))?;
    let metadata = std::fs::metadata(&canonical_parent)
        .map_err(|error| format!("Failed to read design package destination metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Design package destination parent is not a directory".to_string());
    }
    Ok(canonical_parent.join(file_name))
}

fn write_user_selected_design_package(
    raw_path: &str,
    package: &JsonValue,
) -> Result<String, String> {
    let path = validate_user_selected_design_package_path(raw_path, false)?;
    let bytes = serialize_design_package_zip(package)?;
    // codeql[rust/path-injection]
    std::fs::write(&path, bytes)
        .map_err(|error| format!("Failed to write design package export: {error}"))?;
    Ok(path.to_string_lossy().to_string())
}

fn read_user_selected_design_package(path: &Path) -> Result<JsonValue, String> {
    // codeql[rust/path-injection]
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Failed to read design package import: {error}"))?;
    parse_design_package_bytes(&bytes, path.extension().and_then(|value| value.to_str()))
}

fn serialize_design_package_zip(package: &JsonValue) -> Result<Vec<u8>, String> {
    let schema = package
        .get("schema")
        .ok_or_else(|| "Design package is missing schema".to_string())?;
    let source_audit = package
        .get("source_audit")
        .ok_or_else(|| "Design package is missing source_audit".to_string())?;
    let styleguide = package
        .get("styleguide")
        .ok_or_else(|| "Design package is missing styleguide".to_string())?;
    let components = schema
        .get("components")
        .cloned()
        .unwrap_or_else(|| JsonValue::Array(Vec::new()));
    let screen_patterns = schema
        .get("screen_patterns")
        .cloned()
        .unwrap_or_else(|| JsonValue::Array(Vec::new()));

    write_store_zip(vec![
        (
            "manifest.json",
            design_package_json_bytes(&design_package_manifest(package), "manifest")?,
        ),
        (
            "schema/design-system.schema.json",
            design_package_json_bytes(schema, "schema")?,
        ),
        (
            "schema/component-patterns.json",
            design_package_json_bytes(&components, "component patterns")?,
        ),
        (
            "schema/screen-patterns.json",
            design_package_json_bytes(&screen_patterns, "screen patterns")?,
        ),
        (
            "styleguide/styleguide-view-model.json",
            design_package_json_bytes(styleguide, "styleguide")?,
        ),
        (
            "reports/source-audit.json",
            design_package_json_bytes(source_audit, "source audit")?,
        ),
    ])
}

fn design_package_manifest(package: &JsonValue) -> JsonValue {
    json!({
        "package_version": package
            .get("package_version")
            .and_then(JsonValue::as_str)
            .unwrap_or(DESIGN_PACKAGE_VERSION),
        "exported_at": package.get("exported_at").cloned().unwrap_or(JsonValue::Null),
        "redacted": package.get("redacted").and_then(JsonValue::as_bool).unwrap_or(true),
        "design_system": package.get("design_system").cloned().unwrap_or_else(|| json!({})),
        "contents": {
            "schema": "schema/design-system.schema.json",
            "component_patterns": "schema/component-patterns.json",
            "screen_patterns": "schema/screen-patterns.json",
            "styleguide": "styleguide/styleguide-view-model.json",
            "source_audit": "reports/source-audit.json",
        },
        "privacy": {
            "absolute_paths_redacted": package.get("redacted").and_then(JsonValue::as_bool).unwrap_or(true),
            "raw_source_snippets_included": false,
        },
    })
}

fn design_package_json_bytes(value: &JsonValue, label: &str) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Failed to serialize design package {label}: {error}"))
}

fn parse_design_package_bytes(bytes: &[u8], extension: Option<&str>) -> Result<JsonValue, String> {
    let trimmed = bytes
        .iter()
        .copied()
        .skip_while(|byte| byte.is_ascii_whitespace())
        .next();
    if extension == Some("json") || trimmed == Some(b'{') {
        return serde_json::from_slice(bytes)
            .map_err(|error| format!("Failed to parse design package: {error}"));
    }
    parse_design_package_zip(bytes)
}

fn parse_design_package_zip(bytes: &[u8]) -> Result<JsonValue, String> {
    let entries = read_store_zip_entries(bytes)?;
    let manifest = parse_design_package_zip_json(&entries, "manifest.json")?;
    let schema = parse_design_package_zip_json(&entries, "schema/design-system.schema.json")?;
    let source_audit = parse_design_package_zip_json(&entries, "reports/source-audit.json")?;
    let styleguide =
        parse_design_package_zip_json(&entries, "styleguide/styleguide-view-model.json")?;

    Ok(json!({
        "package_version": manifest
            .get("package_version")
            .and_then(JsonValue::as_str)
            .unwrap_or(DESIGN_PACKAGE_VERSION),
        "exported_at": manifest.get("exported_at").cloned().unwrap_or(JsonValue::Null),
        "redacted": manifest.get("redacted").and_then(JsonValue::as_bool).unwrap_or(true),
        "design_system": manifest.get("design_system").cloned().unwrap_or_else(|| json!({})),
        "schema": schema,
        "source_audit": source_audit,
        "styleguide": styleguide,
    }))
}

fn parse_design_package_zip_json(
    entries: &HashMap<String, Vec<u8>>,
    name: &str,
) -> Result<JsonValue, String> {
    let bytes = entries
        .get(name)
        .ok_or_else(|| format!("Design package zip is missing {name}"))?;
    serde_json::from_slice(bytes)
        .map_err(|error| format!("Failed to parse design package entry {name}: {error}"))
}

struct ZipCentralRecord {
    name: String,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    local_header_offset: u32,
}

fn write_store_zip(entries: Vec<(&'static str, Vec<u8>)>) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut central_records = Vec::new();

    for (name, bytes) in entries {
        validate_zip_entry_name(name)?;
        let local_header_offset = checked_zip_u32(output.len(), "zip local header offset")?;
        let size = checked_zip_u32(bytes.len(), "zip entry size")?;
        let name_bytes = name.as_bytes();
        let name_len = checked_zip_u16(name_bytes.len(), "zip entry name")?;
        let crc = crc32(&bytes);

        push_u32(&mut output, 0x0403_4b50);
        push_u16(&mut output, 20);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u32(&mut output, crc);
        push_u32(&mut output, size);
        push_u32(&mut output, size);
        push_u16(&mut output, name_len);
        push_u16(&mut output, 0);
        output.extend_from_slice(name_bytes);
        output.extend_from_slice(&bytes);

        central_records.push(ZipCentralRecord {
            name: name.to_string(),
            crc32: crc,
            compressed_size: size,
            uncompressed_size: size,
            local_header_offset,
        });
    }

    let central_directory_offset = checked_zip_u32(output.len(), "zip central directory offset")?;
    for record in &central_records {
        let name_bytes = record.name.as_bytes();
        let name_len = checked_zip_u16(name_bytes.len(), "zip central entry name")?;

        push_u32(&mut output, 0x0201_4b50);
        push_u16(&mut output, 20);
        push_u16(&mut output, 20);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u32(&mut output, record.crc32);
        push_u32(&mut output, record.compressed_size);
        push_u32(&mut output, record.uncompressed_size);
        push_u16(&mut output, name_len);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u32(&mut output, 0);
        push_u32(&mut output, record.local_header_offset);
        output.extend_from_slice(name_bytes);
    }
    let central_directory_size = checked_zip_u32(
        output.len() - central_directory_offset as usize,
        "zip central directory size",
    )?;
    let entry_count = checked_zip_u16(central_records.len(), "zip entry count")?;

    push_u32(&mut output, 0x0605_4b50);
    push_u16(&mut output, 0);
    push_u16(&mut output, 0);
    push_u16(&mut output, entry_count);
    push_u16(&mut output, entry_count);
    push_u32(&mut output, central_directory_size);
    push_u32(&mut output, central_directory_offset);
    push_u16(&mut output, 0);

    Ok(output)
}

fn read_store_zip_entries(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>, String> {
    let eocd_offset = find_zip_eocd(bytes)?;
    let entry_count = read_u16(bytes, eocd_offset + 10)? as usize;
    let central_directory_size = read_u32(bytes, eocd_offset + 12)? as usize;
    let central_directory_offset = read_u32(bytes, eocd_offset + 16)? as usize;
    ensure_zip_range(
        bytes,
        central_directory_offset,
        central_directory_size,
        "zip central directory",
    )?;

    let mut entries = HashMap::new();
    let mut offset = central_directory_offset;
    for _ in 0..entry_count {
        ensure_zip_signature(bytes, offset, 0x0201_4b50, "zip central directory entry")?;
        let compression_method = read_u16(bytes, offset + 10)?;
        let crc = read_u32(bytes, offset + 16)?;
        let compressed_size = read_u32(bytes, offset + 20)? as usize;
        let uncompressed_size = read_u32(bytes, offset + 24)? as usize;
        let name_len = read_u16(bytes, offset + 28)? as usize;
        let extra_len = read_u16(bytes, offset + 30)? as usize;
        let comment_len = read_u16(bytes, offset + 32)? as usize;
        let local_header_offset = read_u32(bytes, offset + 42)? as usize;
        let name_start = offset + 46;
        let name_end = name_start + name_len;
        ensure_zip_range(bytes, name_start, name_len, "zip entry name")?;
        let name = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|error| format!("Design package zip has invalid entry name: {error}"))?
            .to_string();
        let is_directory = name.ends_with('/');
        let name_for_validation = if is_directory {
            name.trim_end_matches('/')
        } else {
            name.as_str()
        };
        validate_zip_entry_name(name_for_validation)?;
        let record_end = name_end
            .checked_add(extra_len)
            .and_then(|value| value.checked_add(comment_len))
            .ok_or_else(|| "Design package zip central directory entry overflowed".to_string())?;
        ensure_zip_range(
            bytes,
            offset,
            record_end - offset,
            "zip central directory entry",
        )?;
        offset = record_end;

        if is_directory {
            continue;
        }
        if compression_method != 0 {
            return Err(format!(
                "Design package zip entry {name} uses unsupported compression method {compression_method}"
            ));
        }
        let entry = read_store_zip_entry(
            bytes,
            &name,
            local_header_offset,
            compressed_size,
            uncompressed_size,
            crc,
        )?;
        entries.insert(name, entry);
    }

    Ok(entries)
}

fn read_store_zip_entry(
    bytes: &[u8],
    name: &str,
    local_header_offset: usize,
    compressed_size: usize,
    uncompressed_size: usize,
    expected_crc: u32,
) -> Result<Vec<u8>, String> {
    ensure_zip_signature(
        bytes,
        local_header_offset,
        0x0403_4b50,
        "zip local file header",
    )?;
    let name_len = read_u16(bytes, local_header_offset + 26)? as usize;
    let extra_len = read_u16(bytes, local_header_offset + 28)? as usize;
    let data_start = local_header_offset
        .checked_add(30)
        .and_then(|value| value.checked_add(name_len))
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| "Design package zip local file header overflowed".to_string())?;
    ensure_zip_range(bytes, data_start, compressed_size, "zip entry data")?;
    if compressed_size != uncompressed_size {
        return Err(format!(
            "Design package zip entry {name} has mismatched stored sizes"
        ));
    }
    let data = bytes[data_start..data_start + compressed_size].to_vec();
    let actual_crc = crc32(&data);
    if actual_crc != expected_crc {
        return Err(format!(
            "Design package zip entry {name} failed checksum validation"
        ));
    }
    Ok(data)
}

fn find_zip_eocd(bytes: &[u8]) -> Result<usize, String> {
    let min_eocd_len = 22;
    if bytes.len() < min_eocd_len {
        return Err("Design package zip is too small".to_string());
    }
    let search_start = bytes.len().saturating_sub(65_557);
    for offset in (search_start..=bytes.len() - min_eocd_len).rev() {
        if bytes[offset..].starts_with(&0x0605_4b50u32.to_le_bytes()) {
            return Ok(offset);
        }
    }
    Err("Design package zip is missing its central directory".to_string())
}

fn validate_zip_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || name
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(format!("Design package zip has unsafe entry name: {name}"));
    }
    Ok(())
}

fn ensure_zip_signature(
    bytes: &[u8],
    offset: usize,
    expected: u32,
    label: &str,
) -> Result<(), String> {
    if read_u32(bytes, offset)? == expected {
        Ok(())
    } else {
        Err(format!("Design package {label} has an invalid signature"))
    }
}

fn ensure_zip_range(bytes: &[u8], offset: usize, len: usize, label: &str) -> Result<(), String> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("Design package {label} range overflowed"))?;
    if end <= bytes.len() {
        Ok(())
    } else {
        Err(format!("Design package {label} is truncated"))
    }
}

fn checked_zip_u16(value: usize, label: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("Design package {label} is too large"))
}

fn checked_zip_u32(value: usize, label: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("Design package {label} is too large"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    ensure_zip_range(bytes, offset, 2, "field")?;
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    ensure_zip_range(bytes, offset, 4, "field")?;
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn package_import_source_label(
    artifact_id: Option<&ArtifactId>,
    package_path: Option<&str>,
) -> String {
    artifact_id
        .map(|id| format!("artifact {}", id.as_str()))
        .or_else(|| package_path.map(|path| format!("file {path}")))
        .unwrap_or_else(|| "package".to_string())
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
    use crate::commands::design_commands::{create_design_system_core, CreateDesignSystemInput};
    use crate::domain::entities::{Artifact, ArtifactContent, Project};
    use crate::http_server::handlers::design::{
        publish_design_schema_version_for_tool_core, PublishDesignSchemaVersionRequest,
        PublishDesignSourceRefInput, PublishDesignStyleguideItemInput,
    };

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
        let generated = publish_design_schema_version_for_tool_core(
            state,
            storage_paths,
            design_system_id.as_str(),
            PublishDesignSchemaVersionRequest {
                version: Some("0.1.0".to_string()),
                items: test_styleguide_publish_items(&project.id),
            },
        )
        .await
        .expect("publish styleguide");
        let preview_artifact_id = generated.items[0]
            .preview_artifact_id
            .clone()
            .expect("preview artifact id");

        (design_system_id, preview_artifact_id)
    }

    fn test_styleguide_publish_items(
        project_id: &ProjectId,
    ) -> Vec<PublishDesignStyleguideItemInput> {
        let source_refs = || {
            vec![PublishDesignSourceRefInput {
                project_id: project_id.as_str().to_string(),
                path: "frontend/src/components/ui/button.tsx".to_string(),
                line: Some(1),
            }]
        };
        vec![
            PublishDesignStyleguideItemInput {
                item_id: "ui_kit.core_workspace".to_string(),
                group: "ui_kit".to_string(),
                label: "Core workspace".to_string(),
                summary: "Source-grounded workspace layout".to_string(),
                source_refs: source_refs(),
                confidence: Some("high".to_string()),
            },
            PublishDesignStyleguideItemInput {
                item_id: "type.body".to_string(),
                group: "type".to_string(),
                label: "Body type".to_string(),
                summary: "Default readable application typography".to_string(),
                source_refs: source_refs(),
                confidence: Some("high".to_string()),
            },
            PublishDesignStyleguideItemInput {
                item_id: "colors.primary_palette".to_string(),
                group: "colors".to_string(),
                label: "Primary palette".to_string(),
                summary: "Primary action colors".to_string(),
                source_refs: source_refs(),
                confidence: Some("high".to_string()),
            },
            PublishDesignStyleguideItemInput {
                item_id: "spacing.scale".to_string(),
                group: "spacing".to_string(),
                label: "Spacing scale".to_string(),
                summary: "Compact app spacing".to_string(),
                source_refs: source_refs(),
                confidence: Some("medium".to_string()),
            },
            PublishDesignStyleguideItemInput {
                item_id: "components.core_controls".to_string(),
                group: "components".to_string(),
                label: "Core controls".to_string(),
                summary: "Buttons and app controls".to_string(),
                source_refs: source_refs(),
                confidence: Some("high".to_string()),
            },
            PublishDesignStyleguideItemInput {
                item_id: "brand.visual_identity".to_string(),
                group: "brand".to_string(),
                label: "Visual identity".to_string(),
                summary: "Logo and icon treatment".to_string(),
                source_refs: source_refs(),
                confidence: Some("medium".to_string()),
            },
        ]
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
    async fn export_package_writes_redacted_zip_artifact_inside_storage() {
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
                destination_path: None,
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
        assert_eq!(
            path.extension().and_then(|value| value.to_str()),
            Some("zip")
        );
        let package_bytes = std::fs::read(&path).expect("package zip");
        assert!(package_bytes.starts_with(b"PK"));
        let package_json =
            parse_design_package_bytes(&package_bytes, Some("zip")).expect("parse package zip");
        assert_eq!(package_json["redacted"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn generate_design_artifact_writes_schema_aligned_files_inside_storage() {
        let state = AppState::new_test();
        let temp = tempfile::tempdir().expect("tempdir");
        let storage_paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let (design_system_id, _) = create_generated_design_system(&state, &storage_paths).await;

        let response = generate_design_artifact_core(
            &state,
            &storage_paths,
            GenerateDesignArtifactInput {
                design_system_id: design_system_id.as_str().to_string(),
                artifact_kind: "component".to_string(),
                name: "Pricing cards".to_string(),
                brief: Some("Show plan comparison with clear CTAs".to_string()),
                source_item_id: Some("components.core_controls".to_string()),
            },
        )
        .await
        .expect("generate component artifact");

        assert_eq!(response.design_system_id, design_system_id.as_str());
        assert_eq!(response.artifact_kind, "component");
        assert_eq!(response.name, "Pricing cards");
        assert_eq!(
            response.content["artifact"]["storage"].as_str(),
            Some("ralphx_owned")
        );
        assert_eq!(
            response.content["artifact"]["project_write_status"].as_str(),
            Some("not_written")
        );
        assert_eq!(
            response.content["source_item"]["item_id"].as_str(),
            Some("components.core_controls")
        );
        assert!(response.content["source_refs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|source_ref| {
                source_ref
                    .get("path")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|path| !Path::new(path).is_absolute())
            }));

        let generated_artifact = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(response.artifact_id.clone()))
            .await
            .unwrap()
            .expect("generated artifact");
        assert_eq!(generated_artifact.artifact_type, ArtifactType::DesignDoc);
        let generated_path = match generated_artifact.content {
            ArtifactContent::File { path } => PathBuf::from(path),
            ArtifactContent::Inline { .. } => panic!("expected generated file artifact"),
        };
        assert!(generated_path.starts_with(temp.path().canonicalize().unwrap()));
        let generated_json: JsonValue =
            serde_json::from_str(&std::fs::read_to_string(generated_path).expect("generated json"))
                .expect("parse generated json");
        assert_eq!(generated_json["kind"].as_str(), Some("component"));

        let preview_artifact = state
            .artifact_repo
            .get_by_id(&ArtifactId::from_string(
                response.preview_artifact_id.clone(),
            ))
            .await
            .unwrap()
            .expect("preview artifact");
        assert_eq!(
            preview_artifact.derived_from,
            vec![ArtifactId::from_string(response.artifact_id.clone())]
        );
        let preview_path = match preview_artifact.content {
            ArtifactContent::File { path } => PathBuf::from(path),
            ArtifactContent::Inline { .. } => panic!("expected preview file artifact"),
        };
        assert!(preview_path.starts_with(temp.path().canonicalize().unwrap()));

        let runs = state
            .design_run_repo
            .list_by_design_system(&design_system_id)
            .await
            .unwrap();
        let generation_run = runs
            .iter()
            .find(|run| run.id.as_str() == response.run_id)
            .expect("generation run");
        assert_eq!(generation_run.kind, DesignRunKind::GenerateComponent);
        assert_eq!(generation_run.status, DesignRunStatus::Completed);
        assert_eq!(
            generation_run.output_artifact_ids,
            vec![response.artifact_id, response.preview_artifact_id]
        );
    }

    #[tokio::test]
    async fn import_package_creates_ready_design_system_from_export_file_path() {
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
        let export_dir = tempfile::tempdir().expect("export destination tempdir");
        let export_path = export_dir.path().join("design-system-package.zip");
        let export = export_design_system_package_core(
            &state,
            &storage_paths,
            ExportDesignSystemPackageInput {
                design_system_id: design_system_id.as_str().to_string(),
                include_full_provenance: None,
                destination_path: Some(export_path.to_string_lossy().to_string()),
            },
        )
        .await
        .expect("export package");
        assert_eq!(
            export.file_path.as_deref(),
            Some(export_path.to_str().unwrap())
        );
        assert!(export_path.exists());
        assert!(std::fs::read(&export_path)
            .expect("export zip")
            .starts_with(b"PK"));

        let imported = import_design_system_package_core(
            &state,
            &storage_paths,
            ImportDesignSystemPackageInput {
                package_artifact_id: None,
                package_path: export.file_path.clone(),
                attach_project_id: attach_project.id.as_str().to_string(),
                name: Some("Imported Product UI".to_string()),
            },
        )
        .await
        .expect("import package");

        assert!(imported.package_artifact_id.is_none());
        assert_eq!(
            imported.package_path.as_deref(),
            export.file_path.as_deref()
        );
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
        assert!(schema_artifact.derived_from.is_empty());
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
                package_artifact_id: Some(artifact_id),
                package_path: None,
                attach_project_id: attach_project.id.as_str().to_string(),
                name: None,
            },
        )
        .await
        .expect_err("outside package should fail");

        assert!(error.contains("escaped RalphX-owned design storage"));
    }
}
