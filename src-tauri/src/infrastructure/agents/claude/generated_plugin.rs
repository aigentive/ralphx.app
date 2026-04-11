use crate::infrastructure::agents::claude::plugin_repo_root;
use crate::infrastructure::agents::harness_agent_catalog::{
    load_canonical_agent_definition, load_harness_agent_prompt, resolve_project_root_from_plugin_dir,
    AgentPromptHarness, CanonicalAgentDefinition,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const GENERATED_PLUGIN_DIR_REL_DEBUG: &str = ".artifacts/generated/claude-plugin";
const GENERATED_PLUGIN_DIR_REL_PROD: &str = "generated/claude-plugin";

pub(crate) fn materialize_generated_plugin_dir(base_plugin_dir: &Path) -> Result<PathBuf, String> {
    let project_root = resolve_project_root_from_plugin_dir(base_plugin_dir);
    let generated_plugin_dir = generated_plugin_dir_for_base(base_plugin_dir);

    fs::create_dir_all(&generated_plugin_dir).map_err(|error| {
        format!(
            "Failed to create generated Claude plugin dir {}: {error}",
            generated_plugin_dir.display()
        )
    })?;

    sync_runtime_entries(base_plugin_dir, &generated_plugin_dir)?;
    sync_generated_agent_prompts(base_plugin_dir, &generated_plugin_dir, &project_root)?;

    Ok(generated_plugin_dir)
}

fn generated_plugin_dir_for_base(base_plugin_dir: &Path) -> PathBuf {
    let repo_root = plugin_repo_root(base_plugin_dir);
    if cfg!(debug_assertions) {
        repo_root.join(GENERATED_PLUGIN_DIR_REL_DEBUG)
    } else {
        repo_root.join(GENERATED_PLUGIN_DIR_REL_PROD)
    }
}

fn sync_runtime_entries(base_plugin_dir: &Path, generated_plugin_dir: &Path) -> Result<(), String> {
    for entry in fs::read_dir(base_plugin_dir).map_err(|error| {
        format!(
            "Failed to read base Claude plugin dir {}: {error}",
            base_plugin_dir.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "Failed to inspect entry under base Claude plugin dir {}: {error}",
                base_plugin_dir.display()
            )
        })?;
        let file_name = entry.file_name();
        if file_name == "agents" || file_name == ".DS_Store" {
            continue;
        }
        let target = generated_plugin_dir.join(&file_name);
        ensure_symlink(&entry.path(), &target)?;
    }
    Ok(())
}

fn sync_generated_agent_prompts(
    base_plugin_dir: &Path,
    generated_plugin_dir: &Path,
    project_root: &Path,
) -> Result<(), String> {
    let generated_agents_dir = generated_plugin_dir.join("agents");
    if generated_agents_dir.exists() {
        fs::remove_dir_all(&generated_agents_dir).map_err(|error| {
            format!(
                "Failed to clear generated Claude agents dir {}: {error}",
                generated_agents_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&generated_agents_dir).map_err(|error| {
        format!(
            "Failed to create generated Claude agents dir {}: {error}",
            generated_agents_dir.display()
        )
    })?;

    let mut reserved_outputs = HashSet::new();
    let canonical_agents_root = project_root.join("agents");
    if canonical_agents_root.exists() {
        for entry in fs::read_dir(&canonical_agents_root).map_err(|error| {
            format!(
                "Failed to read canonical agents dir {}: {error}",
                canonical_agents_root.display()
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to inspect canonical agent entry under {}: {error}",
                    canonical_agents_root.display()
                )
            })?;
            if !entry
                .file_type()
                .map_err(|error| {
                    format!(
                        "Failed to read canonical agent file type for {}: {error}",
                        entry.path().display()
                    )
                })?
                .is_dir()
            {
                continue;
            }

            let short_name = entry.file_name().to_string_lossy().to_string();
            let Some(definition) = load_canonical_agent_definition(project_root, &short_name) else {
                continue;
            };

            let relative_output = claude_output_relative_path(&definition, &short_name)?;
            reserved_outputs.insert(relative_output.clone());

            let Some(prompt_body) =
                load_harness_agent_prompt(project_root, &short_name, AgentPromptHarness::Claude)
            else {
                continue;
            };

            let legacy_source = base_plugin_dir.join(&relative_output);
            let frontmatter = load_frontmatter_block(&legacy_source)?;
            let generated_target = generated_plugin_dir.join(&relative_output);
            if let Some(parent) = generated_target.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "Failed to create generated Claude agent parent dir {}: {error}",
                        parent.display()
                    )
                })?;
            }
            let rendered = match frontmatter {
                Some(frontmatter) => format!("{frontmatter}\n\n{prompt_body}\n"),
                None => format!("{prompt_body}\n"),
            };
            fs::write(&generated_target, rendered).map_err(|error| {
                format!(
                    "Failed to write generated Claude agent prompt {}: {error}",
                    generated_target.display()
                )
            })?;
        }
    }

    let base_agents_dir = base_plugin_dir.join("agents");
    if base_agents_dir.exists() {
        for entry in fs::read_dir(&base_agents_dir).map_err(|error| {
            format!(
                "Failed to read base Claude agents dir {}: {error}",
                base_agents_dir.display()
            )
        })? {
            let entry = entry.map_err(|error| {
                format!(
                    "Failed to inspect base Claude agent entry under {}: {error}",
                    base_agents_dir.display()
                )
            })?;
            let source_path = entry.path();
            if !entry
                .file_type()
                .map_err(|error| {
                    format!(
                        "Failed to read base Claude agent file type for {}: {error}",
                        source_path.display()
                    )
                })?
                .is_file()
            {
                continue;
            }
            let relative_output = PathBuf::from("agents").join(entry.file_name());
            if reserved_outputs.contains(&relative_output) {
                continue;
            }
            let generated_target = generated_plugin_dir.join(&relative_output);
            ensure_symlink(&source_path, &generated_target)?;
        }
    }

    Ok(())
}

