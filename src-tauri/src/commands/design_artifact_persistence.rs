use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};

use crate::application::AppState;
use crate::domain::entities::{
    Artifact, ArtifactType, DesignConfidence, DesignSchemaVersionId, DesignStyleguideGroup,
    DesignStyleguideItem, DesignSystem, DesignSystemSource,
};
use crate::utils::design_storage_paths::DesignStoragePaths;

pub(crate) struct PersistedDesignArtifacts {
    pub(crate) schema_artifact_id: String,
    pub(crate) manifest_artifact_id: String,
    pub(crate) styleguide_artifact_id: String,
    pub(crate) output_artifact_ids: Vec<String>,
}

pub(crate) async fn persist_design_generation_artifacts(
    state: &AppState,
    storage_paths: &DesignStoragePaths,
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    version: &str,
    sources: &[DesignSystemSource],
    items: &mut [DesignStyleguideItem],
    generated_at: DateTime<Utc>,
) -> Result<PersistedDesignArtifacts, String> {
    let storage_root = storage_paths
        .ensure_design_system_root(&system.storage_root_ref)
        .map_err(|error| error.to_string())?;
    let version_component = storage_paths.schema_version_component(schema_version_id);
    let mut output_artifact_ids = Vec::new();

    for item in items.iter_mut() {
        let preview_value = build_preview_json(system, schema_version_id, item, generated_at);
        let item_component = storage_paths.styleguide_item_component(&item.item_id);
        let preview_path = storage_paths
            .write_json_file(
                &storage_root,
                PathBuf::from("previews")
                    .join(&version_component)
                    .join(format!("{item_component}.json")),
                &preview_value,
            )
            .map_err(|error| error.to_string())?;
        let preview_artifact_id = create_file_artifact(
            state,
            format!("Design preview: {}", item.label),
            ArtifactType::DesignDoc,
            &preview_path,
        )
        .await?;
        item.preview_artifact_id = Some(preview_artifact_id.clone());
        output_artifact_ids.push(preview_artifact_id);
    }

    let schema_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("schema")
                .join(&version_component)
                .join("design-system.schema.json"),
            &build_machine_schema_json(
                system,
                schema_version_id,
                version,
                sources,
                items,
                generated_at,
            ),
        )
        .map_err(|error| error.to_string())?;
    let schema_artifact_id = create_file_artifact(
        state,
        format!("{} design schema {}", system.name, version),
        ArtifactType::Specification,
        &schema_path,
    )
    .await?;

    let manifest_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("reports")
                .join(&version_component)
                .join("source-audit.json"),
            &build_source_audit_json(system, schema_version_id, sources, generated_at),
        )
        .map_err(|error| error.to_string())?;
    let manifest_artifact_id = create_file_artifact(
        state,
        format!("{} design source audit {}", system.name, version),
        ArtifactType::Findings,
        &manifest_path,
    )
    .await?;

    let styleguide_path = storage_paths
        .write_json_file(
            &storage_root,
            PathBuf::from("styleguide")
                .join(&version_component)
                .join("styleguide-view-model.json"),
            &build_styleguide_view_model_json(
                system,
                schema_version_id,
                version,
                items,
                generated_at,
            ),
        )
        .map_err(|error| error.to_string())?;
    let styleguide_artifact_id = create_file_artifact(
        state,
        format!("{} design styleguide {}", system.name, version),
        ArtifactType::DesignDoc,
        &styleguide_path,
    )
    .await?;

    output_artifact_ids.insert(0, styleguide_artifact_id.clone());
    output_artifact_ids.insert(0, manifest_artifact_id.clone());
    output_artifact_ids.insert(0, schema_artifact_id.clone());

    Ok(PersistedDesignArtifacts {
        schema_artifact_id,
        manifest_artifact_id,
        styleguide_artifact_id,
        output_artifact_ids,
    })
}

async fn create_file_artifact(
    state: &AppState,
    name: String,
    artifact_type: ArtifactType,
    path: &Path,
) -> Result<String, String> {
    let artifact = Artifact::new_file(
        name,
        artifact_type,
        path.to_string_lossy().to_string(),
        "ralphx-design",
    );
    state
        .artifact_repo
        .create(artifact)
        .await
        .map(|artifact| artifact.id.as_str().to_string())
        .map_err(|error| error.to_string())
}

fn build_machine_schema_json(
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    version: &str,
    sources: &[DesignSystemSource],
    items: &[DesignStyleguideItem],
    generated_at: DateTime<Utc>,
) -> JsonValue {
    json!({
        "schema_version": "1.0",
        "design_system": {
            "id": system.id.as_str(),
            "name": system.name.as_str(),
            "version": version,
            "created_at": generated_at.to_rfc3339(),
            "schema_version_id": schema_version_id.as_str(),
        },
        "sources": sources_json(sources),
        "brand": group_items_json(items, DesignStyleguideGroup::Brand),
        "tokens": {
            "colors": group_items_json(items, DesignStyleguideGroup::Colors),
            "typography": group_items_json(items, DesignStyleguideGroup::Type),
            "spacing": group_items_json(items, DesignStyleguideGroup::Spacing),
        },
        "components": group_items_json(items, DesignStyleguideGroup::Components),
        "screen_patterns": group_items_json(items, DesignStyleguideGroup::UiKit),
        "layout_patterns": group_items_json(items, DesignStyleguideGroup::UiKit),
        "content_voice": {},
        "assets": group_items_json(items, DesignStyleguideGroup::Brand),
        "accessibility": {
            "source_grounded": true
        },
        "usage_rules": [],
        "caveats": caveats_json(items),
        "provenance": {
            "generated_by": "ralphx-design",
            "generated_at": generated_at.to_rfc3339(),
            "source_ref_count": items.iter().map(|item| item.source_refs.len()).sum::<usize>(),
        }
    })
}

