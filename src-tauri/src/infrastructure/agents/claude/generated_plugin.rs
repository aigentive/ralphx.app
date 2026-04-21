use crate::infrastructure::agents::claude::plugin_repo_root;
use crate::infrastructure::agents::claude::{
    claude_runtime_config, find_base_plugin_dir, get_agent_config,
};
use crate::infrastructure::agents::harness_agent_catalog::{
    list_canonical_agent_names, load_canonical_agent_definition, load_harness_agent_prompt,
    resolve_project_root_from_plugin_dir, try_load_canonical_claude_metadata,
    AgentPromptHarness, CanonicalAgentDefinition, CanonicalClaudeAgentMetadata,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tracing::warn;

const GENERATED_PLUGIN_DIR_REL_DEBUG: &str = ".artifacts/generated/claude-plugin";
const GENERATED_PLUGIN_DIR_REL_PROD: &str = "generated/claude-plugin";
const GENERATED_PLUGIN_DIR_ENV: &str = "RALPHX_GENERATED_PLUGIN_DIR";
const INTERNAL_MCP_SERVER_DIR: &str = "ralphx-mcp-server";
const EXTERNAL_MCP_SERVER_DIR: &str = "ralphx-external-mcp";
const MCP_RUNTIME_ENTRY_REL: &str = "ralphx-mcp-server/build/index.js";
const MCP_RUNTIME_SDK_MARKER_REL: &str =
    "ralphx-mcp-server/node_modules/@modelcontextprotocol/sdk/package.json";
const FALLBACK_RUNTIME_ENTRY_NAMES: &[&str] = &[INTERNAL_MCP_SERVER_DIR, EXTERNAL_MCP_SERVER_DIR];

fn generated_plugin_materialization_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn generated_plugin_dir_cache() -> &'static Mutex<HashMap<PathBuf, PathBuf>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn generated_plugin_cache_key(base_plugin_dir: &Path) -> PathBuf {
    base_plugin_dir
        .canonicalize()
        .unwrap_or_else(|_| base_plugin_dir.to_path_buf())
}

fn cached_generated_plugin_dir(base_plugin_dir: &Path) -> Result<Option<PathBuf>, String> {
    let cache = generated_plugin_dir_cache()
        .lock()
        .map_err(|_| "Generated Claude plugin cache lock poisoned".to_string())?;
    Ok(cache
        .get(&generated_plugin_cache_key(base_plugin_dir))
        .filter(|generated_dir| generated_dir.exists())
        .cloned())
}

fn cache_generated_plugin_dir(
    base_plugin_dir: &Path,
    generated_plugin_dir: &Path,
) -> Result<(), String> {
    let mut cache = generated_plugin_dir_cache()
        .lock()
        .map_err(|_| "Generated Claude plugin cache lock poisoned".to_string())?;
    cache.insert(
        generated_plugin_cache_key(base_plugin_dir),
        generated_plugin_dir.to_path_buf(),
    );
    Ok(())
}

pub(crate) fn materialize_generated_plugin_dir(base_plugin_dir: &Path) -> Result<PathBuf, String> {
    materialize_generated_plugin_dir_with_runtime_source(
        base_plugin_dir,
        find_base_plugin_dir().as_deref(),
    )
}

pub(crate) fn materialize_generated_plugin_dir_with_runtime_source(
    base_plugin_dir: &Path,
    fallback_runtime_plugin_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    // Generated Claude assets are process-local runtime bootstrap outputs.
    // After the first successful materialization, keep reusing that directory
    // so later agent launches do not rewrite prompts under already-starting children.
    if let Some(cached_dir) = cached_generated_plugin_dir(base_plugin_dir)? {
        return Ok(cached_dir);
    }

    let _guard = generated_plugin_materialization_lock()
        .lock()
        .map_err(|_| "Generated Claude plugin materialization lock poisoned".to_string())?;
    if let Some(cached_dir) = cached_generated_plugin_dir(base_plugin_dir)? {
        return Ok(cached_dir);
    }

    let project_root = resolve_project_root_from_plugin_dir(base_plugin_dir);
    let generated_plugin_dir = generated_plugin_dir_for_base(base_plugin_dir);
    let runtime_source_plugin_dir =
        resolve_runtime_entries_source_plugin_dir(base_plugin_dir, fallback_runtime_plugin_dir);

    fs::create_dir_all(&generated_plugin_dir).map_err(|error| {
        format!(
            "Failed to create generated Claude plugin dir {}: {error}",
            generated_plugin_dir.display()
        )
    })?;

    sync_runtime_entries(
        base_plugin_dir,
        &runtime_source_plugin_dir,
        &generated_plugin_dir,
    )?;
    sync_generated_agent_prompts(base_plugin_dir, &generated_plugin_dir, &project_root)?;
    cache_generated_plugin_dir(base_plugin_dir, &generated_plugin_dir)?;

    Ok(generated_plugin_dir)
}