fn claude_output_relative_path(
    definition: &CanonicalAgentDefinition,
    short_name: &str,
) -> Result<PathBuf, String> {
    let configured = definition
        .claude_plugin_output
        .as_deref()
        .unwrap_or_else(|| short_name);
    let raw_path = PathBuf::from(configured);
    let relative = raw_path
        .strip_prefix("plugins/app")
        .or_else(|_| raw_path.strip_prefix("ralphx-plugin"))
        .unwrap_or(raw_path.as_path())
        .to_path_buf();

    if relative.is_absolute() {
        return Err(format!(
            "Canonical Claude plugin output for {} must be repo-relative: {}",
            short_name,
            relative.display()
        ));
    }

    if relative.components().next().is_none() {
        return Ok(PathBuf::from("agents").join(format!("{short_name}.md")));
    }

    Ok(relative)
}

fn load_frontmatter_block(path: &Path) -> Result<Option<String>, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Failed to read legacy Claude agent prompt {}: {error}",
                path.display()
            ))
        }
    };

    if let Some(after_first) = raw.strip_prefix("---") {
        if let Some(end_idx) = after_first.find("\n---") {
            let frontmatter_end = 3 + end_idx + "\n---".len();
            return Ok(Some(raw[..frontmatter_end].trim().to_string()));
        }
    }

    Ok(None)
}

fn ensure_symlink(source: &Path, target: &Path) -> Result<(), String> {
    if let Ok(existing) = fs::read_link(target) {
        if existing == source {
            return Ok(());
        }
    }

    remove_existing_path(target)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create generated Claude plugin parent dir {}: {error}",
                parent.display()
            )
        })?;
    }

    symlink_path(source, target)
}

fn remove_existing_path(path: &Path) -> Result<(), String> {
    if !path.exists() && fs::symlink_metadata(path).is_err() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "Failed to inspect existing generated Claude plugin path {}: {error}",
            path.display()
        )
    })?;

    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|error| {
            format!(
                "Failed to remove generated Claude plugin file {}: {error}",
                path.display()
            )
        })
    } else {
        fs::remove_dir_all(path).map_err(|error| {
            format!(
                "Failed to remove generated Claude plugin directory {}: {error}",
                path.display()
            )
        })
    }
}

#[cfg(unix)]
fn symlink_path(source: &Path, target: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(source, target).map_err(|error| {
        format!(
            "Failed to symlink generated Claude plugin path {} -> {}: {error}",
            target.display(),
            source.display()
        )
    })
}

#[cfg(windows)]
fn symlink_path(source: &Path, target: &Path) -> Result<(), String> {
    let result = if source.is_dir() {
        std::os::windows::fs::symlink_dir(source, target)
    } else {
        std::os::windows::fs::symlink_file(source, target)
    };
    result.map_err(|error| {
        format!(
            "Failed to symlink generated Claude plugin path {} -> {}: {error}",
            target.display(),
            source.display()
        )
    })
}