fn build_source_audit_json(
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    sources: &[DesignSystemSource],
    generated_at: DateTime<Utc>,
) -> JsonValue {
    json!({
        "design_system_id": system.id.as_str(),
        "schema_version_id": schema_version_id.as_str(),
        "generated_at": generated_at.to_rfc3339(),
        "sources": sources_json(sources),
        "total_file_count": sources.iter().map(|source| source.source_hashes.len()).sum::<usize>(),
        "caveats": [
            {
                "id": "initial-deterministic-analysis",
                "severity": "medium",
                "summary": "Initial styleguide rows are derived from deterministic source manifests until the live design steward runtime is enabled."
            }
        ]
    })
}

fn build_styleguide_view_model_json(
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    version: &str,
    items: &[DesignStyleguideItem],
    generated_at: DateTime<Utc>,
) -> JsonValue {
    let groups = [
        DesignStyleguideGroup::UiKit,
        DesignStyleguideGroup::Type,
        DesignStyleguideGroup::Colors,
        DesignStyleguideGroup::Spacing,
        DesignStyleguideGroup::Components,
        DesignStyleguideGroup::Brand,
    ]
    .into_iter()
    .filter_map(|group| {
        let group_items: Vec<_> = items
            .iter()
            .filter(|item| item.group == group)
            .map(styleguide_item_json)
            .collect();
        if group_items.is_empty() {
            None
        } else {
            Some(json!({
                "id": enum_text(&group),
                "label": styleguide_group_label(group),
                "items": group_items,
            }))
        }
    })
    .collect::<Vec<_>>();

    json!({
        "design_system_id": system.id.as_str(),
        "schema_version_id": schema_version_id.as_str(),
        "version": version,
        "generated_at": generated_at.to_rfc3339(),
        "ready_summary": format!("{} is ready for source-grounded styleguide review.", system.name),
        "caveats": caveats_json(items),
        "groups": groups,
    })
}

fn build_preview_json(
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    item: &DesignStyleguideItem,
    generated_at: DateTime<Utc>,
) -> JsonValue {
    json!({
        "design_system_id": system.id.as_str(),
        "schema_version_id": schema_version_id.as_str(),
        "item_id": item.item_id.as_str(),
        "group": enum_text(&item.group),
        "label": item.label.as_str(),
        "summary": item.summary.as_str(),
        "preview_kind": match item.group {
            DesignStyleguideGroup::Colors => "color_swatch",
            DesignStyleguideGroup::Type => "typography_sample",
            DesignStyleguideGroup::Spacing => "spacing_sample",
            DesignStyleguideGroup::Components => "component_sample",
            DesignStyleguideGroup::UiKit => "layout_sample",
            DesignStyleguideGroup::Brand => "asset_sample",
        },
        "confidence": enum_text(&item.confidence),
        "source_refs": &item.source_refs,
        "generated_at": generated_at.to_rfc3339(),
    })
}

fn sources_json(sources: &[DesignSystemSource]) -> Vec<JsonValue> {
    sources
        .iter()
        .map(|source| {
            json!({
                "id": source.id.as_str(),
                "project_id": source.project_id.as_str(),
                "role": enum_text(&source.role),
                "selected_paths": &source.selected_paths,
                "source_kind": enum_text(&source.source_kind),
                "git_commit": source.git_commit.as_deref(),
                "file_count": source.source_hashes.len(),
                "source_hashes": &source.source_hashes,
                "last_analyzed_at": source.last_analyzed_at.as_ref().map(|value| value.to_rfc3339()),
            })
        })
        .collect()
}

fn group_items_json(
    items: &[DesignStyleguideItem],
    group: DesignStyleguideGroup,
) -> Vec<JsonValue> {
    items
        .iter()
        .filter(|item| item.group == group)
        .map(styleguide_item_json)
        .collect()
}

fn styleguide_item_json(item: &DesignStyleguideItem) -> JsonValue {
    json!({
        "id": item.item_id.as_str(),
        "group": enum_text(&item.group),
        "label": item.label.as_str(),
        "summary": item.summary.as_str(),
        "preview_artifact_id": item.preview_artifact_id.as_deref(),
        "source_refs": &item.source_refs,
        "confidence": enum_text(&item.confidence),
        "approval_status": enum_text(&item.approval_status),
        "feedback_status": enum_text(&item.feedback_status),
        "updated_at": item.updated_at.to_rfc3339(),
    })
}

fn caveats_json(items: &[DesignStyleguideItem]) -> Vec<JsonValue> {
    items
        .iter()
        .filter(|item| item.confidence == DesignConfidence::Low)
        .map(|item| {
            json!({
                "item_id": item.item_id.as_str(),
                "severity": "medium",
                "title": format!("Source review needed: {}", item.label),
                "summary": "Only fallback source references matched this row; review before treating it as canonical.",
                "body": format!(
                    "{} used fallback source references because no direct source match was found in the selected paths. Review its sources before approving it.",
                    item.label
                ),
            })
        })
        .collect()
}

fn styleguide_group_label(group: DesignStyleguideGroup) -> &'static str {
    match group {
        DesignStyleguideGroup::UiKit => "UI kit",
        DesignStyleguideGroup::Type => "Type",
        DesignStyleguideGroup::Colors => "Colors",
        DesignStyleguideGroup::Spacing => "Spacing",
        DesignStyleguideGroup::Components => "Components",
        DesignStyleguideGroup::Brand => "Brand",
    }
}

fn enum_text<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}
