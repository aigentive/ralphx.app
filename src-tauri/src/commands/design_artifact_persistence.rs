use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};

use crate::application::AppState;
use crate::domain::entities::{
    Artifact, ArtifactType, DesignConfidence, DesignSchemaVersionId, DesignSourceRef,
    DesignStyleguideGroup, DesignStyleguideItem, DesignSystem, DesignSystemSource,
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
        let preview_value =
            build_preview_json(state, system, schema_version_id, item, generated_at).await;
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

async fn build_preview_json(
    state: &AppState,
    system: &DesignSystem,
    schema_version_id: &DesignSchemaVersionId,
    item: &DesignStyleguideItem,
    generated_at: DateTime<Utc>,
) -> JsonValue {
    let source_hints = build_preview_source_hints(state, item).await;
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
        "source_paths": source_hints.paths,
        "source_labels": source_hints.labels,
        "swatches": source_hints.swatches,
        "typography_samples": source_hints.typography_samples,
        "component_samples": source_hints.component_samples,
        "layout_regions": source_hints.layout_regions,
        "asset_samples": source_hints.asset_samples,
        "generated_at": generated_at.to_rfc3339(),
    })
}

#[derive(Default)]
struct PreviewSourceHints {
    paths: Vec<String>,
    labels: Vec<String>,
    swatches: Vec<JsonValue>,
    typography_samples: Vec<JsonValue>,
    component_samples: Vec<JsonValue>,
    layout_regions: Vec<JsonValue>,
    asset_samples: Vec<JsonValue>,
}

struct SourceSnapshot {
    path: String,
    label: String,
    content: String,
}

async fn build_preview_source_hints(
    state: &AppState,
    item: &DesignStyleguideItem,
) -> PreviewSourceHints {
    let snapshots = collect_source_snapshots(state, &item.source_refs).await;
    let paths = item
        .source_refs
        .iter()
        .map(|source_ref| source_ref.path.clone())
        .collect::<Vec<_>>();
    let labels = unique_limited(
        snapshots
            .iter()
            .map(|snapshot| snapshot.label.clone())
            .chain(paths.iter().map(|path| source_label(path))),
        6,
    );
    let swatches = extract_source_colors(&snapshots)
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            json!({
                "label": format!("Source {}", index + 1),
                "value": value,
            })
        })
        .collect();
    let font_samples = extract_source_fonts(&snapshots);
    let typography_samples = if font_samples.is_empty() {
        Vec::new()
    } else {
        font_samples
            .into_iter()
            .take(4)
            .enumerate()
            .map(|(index, font)| {
                json!({
                    "label": match index {
                        0 => "Display",
                        1 => "Body",
                        2 => "Label",
                        _ => "Code",
                    },
                    "sample": font,
                })
            })
            .collect()
    };
    let component_samples = labels
        .iter()
        .take(5)
        .map(|label| {
            json!({
                "label": label,
            })
        })
        .collect();
    let layout_regions = labels
        .iter()
        .take(3)
        .map(|label| {
            json!({
                "label": label,
            })
        })
        .collect();
    let asset_samples = snapshots
        .iter()
        .filter(|snapshot| is_asset_path(&snapshot.path))
        .take(6)
        .map(|snapshot| {
            json!({
                "label": snapshot.label,
                "path": snapshot.path,
            })
        })
        .collect();

    PreviewSourceHints {
        paths,
        labels,
        swatches,
        typography_samples,
        component_samples,
        layout_regions,
        asset_samples,
    }
}

async fn collect_source_snapshots(
    state: &AppState,
    source_refs: &[DesignSourceRef],
) -> Vec<SourceSnapshot> {
    let mut snapshots = Vec::new();
    for source_ref in source_refs.iter().take(8) {
        let Some(snapshot) = read_source_snapshot(state, source_ref).await else {
            snapshots.push(SourceSnapshot {
                path: source_ref.path.clone(),
                label: source_label(&source_ref.path),
                content: String::new(),
            });
            continue;
        };
        snapshots.push(snapshot);
    }
    snapshots
}

