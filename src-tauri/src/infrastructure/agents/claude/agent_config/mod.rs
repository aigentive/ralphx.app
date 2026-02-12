use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

const MEMORY_SKILLS: &[&str] = &[
    "Skill(ralphx:rule-manager)",
    "Skill(ralphx:knowledge-capture)",
];

const DEFAULT_BASE_CLI_TOOLS: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "Bash",
    "WebFetch",
    "WebSearch",
    "Skill",
];

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
    pub mcp_server_name: String,
    pub permission_mode: String,
    pub dangerously_skip_permissions: bool,
    pub permission_prompt_tool: String,
    pub use_append_system_prompt_file: bool,
    pub setting_sources: Option<Vec<String>>,
    /// JSON object passed to claude CLI via --settings (path or JSON string).
    pub settings: Option<serde_json::Value>,
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
    mcp_server_name: String,
    setting_sources: Option<Vec<String>>,
    permission_mode: String,
    dangerously_skip_permissions: bool,
    permission_prompt_tool: String,
    append_system_prompt_file: bool,
    /// Optional settings passed to claude CLI via --settings (see docs/claude-code/settings.md).
    settings: Option<serde_json::Value>,
}

impl Default for ClaudeRuntimeConfigRaw {
    fn default() -> Self {
        Self {
            mcp_server_name: "ralphx".to_string(),
            setting_sources: None,
            permission_mode: "default".to_string(),
            dangerously_skip_permissions: false,
            permission_prompt_tool: "permission_request".to_string(),
            append_system_prompt_file: true,
            settings: None,
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

fn normalize_mcp_tool_name(raw: &str, server_name: &str) -> String {
    if raw.starts_with("mcp__") {
        raw.to_string()
    } else {
        format!("mcp__{server_name}__{raw}")
    }
}

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

    let extends = raw.tools.extends.as_deref().unwrap_or("base_tools");

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

    let mcp_server_name = parsed.claude.mcp_server_name.clone();
    let claude = ClaudeRuntimeConfig {
        mcp_server_name,
        setting_sources: parsed.claude.setting_sources,
        permission_mode: parsed.claude.permission_mode,
        dangerously_skip_permissions: parsed.claude.dangerously_skip_permissions,
        permission_prompt_tool: normalize_mcp_tool_name(
            &parsed.claude.permission_prompt_tool,
            &parsed.claude.mcp_server_name,
        ),
        use_append_system_prompt_file: parsed.claude.append_system_prompt_file,
        settings: parsed.claude.settings,
    };

    Some(LoadedConfig {
        agents: resolved,
        claude,
    })
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
            mcp_server_name: "ralphx".to_string(),
            setting_sources: None,
            permission_mode: "default".to_string(),
            dangerously_skip_permissions: false,
            permission_prompt_tool: "mcp__ralphx__permission_request".to_string(),
            use_append_system_prompt_file: true,
            settings: None,
        },
    })
}

