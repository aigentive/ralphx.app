use crate::infrastructure::agents::claude::{
    claude_runtime_config, find_base_plugin_dir, get_agent_config,
};
use crate::infrastructure::agents::harness_agent_catalog::{
    list_canonical_agent_names, load_canonical_agent_definition, load_harness_agent_prompt,
    resolve_project_root_from_plugin_dir, try_load_canonical_claude_metadata, AgentPromptHarness,
    CanonicalAgentDefinition, CanonicalClaudeAgentMetadata,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tracing::warn;

const GENERATED_PLUGIN_DIR_REL_DEBUG: &str = ".artifacts/generated/claude-plugin";
const GENERATED_PLUGIN_DIR_REL_PROD: &str = "generated/claude-plugin";
const INTERNAL_MCP_SERVER_DIR: &str = "ralphx-mcp-server";
const EXTERNAL_MCP_SERVER_DIR: &str = "ralphx-external-mcp";
const FALLBACK_RUNTIME_ENTRY_NAMES: &[&str] = &[INTERNAL_MCP_SERVER_DIR, EXTERNAL_MCP_SERVER_DIR];
const GENERATED_PLUGIN_ENTRY_NAMES: &[&str] = &[
    ".claude-plugin",
    ".mcp.json",
    "hooks",
    "memory-framework.md",
    "skills",
    INTERNAL_MCP_SERVER_DIR,
    EXTERNAL_MCP_SERVER_DIR,
];

fn generated_plugin_materialization_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn generated_plugin_dir_cache() -> &'static Mutex<HashMap<PathBuf, PathBuf>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn generated_plugin_dir_override() -> &'static Mutex<Option<PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

pub(super) fn replace_generated_plugin_dir_override(next: Option<PathBuf>) -> Option<PathBuf> {
    let mut guard = generated_plugin_dir_override()
        .lock()
        .expect("generated plugin dir override lock poisoned");
    std::mem::replace(&mut *guard, next)
}

fn configured_generated_plugin_dir() -> Option<PathBuf> {
    generated_plugin_dir_override()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
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
        .filter(|generated_dir| {
            // codeql[rust/path-injection]
            generated_dir.exists()
        })
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

    // codeql[rust/path-injection]
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
    if let Some(fallback_runtime_plugin_dir) =
        fallback_runtime_plugin_dir.filter(|candidate| *candidate != base_plugin_dir)
    {
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
    generated_plugin_dir_for_base_with_override(
        base_plugin_dir,
        configured_generated_plugin_dir().as_deref(),
    )
}

fn generated_plugin_dir_for_base_with_override(
    base_plugin_dir: &Path,
    override_dir: Option<&Path>,
) -> PathBuf {
    if let Some(override_dir) = override_dir {
        return override_dir.to_path_buf();
    }

    let repo_root = resolve_project_root_from_plugin_dir(base_plugin_dir);
    if cfg!(debug_assertions) {
        repo_root.join(GENERATED_PLUGIN_DIR_REL_DEBUG)
    } else {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("com.ralphx.app")
            .join(GENERATED_PLUGIN_DIR_REL_PROD)
    }
}

fn sync_runtime_entries(
    base_plugin_dir: &Path,
    runtime_source_plugin_dir: &Path,
    generated_plugin_dir: &Path,
) -> Result<(), String> {
    for file_name in GENERATED_PLUGIN_ENTRY_NAMES {
        let target = generated_plugin_dir.join(file_name);
        let preferred_runtime_source = runtime_source_plugin_dir.join(file_name);
        let source = if FALLBACK_RUNTIME_ENTRY_NAMES.contains(file_name)
            && runtime_source_plugin_dir != base_plugin_dir
            // codeql[rust/path-injection]
            && preferred_runtime_source.exists()
        {
            preferred_runtime_source
        } else {
            base_plugin_dir.join(file_name)
        };

        ensure_symlink(&source, &target)?;
    }
    Ok(())
}

fn sync_generated_agent_prompts(
    base_plugin_dir: &Path,
    generated_plugin_dir: &Path,
    project_root: &Path,
) -> Result<(), String> {
    let generated_agents_dir = generated_plugin_dir.join("agents");
    // codeql[rust/path-injection]
    if let Err(error) = fs::remove_dir_all(&generated_agents_dir) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(format!(
                "Failed to clear generated Claude agents dir {}: {error}",
                generated_agents_dir.display()
            ));
        }
    }
    // codeql[rust/path-injection]
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
        // codeql[rust/path-injection]
        fs::write(&generated_target, rendered).map_err(|error| {
            format!(
                "Failed to write generated Claude agent prompt {}: {error}",
                generated_target.display()
            )
        })?;
    }

    let base_agents_dir = base_plugin_dir.join("agents");
    // codeql[rust/path-injection]
    match fs::read_dir(&base_agents_dir) {
        Ok(entries) => {
            for entry in entries {
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to read base Claude agents dir {}: {error}",
                base_agents_dir.display()
            ));
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

    // codeql[rust/path-injection]
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
    let tools = build_claude_frontmatter_tools(agent_config, claude_metadata, mcp_server_name);
    let mcp_tools = resolved_claude_mcp_tools(agent_config, claude_metadata);

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

    if !mcp_tools.is_empty() {
        lines.push("mcpServers:".to_string());
        lines.push(format!("  - {}:", yaml_scalar(mcp_server_name)?));
        if claude_metadata.mcp_transport.as_deref() == Some("external") {
            lines.push("      type: http".to_string());
            lines.push(format!(
                "      url: {}",
                yaml_scalar("http://127.0.0.1:3848/mcp")?
            ));
            lines.push("      headers:".to_string());
            lines.push(format!(
                "        Authorization: {}",
                yaml_scalar("Bearer ${RALPHX_TAURI_MCP_BYPASS_TOKEN}")?
            ));
        } else {
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
    claude_metadata: &CanonicalClaudeAgentMetadata,
    mcp_server_name: &str,
) -> Vec<String> {
    let mut tools = Vec::new();
    if !agent_config.mcp_only {
        tools.extend(agent_config.resolved_cli_tools.iter().cloned());
    }

    tools.extend(
        resolved_claude_mcp_tools(agent_config, claude_metadata)
            .iter()
            .map(|tool| normalize_frontmatter_mcp_tool(tool, mcp_server_name)),
    );
    tools.extend(agent_config.preapproved_cli_tools.iter().cloned());

    let mut seen = HashSet::new();
    tools.retain(|tool| seen.insert(tool.clone()));
    tools
}

fn resolved_claude_mcp_tools<'a>(
    agent_config: &'a crate::infrastructure::agents::claude::AgentConfig,
    claude_metadata: &'a CanonicalClaudeAgentMetadata,
) -> &'a [String] {
    if claude_metadata.mcp_transport.as_deref() == Some("external")
        && !claude_metadata.mcp_tools.is_empty()
    {
        &claude_metadata.mcp_tools
    } else {
        &agent_config.allowed_mcp_tools
    }
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
    // codeql[rust/path-injection]
    match fs::symlink_metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to inspect generated Claude plugin source {}: {error}",
                source.display()
            ));
        }
    }

    // codeql[rust/path-injection]
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
    // codeql[rust/path-injection]
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to inspect existing generated Claude plugin path {}: {error}",
                path.display()
            ));
        }
    };

    if metadata.file_type().is_symlink() || metadata.is_file() {
        // codeql[rust/path-injection]
        fs::remove_file(path).map_err(|error| {
            format!(
                "Failed to remove generated Claude plugin file {}: {error}",
                path.display()
            )
        })
    } else {
        // codeql[rust/path-injection]
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
    // codeql[rust/path-injection]
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
    // codeql[rust/path-injection]
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
    use super::{generated_plugin_dir_for_base_with_override, trusted_generated_plugin_child_path};
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
        assert_eq!(
            target.file_name().and_then(|name| name.to_str()),
            Some("ralphx-test.md")
        );
    }
}