async fn read_source_snapshot(
    state: &AppState,
    source_ref: &DesignSourceRef,
) -> Option<SourceSnapshot> {
    let project = state
        .project_repo
        .get_by_id(&source_ref.project_id)
        .await
        .ok()
        .flatten()?;
    let root = Path::new(&project.working_directory).canonicalize().ok()?;
    let relative_path = safe_relative_path(&source_ref.path)?;
    let file_path = root.join(relative_path).canonicalize().ok()?;
    if !file_path.starts_with(&root) {
        return None;
    }
    let metadata = std::fs::metadata(&file_path).ok()?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 {
        return None;
    }
    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    Some(SourceSnapshot {
        path: source_ref.path.clone(),
        label: source_label(&source_ref.path),
        content,
    })
}

fn safe_relative_path(raw_path: &str) -> Option<PathBuf> {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return None;
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!safe.as_os_str().is_empty()).then_some(safe)
}

fn extract_source_colors(snapshots: &[SourceSnapshot]) -> Vec<String> {
    unique_limited(
        snapshots.iter().flat_map(|snapshot| {
            extract_hex_colors(&snapshot.content)
                .into_iter()
                .chain(extract_function_colors(&snapshot.content))
        }),
        6,
    )
}

fn extract_hex_colors(content: &str) -> Vec<String> {
    let chars = content.chars().collect::<Vec<_>>();
    let mut colors = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '#' {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < chars.len() && chars[index].is_ascii_hexdigit() {
            index += 1;
        }
        let len = index.saturating_sub(start + 1);
        if matches!(len, 3 | 4 | 6 | 8) {
            colors.push(chars[start..index].iter().collect());
        }
    }
    colors
}

fn extract_function_colors(content: &str) -> Vec<String> {
    let mut colors = Vec::new();
    for needle in ["rgb(", "rgba(", "hsl(", "hsla("] {
        let mut remainder = content;
        while let Some(start) = remainder.find(needle) {
            let after_start = &remainder[start..];
            let Some(end) = after_start.find(')') else {
                break;
            };
            colors.push(after_start[..=end].to_string());
            remainder = &after_start[end + 1..];
        }
    }
    colors
}

fn extract_source_fonts(snapshots: &[SourceSnapshot]) -> Vec<String> {
    unique_limited(
        snapshots
            .iter()
            .flat_map(|snapshot| snapshot.content.lines())
            .filter_map(font_value_from_line),
        4,
    )
}

fn font_value_from_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let start = lower.find("font-family")?;
    let after_key = &line[start..];
    let delimiter = after_key.find(|ch| ch == ':' || ch == '=')?;
    let value = after_key[delimiter + 1..]
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches(',');
    (!value.is_empty()).then(|| value.to_string())
}

fn unique_limited<I>(values: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut output = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() || output.iter().any(|existing| existing.as_str() == value) {
            continue;
        }
        output.push(value.to_string());
        if output.len() >= limit {
            break;
        }
    }
    output
}

fn source_label(path: &str) -> String {
    let file_stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path);
    let mut label = String::new();
    let mut previous_was_separator = true;
    for ch in file_stem.chars() {
        if matches!(ch, '-' | '_' | '.') {
            label.push(' ');
            previous_was_separator = true;
            continue;
        }
        if ch.is_uppercase() && !previous_was_separator && !label.ends_with(' ') {
            label.push(' ');
        }
        if previous_was_separator {
            label.extend(ch.to_uppercase());
        } else {
            label.push(ch);
        }
        previous_was_separator = false;
    }
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_asset_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    [".svg", ".png", ".jpg", ".jpeg", ".webp", ".gif", ".ico"]
        .iter()
        .any(|extension| lower.ends_with(extension))
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
