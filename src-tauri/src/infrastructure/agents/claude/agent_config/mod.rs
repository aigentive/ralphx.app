use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

const DEFAULT_BASE_CLI_TOOLS: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "Bash",
    "WebFetch",
    "WebSearch",
    "Skill",
];

pub const MEMORY_SKILLS: &[&str] = &["Skill(ralphx:rule-manager)", "Skill(ralphx:knowledge-capture)"];

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub mcp_only: bool,
    pub resolved_cli_tools: Vec<String>,
    pub allowed_mcp_tools: Vec<String>,
    pub preapproved_cli_tools: Vec<String>,
    pub system_prompt_file: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClaudeRuntimeConfig {
    pub permission_mode: String,
    pub dangerously_skip_permissions: bool,
    pub permission_prompt_tool: String,
    pub use_append_system_prompt_file: bool,
    pub setting_sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AgentToolsSpec {
    #[serde(default)]
    mcp_only: bool,
    extends: Option<String>,
    #[serde(default)]
    include: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentConfigRaw {
    name: String,
    #[serde(default)]
    tools: AgentToolsSpec,
    #[serde(default)]
    mcp_tools: Vec<String>,
    #[serde(default)]
    preapproved_cli_tools: Vec<String>,
    system_prompt_file: String,
    model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct ClaudeRuntimeConfigRaw {
    setting_sources: Option<Vec<String>>,
    permission_mode: String,
    dangerously_skip_permissions: bool,
    permission_prompt_tool: String,
    append_system_prompt_file: bool,
}

impl Default for ClaudeRuntimeConfigRaw {
    fn default() -> Self {
        Self {
            setting_sources: None,
            permission_mode: "default".to_string(),
            dangerously_skip_permissions: false,
            permission_prompt_tool: "mcp__ralphx__permission_request".to_string(),
            append_system_prompt_file: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RalphxConfig {
    #[serde(default)]
    tool_sets: HashMap<String, Vec<String>>,
    #[serde(default)]
    claude: ClaudeRuntimeConfigRaw,
    agents: Vec<AgentConfigRaw>,
}

const EMBEDDED_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ralphx.yaml"));

struct LoadedConfig {
    agents: Vec<AgentConfig>,
    claude: ClaudeRuntimeConfig,
}

static LOADED_CONFIG_CELL: OnceLock<LoadedConfig> = OnceLock::new();

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("RALPHX_CONFIG_PATH") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    root.join("ralphx.yaml")
}

fn resolve_tools(raw: &AgentConfigRaw, tool_sets: &HashMap<String, Vec<String>>) -> Vec<String> {
    if raw.tools.mcp_only {
        return Vec::new();
    }

    let mut out = Vec::<String>::new();

    let extends = raw
        .tools
        .extends
        .as_deref()
        .unwrap_or("base_tools");

    if let Some(base) = tool_sets.get(extends) {
        out.extend(base.iter().cloned());
    } else if extends == "base_tools" {
        out.extend(DEFAULT_BASE_CLI_TOOLS.iter().map(|t| (*t).to_string()));
    } else {
        tracing::warn!(agent = %raw.name, tool_set = %extends, "Unknown tools.extends set; using include only");
    }

    out.extend(raw.tools.include.iter().cloned());

    // Stable de-dup while preserving first-seen order
    let mut seen = HashSet::new();
    out.retain(|t| seen.insert(t.clone()));
    out
}

fn parse_config(yaml: &str) -> Option<LoadedConfig> {
    let parsed: RalphxConfig = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse ralphx.yaml");
            return None;
        }
    };

    let mut seen_names = HashSet::new();
    let mut resolved = Vec::with_capacity(parsed.agents.len());

    for raw in &parsed.agents {
        if !seen_names.insert(raw.name.clone()) {
            tracing::warn!(agent = %raw.name, "Duplicate agent name in config");
            return None;
        }

        let cli_tools = resolve_tools(raw, &parsed.tool_sets);
        resolved.push(AgentConfig {
            name: raw.name.clone(),
            mcp_only: raw.tools.mcp_only,
            resolved_cli_tools: cli_tools,
            allowed_mcp_tools: raw.mcp_tools.clone(),
            preapproved_cli_tools: raw.preapproved_cli_tools.clone(),
            system_prompt_file: raw.system_prompt_file.clone(),
            model: raw.model.clone(),
        });
    }

    let claude = ClaudeRuntimeConfig {
        setting_sources: parsed.claude.setting_sources,
        permission_mode: parsed.claude.permission_mode,
        dangerously_skip_permissions: parsed.claude.dangerously_skip_permissions,
        permission_prompt_tool: parsed.claude.permission_prompt_tool,
        use_append_system_prompt_file: parsed.claude.append_system_prompt_file,
    };

    Some(LoadedConfig { agents: resolved, claude })
}

fn load_config() -> LoadedConfig {
    let path = config_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Some(cfg) = parse_config(&raw) {
            tracing::info!(
                path = %path.display(),
                agents = cfg.agents.len(),
                permission_mode = %cfg.claude.permission_mode,
                dangerously_skip_permissions = cfg.claude.dangerously_skip_permissions,
                append_system_prompt_file = cfg.claude.use_append_system_prompt_file,
                "Loaded agent config from ralphx.yaml"
            );
            return cfg;
        }
        tracing::warn!(path = %path.display(), "Falling back to embedded config due to parse error");
    } else {
        tracing::warn!(path = %path.display(), "ralphx.yaml not found/readable, using embedded config");
    }

    parse_config(EMBEDDED_CONFIG).unwrap_or_else(|| LoadedConfig {
        agents: Vec::new(),
        claude: ClaudeRuntimeConfig {
            setting_sources: None,
            permission_mode: "default".to_string(),
            dangerously_skip_permissions: false,
            permission_prompt_tool: "mcp__ralphx__permission_request".to_string(),
            use_append_system_prompt_file: true,
        },
    })
}

pub fn agent_configs() -> &'static [AgentConfig] {
    LOADED_CONFIG_CELL.get_or_init(load_config).agents.as_slice()
}

pub fn claude_runtime_config() -> &'static ClaudeRuntimeConfig {
    &LOADED_CONFIG_CELL.get_or_init(load_config).claude
}