pub(crate) fn resolve_runtime_entries_source_plugin_dir(
    base_plugin_dir: &Path,
    fallback_runtime_plugin_dir: Option<&Path>,
) -> PathBuf {
    if plugin_dir_has_runnable_mcp_runtime(base_plugin_dir) {
        return base_plugin_dir.to_path_buf();
    }

    if let Some(fallback_runtime_plugin_dir) = fallback_runtime_plugin_dir.filter(|candidate| {
        *candidate != base_plugin_dir && plugin_dir_has_runnable_mcp_runtime(candidate)
    }) {
        warn!(
            base_plugin_dir = %base_plugin_dir.display(),
            fallback_runtime_plugin_dir = %fallback_runtime_plugin_dir.display(),
            "Local plugin dir is missing runnable MCP runtime dependencies; falling back to canonical runtime bundle"
        );
        return fallback_runtime_plugin_dir.to_path_buf();
    }

    base_plugin_dir.to_path_buf()
}

fn generated_plugin_dir_for_base(base_plugin_dir: &Path) -> PathBuf {
    let override_dir = std::env::var_os(GENERATED_PLUGIN_DIR_ENV).map(PathBuf::from);
    generated_plugin_dir_for_base_with_override(base_plugin_dir, override_dir.as_deref())
}

fn generated_plugin_dir_for_base_with_override(
    base_plugin_dir: &Path,
    override_dir: Option<&Path>,
) -> PathBuf {
    if let Some(override_dir) = override_dir {
        return override_dir.to_path_buf();
    }

    let repo_root = plugin_repo_root(base_plugin_dir);
    if cfg!(debug_assertions) {
        repo_root.join(GENERATED_PLUGIN_DIR_REL_DEBUG)
    } else {
        repo_root.join(GENERATED_PLUGIN_DIR_REL_PROD)
    }
}

fn sync_runtime_entries(
    base_plugin_dir: &Path,
    runtime_source_plugin_dir: &Path,
    generated_plugin_dir: &Path,
) -> Result<(), String> {
    let mut entry_names = BTreeSet::new();
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
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == "agents" || file_name == ".DS_Store" {
            continue;
        }
        entry_names.insert(file_name);
    }

    for runtime_entry_name in FALLBACK_RUNTIME_ENTRY_NAMES {
        if runtime_source_plugin_dir.join(runtime_entry_name).exists() {
            entry_names.insert((*runtime_entry_name).to_string());
        }
    }

    for file_name in entry_names {
        let target = generated_plugin_dir.join(&file_name);
        let preferred_runtime_source = runtime_source_plugin_dir.join(&file_name);
        let source = if FALLBACK_RUNTIME_ENTRY_NAMES.contains(&file_name.as_str())
            && preferred_runtime_source.exists()
        {
            preferred_runtime_source
        } else {
            base_plugin_dir.join(&file_name)
        };

        if !source.exists() {
            continue;
        }
        ensure_symlink(&source, &target)?;
    }
    Ok(())
}

