use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::infrastructure::agents::claude::canonical_short_agent_name;

const CANONICAL_AGENTS_DIR: &str = "agents";
const PROMPT_FILE_NAME: &str = "prompt.md";
const AGENT_FILE_NAME: &str = "agent.yaml";
const SHARED_PROMPT_DIR_NAME: &str = "shared";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalAgentDefinition {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CanonicalClaudeAgentMetadata {
    #[serde(default)]
    pub frontmatter_name: Option<String>,
    #[serde(default)]
    pub disallowed_tools: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub max_turns: Option<u32>,
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
    canonical_short_agent_name(agent_name)
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
    let agent_path = canonical_agent_root(project_root, short_name).join(AGENT_FILE_NAME);
    let raw = std::fs::read_to_string(agent_path).ok()?;
    let definition = serde_yaml::from_str::<CanonicalAgentDefinition>(&raw).ok()?;
    if definition.name == short_name {
        Some(definition)
    } else {
        None
    }
}

pub fn has_canonical_agent_definition(project_root: &Path, agent_name: &str) -> bool {
    canonical_agent_root(project_root, agent_name)
        .join(AGENT_FILE_NAME)
        .exists()
}

pub fn load_canonical_claude_metadata(
    project_root: &Path,
    agent_name: &str,
) -> CanonicalClaudeAgentMetadata {
    try_load_canonical_claude_metadata(project_root, agent_name).unwrap_or_default()
}

pub fn try_load_canonical_claude_metadata(
    project_root: &Path,
    agent_name: &str,
) -> Result<CanonicalClaudeAgentMetadata, String> {
    match load_harness_agent_metadata(project_root, agent_name, AgentPromptHarness::Claude) {
        Some(raw) => serde_yaml::from_str::<CanonicalClaudeAgentMetadata>(&raw).map_err(|error| {
            format!(
                "Failed to parse Claude harness metadata for agent {}: {error}",
                canonical_agent_name(agent_name)
            )
        }),
        None => Ok(CanonicalClaudeAgentMetadata::default()),
    }
}

pub fn resolve_harness_agent_prompt_path(
    project_root: &Path,
    agent_name: &str,
    harness: AgentPromptHarness,
) -> Option<PathBuf> {
    load_canonical_agent_definition(project_root, agent_name)?;
    let agent_root = canonical_agent_root(project_root, agent_name);
    let prompt_path = agent_root
        .join(harness.as_dir())
        .join(PROMPT_FILE_NAME);
    if prompt_path.exists() {
        return Some(prompt_path);
    }

    let shared_prompt_path = agent_root.join(SHARED_PROMPT_DIR_NAME).join(PROMPT_FILE_NAME);
    if shared_prompt_path.exists() {
        Some(shared_prompt_path)
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

fn load_harness_agent_metadata(
    project_root: &Path,
    agent_name: &str,
    harness: AgentPromptHarness,
) -> Option<String> {
    load_canonical_agent_definition(project_root, agent_name)?;
    let metadata_path = canonical_agent_root(project_root, agent_name)
        .join(harness.as_dir())
        .join(AGENT_FILE_NAME);
    std::fs::read_to_string(metadata_path).ok()
}

#[cfg(test)]
#[path = "harness_agent_catalog_tests.rs"]
mod tests;