pub fn agent_configs() -> &'static [AgentConfig] {
    LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .agents
        .as_slice()
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
        let mcp_server = &claude_runtime_config().mcp_server_name;

        for t in &c.allowed_mcp_tools {
            tools.push(format!("mcp__{}__{}", mcp_server, t));
        }

        // CLI tools the agent can use (--tools) are also pre-approved so they don't prompt.
        if !c.mcp_only {
            tools.extend(c.resolved_cli_tools.iter().cloned());
        }
        tools.extend(c.preapproved_cli_tools.iter().cloned());

        if !c.mcp_only {
            // Memory skills only for dedicated memory agents
            let lookup_name = agent_name.strip_prefix("ralphx:").unwrap_or(agent_name);
            if lookup_name == "memory-maintainer" || lookup_name == "memory-capture" {
                for t in MEMORY_SKILLS {
                    tools.push((*t).to_string());
                }
            }
        }

        // Dedupe while preserving order (first occurrence wins)
        let mut seen = HashSet::new();
        tools.retain(|t| seen.insert(t.clone()));

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
        SHORT_MEMORY_CAPTURE, SHORT_MEMORY_MAINTAINER, SHORT_MERGER, SHORT_ORCHESTRATOR,
        SHORT_ORCHESTRATOR_IDEATION, SHORT_ORCHESTRATOR_IDEATION_READONLY,
        SHORT_PROJECT_ANALYZER, SHORT_QA_EXECUTOR, SHORT_QA_PREP, SHORT_REVIEW_CHAT,
        SHORT_REVIEW_HISTORY, SHORT_REVIEWER, SHORT_SESSION_NAMER, SHORT_SUPERVISOR,
        SHORT_WORKER,
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
        // Workers should NOT have memory skills - only dedicated memory agents
        assert!(!tools.contains("Skill(ralphx:rule-manager)"));
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
            SHORT_MEMORY_MAINTAINER,
            SHORT_MEMORY_CAPTURE,
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

    #[test]
    fn test_permission_prompt_tool_accepts_shorthand() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        assert_eq!(
            parsed.claude.permission_prompt_tool,
            "mcp__ralphx__permission_request"
        );
    }

    #[test]
    fn test_permission_prompt_tool_keeps_fully_qualified_name() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: mcp__external__permission_prompt
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        assert_eq!(
            parsed.claude.permission_prompt_tool,
            "mcp__external__permission_prompt"
        );
    }

    #[test]
    fn test_mcp_server_name_changes_shorthand_prefix() {
        let yaml = r#"
claude:
  mcp_server_name: acme
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        assert_eq!(parsed.claude.mcp_server_name, "acme");
        assert_eq!(
            parsed.claude.permission_prompt_tool,
            "mcp__acme__permission_request"
        );
    }

    #[test]
    fn test_memory_maintainer_has_memory_skills() {
        let tools = get_preapproved_tools("ralphx:memory-maintainer").unwrap();
        assert!(tools.contains("Skill(ralphx:rule-manager)"));
        assert!(tools.contains("Skill(ralphx:knowledge-capture)"));
    }

    #[test]
    fn test_memory_capture_has_memory_skills() {
        let tools = get_preapproved_tools("ralphx:memory-capture").unwrap();
        assert!(tools.contains("Skill(ralphx:rule-manager)"));
        assert!(tools.contains("Skill(ralphx:knowledge-capture)"));
    }

    #[test]
    fn test_non_memory_agents_lack_memory_skills() {
        let agents_to_test = vec![
            "ralphx-worker",
            "ralphx-reviewer",
            "ralphx-orchestrator",
            "ralphx-chat-task",
            "ralphx-chat-project",
        ];
        for agent_name in agents_to_test {
            if let Some(tools) = get_preapproved_tools(agent_name) {
                assert!(
                    !tools.contains("Skill(ralphx:rule-manager)"),
                    "Agent {} should not have rule-manager skill",
                    agent_name
                );
                assert!(
                    !tools.contains("Skill(ralphx:knowledge-capture)"),
                    "Agent {} should not have knowledge-capture skill",
                    agent_name
                );
            }
        }
    }

    #[test]
    fn test_non_memory_agents_lack_memory_write_mcp_tools() {
        // Memory write tools per spec section 11.2
        let memory_write_tools = vec![
            "upsert_memories",
            "mark_memory_obsolete",
            "refresh_memory_rule_index",
            "ingest_rule_file",
            "rebuild_archive_snapshots",
        ];

        let agents_to_test = vec![
            "ralphx-worker",
            "ralphx-reviewer",
            "ralphx-orchestrator",
            "ralphx-chat-task",
            "ralphx-chat-project",
        ];

        for agent_name in agents_to_test {
            if let Some(config) = get_agent_config(agent_name) {
                for write_tool in &memory_write_tools {
                    assert!(
                        !config.allowed_mcp_tools.contains(&write_tool.to_string()),
                        "Agent {} should not have write memory tool: {}",
                        agent_name,
                        write_tool
                    );
                }
            }
        }
    }

    #[test]
    fn test_memory_agents_have_write_mcp_tools() {
        // Memory maintainer should have write tools
        if let Some(config) = get_agent_config("memory-maintainer") {
            assert!(config.allowed_mcp_tools.contains(&"upsert_memories".to_string()));
            assert!(config.allowed_mcp_tools.contains(&"mark_memory_obsolete".to_string()));
            assert!(config.allowed_mcp_tools.contains(&"refresh_memory_rule_index".to_string()));
            assert!(config.allowed_mcp_tools.contains(&"ingest_rule_file".to_string()));
            assert!(config.allowed_mcp_tools.contains(&"rebuild_archive_snapshots".to_string()));
        }

        // Memory capture should have upsert_memories
        if let Some(config) = get_agent_config("memory-capture") {
            assert!(config.allowed_mcp_tools.contains(&"upsert_memories".to_string()));
        }
    }

    #[test]
    #[ignore = "memory read tools not yet added to worker/reviewer/orchestrator configs"]
    fn test_read_only_agents_have_read_memory_tools() {
        let read_memory_tools = vec![
            "search_memories",
            "get_memory",
            "get_memories_for_paths",
        ];

        let agents_to_test = vec![
            "ralphx-worker",
            "ralphx-reviewer",
            "ralphx-orchestrator",
        ];

        for agent_name in agents_to_test {
            if let Some(config) = get_agent_config(agent_name) {
                // Each of these should have at least one of the read memory tools
                let has_read_tool = read_memory_tools.iter().any(|t| {
                    config.allowed_mcp_tools.contains(&t.to_string())
                });
                assert!(
                    has_read_tool,
                    "Agent {} should have at least one read memory tool",
                    agent_name
                );
            }
        }
    }

    #[test]
    fn test_memory_maintainer_has_cli_write_tools() {
        // Memory maintainer must have Write and Edit to update rule files and archives
        if let Some(config) = get_agent_config("memory-maintainer") {
            assert!(
                config.preapproved_cli_tools.contains(&"Write".to_string()),
                "memory-maintainer must have Write tool"
            );
            assert!(
                config.preapproved_cli_tools.contains(&"Edit".to_string()),
                "memory-maintainer must have Edit tool"
            );
            assert!(
                config.preapproved_cli_tools.contains(&"Bash".to_string()),
                "memory-maintainer must have Bash tool for file operations"
            );
        }

        // Verify it's not MCP-only
        if let Some(config) = get_agent_config("memory-maintainer") {
            assert!(!config.mcp_only, "memory-maintainer should have CLI tools");
        }
    }

    #[test]
    fn test_memory_capture_has_read_cli_tools() {
        // Memory capture needs read tools to analyze conversations and extract memory
        if let Some(config) = get_agent_config("memory-capture") {
            assert!(
                config.preapproved_cli_tools.contains(&"Read".to_string()),
                "memory-capture must have Read tool"
            );
            assert!(
                config.preapproved_cli_tools.contains(&"Grep".to_string()),
                "memory-capture must have Grep tool"
            );
        }

        // Verify it's not MCP-only
        if let Some(config) = get_agent_config("memory-capture") {
            assert!(!config.mcp_only, "memory-capture should have CLI tools");
        }
    }
}