fn plugin_dir_has_runnable_mcp_runtime(plugin_dir: &Path) -> bool {
    plugin_dir.join(MCP_RUNTIME_ENTRY_REL).is_file()
        && plugin_dir.join(MCP_RUNTIME_SDK_MARKER_REL).is_file()
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
    for short_name in list_canonical_agent_names(project_root) {
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
        let claude_metadata = try_load_canonical_claude_metadata(project_root, &short_name)?;

        let generated_target =
            trusted_generated_plugin_child_path(generated_plugin_dir, &relative_output)?;
        let rendered = render_generated_agent_markdown(
            &short_name,
            &definition,
            &claude_metadata,
            &prompt_body,
        )?;
        fs::write(&generated_target, rendered).map_err(|error| {
            format!(
                "Failed to write generated Claude agent prompt {}: {error}",
                generated_target.display()
            )
        })?;
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
            let generated_target =
                trusted_generated_plugin_child_path(generated_plugin_dir, &relative_output)?;
            ensure_symlink(&source_path, &generated_target)?;
        }
    }

    Ok(())
}

fn claude_output_relative_path(
    _definition: &CanonicalAgentDefinition,
    short_name: &str,
) -> Result<PathBuf, String> {
    Ok(PathBuf::from("agents").join(format!("{short_name}.md")))
}

fn relative_path_has_only_trusted_components(relative_path: &Path) -> bool {
    relative_path.components().all(|component| match component {
        std::path::Component::Normal(part) => {
            let part = part.to_string_lossy();
            !part.is_empty() && !part.contains("..")
        }
        _ => false,
    })
}

