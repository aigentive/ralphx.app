use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use tauri::{Manager, State};

use crate::application::AppState;
use crate::domain::entities::{
    Artifact, ArtifactContent, ArtifactId, ArtifactType, DesignSchemaVersion,
    DesignSchemaVersionId, DesignStyleguideItem, DesignSystem, DesignSystemId,
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

fn parse_design_system_id(value: &str) -> Result<DesignSystemId, String> {
    let value = parse_required_string(value, "design_system_id")?;
    Ok(DesignSystemId::from_string(value))
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
}
