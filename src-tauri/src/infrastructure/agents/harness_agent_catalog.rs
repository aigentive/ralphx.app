use serde::Deserialize;
use std::path::{Path, PathBuf};

const CANONICAL_AGENTS_DIR: &str = "agents";
const PROMPT_FILE_NAME: &str = "prompt.md";
const SHARED_FILE_NAME: &str = "shared.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CanonicalAgentDefinition {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub claude_plugin_output: Option<String>,
    #[serde(default)]
    pub migration_phase: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPromptHarness {
    Claude,
    Codex,
}

impl AgentPromptHarness {
    fn as_dir(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

fn canonical_agent_name(agent_name: &str) -> &str {
    agent_name
        .strip_prefix("ralphx:")
        .unwrap_or(agent_name)
}

fn canonical_agent_root(project_root: &Path, agent_name: &str) -> PathBuf {
    project_root
        .join(CANONICAL_AGENTS_DIR)
        .join(canonical_agent_name(agent_name))
}

pub fn resolve_project_root_from_plugin_dir(plugin_dir: &Path) -> PathBuf {
    let parent = plugin_dir.parent().unwrap_or(plugin_dir);
    if plugin_dir.ends_with(Path::new("plugins/app")) {
        parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| parent.to_path_buf())
    } else {
        parent.to_path_buf()
    }
}

pub fn load_canonical_agent_definition(
    project_root: &Path,
    agent_name: &str,
) -> Option<CanonicalAgentDefinition> {
    let short_name = canonical_agent_name(agent_name);
    let shared_path = canonical_agent_root(project_root, short_name).join(SHARED_FILE_NAME);
    let raw = std::fs::read_to_string(shared_path).ok()?;
    let definition = serde_yaml::from_str::<CanonicalAgentDefinition>(&raw).ok()?;
    if definition.name == short_name {
        Some(definition)
    } else {
        None
    }
}

pub fn resolve_harness_agent_prompt_path(
    project_root: &Path,
    agent_name: &str,
    harness: AgentPromptHarness,
) -> Option<PathBuf> {
    load_canonical_agent_definition(project_root, agent_name)?;
    let prompt_path = canonical_agent_root(project_root, agent_name)
        .join(harness.as_dir())
        .join(PROMPT_FILE_NAME);
    if prompt_path.exists() {
        Some(prompt_path)
    } else {
        None
    }
}

pub fn load_harness_agent_prompt(
    project_root: &Path,
    agent_name: &str,
    harness: AgentPromptHarness,
) -> Option<String> {
    let prompt_path = resolve_harness_agent_prompt_path(project_root, agent_name, harness)?;
    let raw = std::fs::read_to_string(prompt_path).ok()?;
    Some(raw.trim().to_string())
}

#[cfg(test)]
#[path = "harness_agent_catalog_tests.rs"]
mod tests;