fn trusted_generated_plugin_child_path(
    generated_plugin_dir: &Path,
    relative_output: &Path,
) -> Result<PathBuf, String> {
    if !relative_path_has_only_trusted_components(relative_output) {
        return Err(format!(
            "Refusing generated Claude plugin output outside trusted relative path: {}",
            relative_output.display()
        ));
    }

    let generated_root = generated_plugin_dir.canonicalize().map_err(|error| {
        format!(
            "Failed to canonicalize generated Claude plugin dir {}: {error}",
            generated_plugin_dir.display()
        )
    })?;
    let target = generated_root.join(relative_output);
    let Some(parent) = target.parent() else {
        return Err(format!(
            "Generated Claude plugin output has no parent: {}",
            target.display()
        ));
    };

    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Failed to create generated Claude plugin output parent dir {}: {error}",
            parent.display()
        )
    })?;
    let canonical_parent = parent.canonicalize().map_err(|error| {
        format!(
            "Failed to canonicalize generated Claude plugin output parent dir {}: {error}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(&generated_root) {
        return Err(format!(
            "Refusing generated Claude plugin output outside {}: {}",
            generated_root.display(),
            canonical_parent.display()
        ));
    }

    let Some(file_name) = target.file_name() else {
        return Err(format!(
            "Generated Claude plugin output has no file name: {}",
            target.display()
        ));
    };
    Ok(canonical_parent.join(file_name))
}

fn render_generated_agent_markdown(
    agent_name: &str,
    definition: &CanonicalAgentDefinition,
    claude_metadata: &CanonicalClaudeAgentMetadata,
    prompt_body: &str,
) -> Result<String, String> {
    let frontmatter = build_claude_frontmatter(agent_name, definition, claude_metadata)?;
    Ok(format!("{frontmatter}\n\n{prompt_body}\n"))
}

fn build_claude_frontmatter(
    agent_name: &str,
    definition: &CanonicalAgentDefinition,
    claude_metadata: &CanonicalClaudeAgentMetadata,
) -> Result<String, String> {
    let agent_config = get_agent_config(agent_name).ok_or_else(|| {
        format!(
            "Canonical Claude agent {} is missing runtime config in config/ralphx.yaml",
            agent_name
        )
    })?;
    let mcp_server_name = &claude_runtime_config().mcp_server_name;
    let tools = build_claude_frontmatter_tools(agent_config, mcp_server_name);

    let mut lines = vec![
        "---".to_string(),
        format!("name: {}", yaml_scalar(&definition.name)?),
    ];

    if let Some(description) = definition.description.as_deref() {
        lines.push(format!("description: {}", yaml_scalar(description)?));
    }

    if !tools.is_empty() {
        lines.push("tools:".to_string());
        for tool in tools {
            lines.push(format!("  - {}", yaml_scalar(&tool)?));
        }
    }

    if !agent_config.allowed_mcp_tools.is_empty() {
        lines.push("mcpServers:".to_string());
        lines.push(format!("  - {}:", yaml_scalar(mcp_server_name)?));
        lines.push("      type: stdio".to_string());
        lines.push("      command: node".to_string());
        lines.push("      args:".to_string());
        lines.push(format!(
            "        - {}",
            yaml_scalar("${CLAUDE_PLUGIN_ROOT}/ralphx-mcp-server/build/index.js")?
        ));
        lines.push(format!("        - {}", yaml_scalar("--agent-type")?));
        lines.push(format!("        - {}", yaml_scalar(agent_name)?));
    }

    if !claude_metadata.disallowed_tools.is_empty() {
        lines.push("disallowedTools:".to_string());
        for tool in &claude_metadata.disallowed_tools {
            lines.push(format!("  - {}", yaml_scalar(tool)?));
        }
    }

    if let Some(model) = agent_config.model.as_deref() {
        lines.push(format!("model: {}", yaml_scalar(model)?));
    }

    if let Some(max_turns) = claude_metadata.max_turns {
        lines.push(format!("maxTurns: {max_turns}"));
    }

    if !claude_metadata.skills.is_empty() {
        lines.push("skills:".to_string());
        for skill in &claude_metadata.skills {
            lines.push(format!("  - {}", yaml_scalar(skill)?));
        }
    }

    lines.push("---".to_string());
    Ok(lines.join("\n"))
}

fn build_claude_frontmatter_tools(
    agent_config: &crate::infrastructure::agents::claude::AgentConfig,
    mcp_server_name: &str,
) -> Vec<String> {
    let mut tools = Vec::new();
    if !agent_config.mcp_only {
        tools.extend(agent_config.resolved_cli_tools.iter().cloned());
    }

    tools.extend(
        agent_config
            .allowed_mcp_tools
            .iter()
            .map(|tool| normalize_frontmatter_mcp_tool(tool, mcp_server_name)),
    );
    tools.extend(agent_config.preapproved_cli_tools.iter().cloned());

    let mut seen = HashSet::new();
    tools.retain(|tool| seen.insert(tool.clone()));
    tools
}

fn normalize_frontmatter_mcp_tool(tool: &str, mcp_server_name: &str) -> String {
    if tool.starts_with("mcp__") {
        tool.to_string()
    } else {
        format!("mcp__{mcp_server_name}__{tool}")
    }
}

fn yaml_scalar(value: &str) -> Result<String, String> {
    serde_yaml::to_string(value)
        .map(|rendered| rendered.trim().to_string())
        .map_err(|error| format!("Failed to render YAML scalar {value:?}: {error}"))
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

#[cfg(test)]
mod tests {
    use super::{
        generated_plugin_dir_for_base_with_override, trusted_generated_plugin_child_path,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn generated_plugin_dir_uses_override_when_present() {
        let override_dir = PathBuf::from("/tmp/custom-generated-plugin-dir");
        let resolved = generated_plugin_dir_for_base_with_override(
            Path::new("/tmp/ralphx/plugins/app"),
            Some(&override_dir),
        );

        assert_eq!(resolved, PathBuf::from(&override_dir));
    }

    #[test]
    fn trusted_generated_plugin_child_path_rejects_parent_traversal() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let err = trusted_generated_plugin_child_path(
            dir.path(),
            Path::new("agents").join("..").join("escape.md").as_path(),
        )
        .expect_err("parent traversal must be rejected");

        assert!(
            err.contains("Refusing generated Claude plugin output outside trusted relative path"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn trusted_generated_plugin_child_path_allows_normal_relative_output() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let target =
            trusted_generated_plugin_child_path(dir.path(), Path::new("agents/ralphx-test.md"))
                .expect("trusted relative path should be accepted");

        assert!(target.starts_with(dir.path().canonicalize().unwrap()));
        assert_eq!(target.file_name().and_then(|name| name.to_str()), Some("ralphx-test.md"));
    }
}
