use std::collections::BTreeMap;
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
            "radius": group_items_json(items, DesignStyleguideGroup::Spacing),
            "shadow_elevation": group_items_json(items, DesignStyleguideGroup::Spacing),
            "borders_rings_focus": group_items_json(items, DesignStyleguideGroup::Spacing),
            "motion": [],
        },
        "components": component_patterns_json(items),
        "screen_patterns": screen_patterns_json(items),
        "layout_patterns": layout_patterns_json(items),
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
        "hero_artifact": source_hints.hero_artifact,
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
    hero_artifact: Option<JsonValue>,
}

struct SourceSnapshot {
    path: String,
    label: String,
    content: String,
    asset_data_uri: Option<String>,
    asset_media_type: Option<String>,
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
    let css_variables = extract_css_variables(&snapshots);
    let component_samples = build_component_samples(item, &snapshots, &labels, &css_variables);
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
                "media_type": snapshot.asset_media_type,
                "uri": snapshot.asset_data_uri,
                "surface": asset_surface(&snapshot.path),
            })
        })
        .collect();
    let hero_artifact = build_hero_artifact_sample(item, &snapshots, &css_variables);

    PreviewSourceHints {
        paths,
        labels,
        swatches,
        typography_samples,
        component_samples,
        layout_regions,
        asset_samples,
        hero_artifact,
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
                asset_data_uri: None,
                asset_media_type: None,
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
    let bytes = std::fs::read(&file_path).ok()?;
    let content = String::from_utf8_lossy(&bytes).to_string();
    let asset_media_type = media_type_for_asset_path(&source_ref.path);
    let asset_data_uri = asset_media_type
        .as_deref()
        .map(|media_type| data_uri(media_type, &bytes));
    Some(SourceSnapshot {
        path: source_ref.path.clone(),
        label: source_label(&source_ref.path),
        content,
        asset_data_uri,
        asset_media_type,
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

fn build_component_samples(
    item: &DesignStyleguideItem,
    snapshots: &[SourceSnapshot],
    labels: &[String],
    css_variables: &BTreeMap<String, String>,
) -> Vec<JsonValue> {
    let mut samples = Vec::new();
    let item_text = format!("{} {}", item.label, item.summary).to_ascii_lowercase();

    if item_text.contains("button")
        || snapshots
            .iter()
            .any(|snapshot| snapshot.content.to_ascii_lowercase().contains(".button"))
    {
        for block in snapshots
            .iter()
            .flat_map(|snapshot| css_blocks(&snapshot.content))
        {
            let selector = block.selector.to_ascii_lowercase();
            if !selector.contains("button") {
                continue;
            }
            let styles = style_json_from_declarations(
                &css_declarations(block.body),
                css_variables,
                &[
                    "background",
                    "background-color",
                    "color",
                    "border",
                    "border-radius",
                    "box-shadow",
                    "min-height",
                    "height",
                    "padding",
                    "font-size",
                    "font-weight",
                    "letter-spacing",
                    "transform",
                ],
            );
            samples.push(json!({
                "kind": "button",
                "label": label_from_selector(block.selector).unwrap_or_else(|| "Button".to_string()),
                "selector": block.selector.trim(),
                "styles": styles,
                "states": ["default", "hover", "focus", "loading"],
            }));
            if samples.len() >= 6 {
                break;
            }
        }
    }

    if samples.is_empty()
        && (item_text.contains("hero artifact") || item_text.contains("hero sample"))
    {
        samples.push(json!({
            "kind": "hero_artifact",
            "label": item.label,
        }));
    }

    if samples.is_empty() {
        samples = labels
            .iter()
            .take(5)
            .map(|label| {
                json!({
                    "kind": "source_chip",
                    "label": label,
                })
            })
            .collect();
    }

    samples
}

fn build_hero_artifact_sample(
    item: &DesignStyleguideItem,
    snapshots: &[SourceSnapshot],
    css_variables: &BTreeMap<String, String>,
) -> Option<JsonValue> {
    let evidence = format!(
        "{} {} {}",
        item.label,
        item.summary,
        snapshots
            .iter()
            .map(|snapshot| snapshot.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    )
    .to_ascii_lowercase();
    if !evidence.contains("hero-sample")
        && !evidence.contains("hero artifact")
        && !evidence.contains("workflow block")
    {
        return None;
    }

    let blocks = snapshots
        .iter()
        .flat_map(|snapshot| css_blocks(&snapshot.content))
        .collect::<Vec<_>>();
    let panel_styles =
        style_json_from_matching_block(&blocks, css_variables, &["hero-sample", "hero sample"]);
    let trigger_styles =
        style_json_from_matching_block(&blocks, css_variables, &["trigger", "whatsapp", "message"]);
    let workflow_styles =
        style_json_from_matching_block(&blocks, css_variables, &["workflow", "artifact"]);

    Some(json!({
        "kind": "hero_artifact",
        "label": item.label,
        "summary": item.summary,
        "panel_styles": panel_styles,
        "trigger_styles": trigger_styles,
        "workflow_styles": workflow_styles,
        "agent_label": "Agent AI",
        "status_label": "Workflow running",
        "channel_label": "WhatsApp",
        "steps": [
            { "label": "Lead captured", "state": "done" },
            { "label": "CRM checked", "state": "done" },
            { "label": "Proposal drafted", "state": "pending" }
        ],
    }))
}

fn extract_css_variables(snapshots: &[SourceSnapshot]) -> BTreeMap<String, String> {
    let mut variables = BTreeMap::new();
    for declarations in snapshots
        .iter()
        .flat_map(|snapshot| css_blocks(&snapshot.content))
        .map(|block| css_declarations(block.body))
    {
        for (property, value) in declarations {
            if property.starts_with("--") {
                variables.entry(property).or_insert(value);
            }
        }
    }
    variables
}

struct CssBlock<'a> {
    selector: &'a str,
    body: &'a str,
}

fn css_blocks(content: &str) -> Vec<CssBlock<'_>> {
    let mut blocks = Vec::new();
    let mut search_start = 0;
    while let Some(open_offset) = content[search_start..].find('{') {
        let open = search_start + open_offset;
        let selector_start = content[..open]
            .rfind('}')
            .map(|index| index + 1)
            .unwrap_or(0);
        let selector = content[selector_start..open].trim();
        let Some(close_offset) = content[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + close_offset;
        let body = &content[open + 1..close];
        if !selector.is_empty() {
            blocks.push(CssBlock { selector, body });
        }
        search_start = close + 1;
    }
    blocks
}

fn css_declarations(body: &str) -> BTreeMap<String, String> {
    let mut declarations = BTreeMap::new();
    for declaration in body.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        if property.is_empty() || value.is_empty() {
            continue;
        }
        declarations.insert(property, value.to_string());
    }
    declarations
}

fn style_json_from_matching_block(
    blocks: &[CssBlock<'_>],
    css_variables: &BTreeMap<String, String>,
    needles: &[&str],
) -> JsonValue {
    let declarations = blocks
        .iter()
        .find(|block| {
            let selector = block.selector.to_ascii_lowercase();
            needles.iter().any(|needle| selector.contains(needle))
        })
        .map(|block| css_declarations(block.body))
        .unwrap_or_default();
    style_json_from_declarations(
        &declarations,
        css_variables,
        &[
            "background",
            "background-color",
            "border",
            "border-radius",
            "box-shadow",
            "padding",
            "min-height",
            "color",
        ],
    )
}

fn style_json_from_declarations(
    declarations: &BTreeMap<String, String>,
    css_variables: &BTreeMap<String, String>,
    allowed_properties: &[&str],
) -> JsonValue {
    let mut styles = serde_json::Map::new();
    for property in allowed_properties {
        let Some(value) = declarations.get(*property) else {
            continue;
        };
        styles.insert(
            (*property).to_string(),
            JsonValue::String(resolve_css_value(value, css_variables)),
        );
    }
    JsonValue::Object(styles)
}

fn resolve_css_value(value: &str, css_variables: &BTreeMap<String, String>) -> String {
    let mut output = value.trim().to_string();
    for _ in 0..4 {
        let Some(start) = output.find("var(") else {
            break;
        };
        let Some(relative_end) = output[start..].find(')') else {
            break;
        };
        let end = start + relative_end;
        let inner = &output[start + 4..end];
        let mut parts = inner.split(',').map(str::trim);
        let variable = parts.next().unwrap_or_default();
        let fallback = parts.next();
        let replacement = css_variables
            .get(variable)
            .map(String::as_str)
            .or(fallback)
            .unwrap_or(variable)
            .to_string();
        output.replace_range(start..=end, &replacement);
    }
    output
}

fn label_from_selector(selector: &str) -> Option<String> {
    let lower = selector.to_ascii_lowercase();
    let label = if lower.contains("primary") {
        "Primary"
    } else if lower.contains("secondary") {
        "Secondary"
    } else if lower.contains("accent") {
        "Accent"
    } else if lower.contains("nav") {
        "Nav"
    } else if lower.contains("ghost") {
        "Ghost"
    } else {
        "Base"
    };
    Some(label.to_string())
}

fn unique_limited<I>(values: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut output: Vec<String> = Vec::new();
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

fn media_type_for_asset_path(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    let media_type = if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else {
        return None;
    };
    Some(media_type.to_string())
}

fn data_uri(media_type: &str, bytes: &[u8]) -> String {
    format!("data:{media_type};base64,{}", base64_encode(bytes))
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        let triple = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;
        output.push(ALPHABET[((triple >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(triple & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn asset_surface(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.contains("logo.svg") && !lower.contains("black") {
        "dark"
    } else {
        "light"
    }
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

fn component_patterns_json(items: &[DesignStyleguideItem]) -> Vec<JsonValue> {
    items
        .iter()
        .filter(|item| item.group == DesignStyleguideGroup::Components)
        .map(|item| {
            json!({
                "id": item.item_id.as_str(),
                "kind": "component",
                "name": item.label.as_str(),
                "source_refs": &item.source_refs,
                "slots": [],
                "variants": [],
                "states": ["default", "hover", "focus", "disabled"],
                "tokens": {},
                "usage": {
                    "do": [item.summary.as_str()],
                    "avoid": [],
                },
                "confidence": enum_text(&item.confidence),
            })
        })
        .collect()
}

fn screen_patterns_json(items: &[DesignStyleguideItem]) -> Vec<JsonValue> {
    items
        .iter()
        .filter(|item| item.group == DesignStyleguideGroup::UiKit)
        .map(|item| {
            json!({
                "id": item.item_id.as_str(),
                "kind": "screen",
                "name": item.label.as_str(),
                "source_refs": &item.source_refs,
                "layout": "source_grounded_review_workspace",
                "regions": [],
                "density": "desktop_app_compact",
                "responsive_rules": [],
                "component_refs": [],
                "content_rules": [item.summary.as_str()],
                "confidence": enum_text(&item.confidence),
            })
        })
        .collect()
}

fn layout_patterns_json(items: &[DesignStyleguideItem]) -> Vec<JsonValue> {
    items
        .iter()
        .filter(|item| item.group == DesignStyleguideGroup::UiKit)
        .map(|item| {
            json!({
                "id": format!("layout.{}", item.item_id.as_str()),
                "kind": "layout",
                "name": item.label.as_str(),
                "source_refs": &item.source_refs,
                "summary": item.summary.as_str(),
                "confidence": enum_text(&item.confidence),
            })
        })
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