pub fn get_agent_config(agent_name: &str) -> Option<&'static AgentConfig> {
    let lookup_name = agent_name.strip_prefix("ralphx:").unwrap_or(agent_name);
    agent_configs().iter().find(|c| c.name == lookup_name)
}

pub fn get_allowed_tools(agent_name: &str) -> Option<String> {
    get_agent_config(agent_name).map(|c| {
        if c.mcp_only {
            String::new()
        } else {
            c.resolved_cli_tools.join(",")
        }
    })
}

pub fn get_preapproved_tools(agent_name: &str) -> Option<String> {
    get_agent_config(agent_name).and_then(|c| {
        let mut tools: Vec<String> = Vec::new();

        for t in &c.allowed_mcp_tools {
            tools.push(format!("mcp__ralphx__{}", t));
        }

        tools.extend(c.preapproved_cli_tools.iter().cloned());

        if !c.mcp_only {
            for t in MEMORY_SKILLS {
                tools.push((*t).to_string());
            }
        }

        if tools.is_empty() {
            None
        } else {
            Some(tools.join(","))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::agents::claude::agent_names::{
        SHORT_CHAT_PROJECT, SHORT_CHAT_TASK, SHORT_DEEP_RESEARCHER, SHORT_DEPENDENCY_SUGGESTER,
        SHORT_MERGER, SHORT_ORCHESTRATOR, SHORT_ORCHESTRATOR_IDEATION,
        SHORT_ORCHESTRATOR_IDEATION_READONLY, SHORT_PROJECT_ANALYZER, SHORT_QA_EXECUTOR,
        SHORT_QA_PREP, SHORT_REVIEW_CHAT, SHORT_REVIEW_HISTORY, SHORT_REVIEWER,
        SHORT_SESSION_NAMER, SHORT_SUPERVISOR, SHORT_WORKER,
    };
    use std::collections::HashSet;

    #[test]
    fn test_yaml_loaded_has_unique_names() {
        let mut names: Vec<String> = agent_configs().iter().map(|c| c.name.clone()).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len);
    }

    #[test]
    fn test_get_allowed_tools_worker_agent() {
        let tools = get_allowed_tools("ralphx-worker").unwrap();
        assert!(tools.contains("Read"));
        assert!(tools.contains("Write"));
        assert!(tools.contains("Edit"));
        assert!(tools.contains("Task"));
    }

    #[test]
    fn test_get_allowed_tools_mcp_only_agent() {
        assert_eq!(get_allowed_tools("session-namer"), Some(String::new()));
    }

    #[test]
    fn test_get_preapproved_tools_worker_contains_expected() {
        let tools = get_preapproved_tools("ralphx-worker").unwrap();
        assert!(tools.contains("mcp__ralphx__get_task_context"));
        assert!(tools.contains("mcp__ralphx__get_project_analysis"));
        assert!(tools.contains("Write"));
        assert!(tools.contains("Task(Explore)"));
        assert!(tools.contains("Skill(ralphx:rule-manager)"));
    }

    #[test]
    fn test_default_base_tool_set_present_in_worker() {
        let tools = get_allowed_tools("ralphx-worker").unwrap();
        for t in DEFAULT_BASE_CLI_TOOLS {
            assert!(tools.contains(t), "worker missing base tool {}", t);
        }
    }

    #[test]
    fn test_all_agent_names_are_known() {
        let known: HashSet<&str> = HashSet::from([
            SHORT_ORCHESTRATOR_IDEATION,
            SHORT_ORCHESTRATOR_IDEATION_READONLY,
            SHORT_SESSION_NAMER,
            SHORT_DEPENDENCY_SUGGESTER,
            SHORT_CHAT_TASK,
            SHORT_CHAT_PROJECT,
            SHORT_REVIEW_CHAT,
            SHORT_REVIEW_HISTORY,
            SHORT_WORKER,
            SHORT_REVIEWER,
            SHORT_QA_PREP,
            SHORT_QA_EXECUTOR,
            SHORT_ORCHESTRATOR,
            SHORT_SUPERVISOR,
            SHORT_DEEP_RESEARCHER,
            SHORT_PROJECT_ANALYZER,
            SHORT_MERGER,
        ]);

        for agent in agent_configs() {
            assert!(
                known.contains(agent.name.as_str()),
                "Unknown agent name in ralphx.yaml: {}",
                agent.name
            );
        }
    }

    #[test]
    fn test_all_system_prompt_files_exist() {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

        for agent in agent_configs() {
            let prompt_path = project_root.join(&agent.system_prompt_file);
            assert!(
                prompt_path.exists(),
                "Missing system_prompt_file for {}: {}",
                agent.name,
                prompt_path.display()
            );
        }
    }
}
