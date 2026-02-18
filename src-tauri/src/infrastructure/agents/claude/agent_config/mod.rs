pub mod team_config;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

#[allow(unused_imports)]
pub use team_config::{
    ApprovedTeamPlan, ApprovedTeammate, ProcessMapping, ProcessSlot, TeamConstraints,
    TeamConstraintError, TeamConstraintsConfig, TeamMode, TeammateSpawnRequest,
};

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
    /// Effective settings JSON for this agent (if any), resolved from settings_profile.
    pub settings: Option<serde_json::Value>,
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
    /// Parent agent name for config inheritance. Child fields override parent.
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    tools: AgentToolsSpec,
    #[serde(default)]
    mcp_tools: Vec<String>,
    #[serde(default)]
    preapproved_cli_tools: Vec<String>,
    #[serde(default)]
    system_prompt_file: Option<String>,
    model: Option<String>,
    settings_profile: Option<String>,
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
    /// Optional profile selector for claude settings (`settings_profiles.<name>`).
    settings_profile: Option<String>,
    /// Optional settings merged into every selected profile.
    settings_profile_defaults: Option<serde_json::Value>,
    /// Named claude settings profiles passed via --settings.
    #[serde(default)]
    settings_profiles: HashMap<String, serde_json::Value>,
    /// Optional settings passed to claude CLI via --settings (see docs/claude-code/settings.md).
    /// Legacy field kept for backwards compatibility when profiles are not configured.
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
            settings_profile: None,
            settings_profile_defaults: None,
            settings_profiles: HashMap::new(),
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
    #[serde(default)]
    agents: Vec<AgentConfigRaw>,
    #[serde(default)]
    process_mapping: ProcessMapping,
    #[serde(default)]
    team_constraints: TeamConstraintsConfig,
    /// If true (default), defers merges when conflicts exist or agents are running.
    /// If false, all merges proceed immediately without deferral.
    #[serde(default = "default_defer_merge_enabled")]
    defer_merge_enabled: bool,
}

const EMBEDDED_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ralphx.yaml"));

fn default_defer_merge_enabled() -> bool {
    true
}

struct LoadedConfig {
    agents: Vec<AgentConfig>,
    claude: ClaudeRuntimeConfig,
    process_mapping: ProcessMapping,
    team_constraints: TeamConstraintsConfig,
    defer_merge_enabled: bool,
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

// ── Agent config inheritance (extends) ──────────────────────────────────

/// Check if a tools spec has any explicit user-provided values.
fn tools_spec_is_default(spec: &AgentToolsSpec) -> bool {
    !spec.mcp_only && spec.extends.is_none() && spec.include.is_empty()
}

/// Recursively resolve agent inheritance via `extends` field.
///
/// Child fields override parent; missing/default fields fall through.
/// Circular extends detected via stack tracking.
fn resolve_agent_extends(
    raw: &AgentConfigRaw,
    all_agents: &[AgentConfigRaw],
    stack: &mut Vec<String>,
) -> AgentConfigRaw {
    let parent_name = match &raw.extends {
        Some(name) => name,
        None => return raw.clone(),
    };

    if stack.contains(parent_name) {
        tracing::warn!(
            agent = %raw.name,
            parent = %parent_name,
            chain = ?stack,
            "Circular agent extends detected"
        );
        return raw.clone();
    }

    stack.push(parent_name.clone());
    let parent = all_agents.iter().find(|a| a.name == *parent_name);
    let result = if let Some(parent) = parent {
        let resolved_parent = resolve_agent_extends(parent, all_agents, stack);
        merge_agent_configs(&resolved_parent, raw)
    } else {
        tracing::warn!(
            agent = %raw.name,
            parent = %parent_name,
            "Agent extends references unknown parent"
        );
        raw.clone()
    };
    stack.pop();
    result
}

/// Merge parent and child agent configs. Child fields override parent.
fn merge_agent_configs(parent: &AgentConfigRaw, child: &AgentConfigRaw) -> AgentConfigRaw {
    AgentConfigRaw {
        name: child.name.clone(),
        extends: None, // inheritance resolved
        system_prompt_file: child
            .system_prompt_file
            .clone()
            .or_else(|| parent.system_prompt_file.clone()),
        model: child.model.clone().or_else(|| parent.model.clone()),
        tools: if tools_spec_is_default(&child.tools) {
            parent.tools.clone()
        } else {
            child.tools.clone()
        },
        mcp_tools: if child.mcp_tools.is_empty() {
            parent.mcp_tools.clone()
        } else {
            child.mcp_tools.clone()
        },
        preapproved_cli_tools: if child.preapproved_cli_tools.is_empty() {
            parent.preapproved_cli_tools.clone()
        } else {
            child.preapproved_cli_tools.clone()
        },
        settings_profile: child
            .settings_profile
            .clone()
            .or_else(|| parent.settings_profile.clone()),
    }
}

fn parse_config(yaml: &str) -> Option<LoadedConfig> {
    let parsed: RalphxConfig = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse ralphx.yaml");
            return None;
        }
    };

    // Phase 1: resolve extends inheritance for all agents
    let resolved_raw_agents: Vec<AgentConfigRaw> = parsed
        .agents
        .iter()
        .map(|raw| {
            let mut stack = Vec::new();
            resolve_agent_extends(raw, &parsed.agents, &mut stack)
        })
        .collect();

    let mut seen_names = HashSet::new();
    let mut resolved = Vec::with_capacity(resolved_raw_agents.len());
    let global_profile_selection =
        runtime_settings_profile_override().or_else(|| parsed.claude.settings_profile.clone());
    let resolved_settings =
        resolve_claude_settings(&parsed.claude, global_profile_selection.as_deref());

    for raw in &resolved_raw_agents {
        if !seen_names.insert(raw.name.clone()) {
            tracing::warn!(agent = %raw.name, "Duplicate agent name in config");
            return None;
        }

        let system_prompt = match &raw.system_prompt_file {
            Some(path) => path.clone(),
            None => {
                tracing::warn!(
                    agent = %raw.name,
                    "Agent has no system_prompt_file (even after extends resolution)"
                );
                String::new()
            }
        };

        let cli_tools = resolve_tools(raw, &parsed.tool_sets);
        let agent_profile_selection =
            runtime_settings_profile_override_for_agent(&raw.name)
                .or_else(|| raw.settings_profile.clone());
        let agent_settings = if let Some(profile_name) = agent_profile_selection.as_deref() {
            if parsed.claude.settings_profiles.contains_key(profile_name) {
                resolve_claude_settings(&parsed.claude, Some(profile_name))
            } else {
                tracing::warn!(
                    agent = %raw.name,
                    profile = profile_name,
                    "Unknown agent settings_profile; falling back to global settings profile"
                );
                resolved_settings.clone()
            }
        } else {
            resolved_settings.clone()
        };
        resolved.push(AgentConfig {
            name: raw.name.clone(),
            mcp_only: raw.tools.mcp_only,
            resolved_cli_tools: cli_tools,
            allowed_mcp_tools: raw.mcp_tools.clone(),
            preapproved_cli_tools: raw.preapproved_cli_tools.clone(),
            system_prompt_file: system_prompt,
            model: raw.model.clone(),
            settings: agent_settings,
        });
    }

    let mcp_server_name = parsed.claude.mcp_server_name.clone();
    let resolved_settings =
        resolve_claude_settings(&parsed.claude, global_profile_selection.as_deref());
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
        settings: resolved_settings,
    };

    Some(LoadedConfig {
        agents: resolved,
        claude,
        process_mapping: parsed.process_mapping,
        team_constraints: parsed.team_constraints,
        defer_merge_enabled: parsed.defer_merge_enabled,
    })
}

fn runtime_settings_profile_override() -> Option<String> {
    runtime_settings_profile_override_with(&|name| std::env::var(name).ok())
}

fn runtime_settings_profile_override_with(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    lookup("RALPHX_CLAUDE_SETTINGS_PROFILE").and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn runtime_settings_profile_override_for_agent(agent_name: &str) -> Option<String> {
    runtime_settings_profile_override_for_agent_with(agent_name, &|name| std::env::var(name).ok())
}

fn runtime_settings_profile_override_for_agent_with(
    agent_name: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let normalized = normalize_agent_name_for_env(agent_name);
    let key = format!("RALPHX_CLAUDE_SETTINGS_PROFILE_{}", normalized);
    lookup(&key).and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_agent_name_for_env(agent_name: &str) -> String {
    let mut out = String::with_capacity(agent_name.len());
    for ch in agent_name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

fn resolve_claude_settings(
    raw: &ClaudeRuntimeConfigRaw,
    profile_selection: Option<&str>,
) -> Option<serde_json::Value> {
    let mut selected = if let Some(profile_name) = profile_selection {
        resolve_profile_settings(raw, profile_name)
    } else if raw.settings_profiles.contains_key("default") {
        resolve_profile_settings(raw, "default")
    } else {
        raw.settings.clone()
    };

    if let Some(defaults) = raw.settings_profile_defaults.clone() {
        selected = Some(match selected {
            Some(profile) => merge_settings(defaults, profile),
            None => defaults,
        });
    }

    if let Some(ref mut value) = selected {
        apply_prefixed_env_overrides(value);
        if value.as_object().is_some_and(|obj| obj.is_empty()) {
            return None;
        }
    }

    selected
}

fn resolve_profile_settings(
    raw: &ClaudeRuntimeConfigRaw,
    profile_name: &str,
) -> Option<serde_json::Value> {
    let mut stack = Vec::<String>::new();
    resolve_profile_settings_inner(raw, profile_name, &mut stack)
}

fn resolve_profile_settings_inner(
    raw: &ClaudeRuntimeConfigRaw,
    profile_name: &str,
    stack: &mut Vec<String>,
) -> Option<serde_json::Value> {
    if stack.iter().any(|v| v == profile_name) {
        tracing::warn!(
            profile = profile_name,
            chain = ?stack,
            "Cycle detected while resolving claude settings profile extends"
        );
        return None;
    }

    let profile = match raw.settings_profiles.get(profile_name) {
        Some(v) => v.clone(),
        None => {
            tracing::warn!(
                profile = profile_name,
                "Unknown claude.settings_profile; falling back to no custom settings"
            );
            return None;
        }
    };

    stack.push(profile_name.to_string());

    let mut merged = serde_json::json!({});
    let mut current_profile = profile;

    if let Some(current_obj) = current_profile.as_object_mut() {
        let extends_value = current_obj.remove("extends");
        if let Some(extends_list) = parse_extends_list(extends_value.as_ref(), profile_name) {
            for base_name in extends_list {
                if let Some(base) = resolve_profile_settings_inner(raw, &base_name, stack) {
                    merged = merge_settings(merged, base);
                }
            }
        }
    }

    stack.pop();
    Some(merge_settings(merged, current_profile))
}

fn parse_extends_list(
    extends_value: Option<&serde_json::Value>,
    profile_name: &str,
) -> Option<Vec<String>> {
    let value = extends_value?;
    match value {
        serde_json::Value::String(s) => Some(vec![s.clone()]),
        serde_json::Value::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(name) = item.as_str() {
                    out.push(name.to_string());
                } else {
                    tracing::warn!(
                        profile = profile_name,
                        invalid = ?item,
                        "Ignoring non-string entry in profile extends list"
                    );
                }
            }
            Some(out)
        }
        other => {
            tracing::warn!(
                profile = profile_name,
                invalid = ?other,
                "Ignoring invalid profile extends value; expected string or array"
            );
            None
        }
    }
}

fn merge_settings(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    match (base, overlay) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                let merged_value = if let Some(base_value) = base_map.remove(&key) {
                    merge_settings(base_value, overlay_value)
                } else {
                    overlay_value
                };
                base_map.insert(key, merged_value);
            }
            serde_json::Value::Object(base_map)
        }
        (_, overlay_value) => overlay_value,
    }
}

fn apply_prefixed_env_overrides(settings: &mut serde_json::Value) {
    apply_prefixed_env_overrides_with(settings, &|name| std::env::var(name).ok());
}

fn apply_prefixed_env_overrides_with(
    settings: &mut serde_json::Value,
    lookup: &dyn Fn(&str) -> Option<String>,
) {
    let Some(env_settings) = settings.get_mut("env").and_then(|v| v.as_object_mut()) else {
        return;
    };

    for (target_key, target_value) in env_settings.iter_mut() {
        let source_key = format!("RALPHX_{target_key}");
        if let Some(value) = lookup(&source_key) {
            *target_value = serde_json::Value::String(value);
        }
    }
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
        process_mapping: ProcessMapping::default(),
        team_constraints: TeamConstraintsConfig::default(),
        defer_merge_enabled: true,
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

pub fn get_effective_settings(agent_name: Option<&str>) -> Option<&'static serde_json::Value> {
    let loaded = LOADED_CONFIG_CELL.get_or_init(load_config);
    if let Some(name) = agent_name {
        let lookup_name = name.strip_prefix("ralphx:").unwrap_or(name);
        if let Some(agent) = loaded.agents.iter().find(|c| c.name == lookup_name) {
            return agent.settings.as_ref();
        }
    }
    loaded.claude.settings.as_ref()
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

pub fn process_mapping() -> &'static ProcessMapping {
    &LOADED_CONFIG_CELL.get_or_init(load_config).process_mapping
}

pub fn team_constraints_config() -> &'static TeamConstraintsConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .team_constraints
}

pub fn defer_merge_enabled() -> bool {
    LOADED_CONFIG_CELL.get_or_init(load_config).defer_merge_enabled
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
        SHORT_CHAT_PROJECT, SHORT_CHAT_TASK, SHORT_CODER, SHORT_DEEP_RESEARCHER,
        SHORT_DEPENDENCY_SUGGESTER, SHORT_IDEATION_TEAM_LEAD, SHORT_MEMORY_CAPTURE,
        SHORT_MEMORY_MAINTAINER, SHORT_MERGER, SHORT_ORCHESTRATOR, SHORT_ORCHESTRATOR_IDEATION,
        SHORT_ORCHESTRATOR_IDEATION_READONLY, SHORT_PROJECT_ANALYZER, SHORT_QA_EXECUTOR,
        SHORT_QA_PREP, SHORT_REVIEWER, SHORT_REVIEW_CHAT, SHORT_REVIEW_HISTORY,
        SHORT_SESSION_NAMER, SHORT_SUPERVISOR, SHORT_WORKER, SHORT_WORKER_TEAM,
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
            SHORT_CODER,
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
            // Team lead variants
            SHORT_IDEATION_TEAM_LEAD,
            SHORT_WORKER_TEAM,
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
    fn test_settings_profile_selection_uses_default_profile_payload() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: default
  settings_profiles:
    default:
      sandbox:
        enabled: false
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
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
            parsed.claude.settings,
            Some(serde_json::json!({
                "sandbox": { "enabled": false }
            }))
        );
    }

    #[test]
    fn test_settings_profile_resolves_prefixed_env_overrides() {
        let mut settings = serde_json::json!({
            "env": {
                "ANTHROPIC_DEFAULT_HAIKU_MODEL": "glm-4.5-air",
                "ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5",
                "ANTHROPIC_DEFAULT_OPUS_MODEL": "glm-5",
            }
        });

        apply_prefixed_env_overrides_with(&mut settings, &|name| match name {
            "RALPHX_ANTHROPIC_DEFAULT_HAIKU_MODEL" => Some("custom-haiku".to_string()),
            "RALPHX_ANTHROPIC_DEFAULT_SONNET_MODEL" => Some("custom-sonnet".to_string()),
            _ => None,
        });

        assert_eq!(
            settings
                .get("env")
                .and_then(|v| v.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
                .and_then(|v| v.as_str()),
            Some("custom-haiku")
        );
        assert_eq!(
            settings
                .get("env")
                .and_then(|v| v.get("ANTHROPIC_DEFAULT_SONNET_MODEL"))
                .and_then(|v| v.as_str()),
            Some("custom-sonnet")
        );
        assert_eq!(
            settings
                .get("env")
                .and_then(|v| v.get("ANTHROPIC_DEFAULT_OPUS_MODEL"))
                .and_then(|v| v.as_str()),
            Some("glm-5")
        );
    }

    #[test]
    fn test_agent_settings_profile_overrides_global_profile() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
    default:
      sandbox:
        enabled: false
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    settings_profile: default
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
  - name: ralphx-coder
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/coder.md
"#;
        let parsed = parse_config(yaml).expect("config should parse");

        assert!(
            parsed.claude.settings.is_some(),
            "global z_ai should be active"
        );

        let worker = parsed
            .agents
            .iter()
            .find(|a| a.name == "ralphx-worker")
            .expect("worker should exist");
        assert_eq!(
            worker.settings,
            Some(serde_json::json!({
                "sandbox": { "enabled": false }
            })),
            "worker should override to default profile"
        );

        let coder = parsed
            .agents
            .iter()
            .find(|a| a.name == "ralphx-coder")
            .expect("coder should exist");
        assert!(
            coder.settings.is_some(),
            "coder should inherit global z_ai profile"
        );
    }

    #[test]
    fn test_unknown_agent_settings_profile_falls_back_to_global() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    settings_profile: missing_profile
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let worker = parsed
            .agents
            .iter()
            .find(|a| a.name == "ralphx-worker")
            .expect("worker should exist");
        assert_eq!(
            worker.settings, parsed.claude.settings,
            "unknown agent profile should inherit global settings"
        );
    }

    #[test]
    fn test_runtime_settings_profile_override_reads_env_value() {
        let selection = runtime_settings_profile_override_with(&|name| match name {
            "RALPHX_CLAUDE_SETTINGS_PROFILE" => Some("z_ai".to_string()),
            _ => None,
        });
        assert_eq!(selection.as_deref(), Some("z_ai"));
    }

    #[test]
    fn test_runtime_settings_profile_override_ignores_blank_value() {
        let selection = runtime_settings_profile_override_with(&|name| match name {
            "RALPHX_CLAUDE_SETTINGS_PROFILE" => Some("   ".to_string()),
            _ => None,
        });
        assert_eq!(selection, None);
    }

    #[test]
    fn test_runtime_settings_profile_override_for_agent_uses_normalized_key() {
        let selection = runtime_settings_profile_override_for_agent_with(
            "orchestrator-ideation",
            &|name| match name {
                "RALPHX_CLAUDE_SETTINGS_PROFILE_ORCHESTRATOR_IDEATION" => {
                    Some("default".to_string())
                }
                _ => None,
            },
        );
        assert_eq!(selection.as_deref(), Some("default"));
    }

    #[test]
    fn test_normalize_agent_name_for_env_replaces_symbols() {
        assert_eq!(
            normalize_agent_name_for_env("ralphx:session-namer"),
            "RALPHX_SESSION_NAMER"
        );
    }

    #[test]
    fn test_settings_profile_defaults_apply_to_selected_profile() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profile_defaults:
    permissions:
      deny:
        - Read(./.env)
  settings_profiles:
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
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
            parsed.claude.settings,
            Some(serde_json::json!({
                "permissions": { "deny": ["Read(./.env)"] },
                "env": { "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic" }
            }))
        );
    }

    #[test]
    fn test_settings_profile_extends_supports_base_profile() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
    locked_down:
      permissions:
        deny:
          - Read(./.env)
          - Edit(./.env)
    z_ai:
      extends: locked_down
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
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
            parsed.claude.settings,
            Some(serde_json::json!({
                "permissions": {
                    "deny": ["Read(./.env)", "Edit(./.env)"]
                },
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic"
                }
            }))
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
            assert!(config
                .allowed_mcp_tools
                .contains(&"upsert_memories".to_string()));
            assert!(config
                .allowed_mcp_tools
                .contains(&"mark_memory_obsolete".to_string()));
            assert!(config
                .allowed_mcp_tools
                .contains(&"refresh_memory_rule_index".to_string()));
            assert!(config
                .allowed_mcp_tools
                .contains(&"ingest_rule_file".to_string()));
            assert!(config
                .allowed_mcp_tools
                .contains(&"rebuild_archive_snapshots".to_string()));
        }

        // Memory capture should have upsert_memories
        if let Some(config) = get_agent_config("memory-capture") {
            assert!(config
                .allowed_mcp_tools
                .contains(&"upsert_memories".to_string()));
        }
    }

    #[test]
    #[ignore = "memory read tools not yet added to worker/reviewer/orchestrator configs"]
    fn test_read_only_agents_have_read_memory_tools() {
        let read_memory_tools = vec!["search_memories", "get_memory", "get_memories_for_paths"];

        let agents_to_test = vec!["ralphx-worker", "ralphx-reviewer", "ralphx-orchestrator"];

        for agent_name in agents_to_test {
            if let Some(config) = get_agent_config(agent_name) {
                // Each of these should have at least one of the read memory tools
                let has_read_tool = read_memory_tools
                    .iter()
                    .any(|t| config.allowed_mcp_tools.contains(&t.to_string()));
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

    // ── Agent extends inheritance tests ─────────────────────────────

    #[test]
    fn test_extends_inherits_parent_tools() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: base-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit, Task] }
    mcp_tools: [start_step, complete_step]
    preapproved_cli_tools: [Write, Edit, Bash]
  - name: worker-team
    extends: base-worker
    system_prompt_file: ralphx-plugin/agents/worker-team.md
    model: opus
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let team = parsed
            .agents
            .iter()
            .find(|a| a.name == "worker-team")
            .expect("worker-team should exist");

        // model overridden by child
        assert_eq!(team.model.as_deref(), Some("opus"));
        // system_prompt_file overridden by child
        assert_eq!(team.system_prompt_file, "ralphx-plugin/agents/worker-team.md");
        // tools inherited from parent (child didn't specify)
        assert!(team.resolved_cli_tools.contains(&"Write".to_string()));
        assert!(team.resolved_cli_tools.contains(&"Edit".to_string()));
        assert!(team.resolved_cli_tools.contains(&"Task".to_string()));
        // mcp_tools inherited from parent
        assert!(team.allowed_mcp_tools.contains(&"start_step".to_string()));
        assert!(team.allowed_mcp_tools.contains(&"complete_step".to_string()));
        // preapproved_cli_tools inherited from parent
        assert!(team.preapproved_cli_tools.contains(&"Write".to_string()));
    }

    #[test]
    fn test_extends_child_overrides_mcp_tools() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: base-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [start_step, complete_step]
    preapproved_cli_tools: [Write]
  - name: custom-worker
    extends: base-worker
    mcp_tools: [get_task_context]
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let custom = parsed
            .agents
            .iter()
            .find(|a| a.name == "custom-worker")
            .expect("custom-worker should exist");

        // mcp_tools overridden by child
        assert_eq!(custom.allowed_mcp_tools, vec!["get_task_context"]);
        // model inherited
        assert_eq!(custom.model.as_deref(), Some("sonnet"));
        // system_prompt_file inherited
        assert_eq!(custom.system_prompt_file, "ralphx-plugin/agents/worker.md");
    }

    #[test]
    fn test_extends_circular_detection() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: agent-a
    extends: agent-b
    system_prompt_file: ralphx-plugin/agents/worker.md
  - name: agent-b
    extends: agent-a
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
        // Should parse without panic (circular breaks with warning)
        let parsed = parse_config(yaml).expect("config should parse despite circular extends");
        assert_eq!(parsed.agents.len(), 2);
    }

    #[test]
    fn test_extends_unknown_parent_keeps_child_as_is() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: orphan-agent
    extends: nonexistent-parent
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: haiku
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let agent = parsed
            .agents
            .iter()
            .find(|a| a.name == "orphan-agent")
            .expect("orphan-agent should exist");
        assert_eq!(agent.model.as_deref(), Some("haiku"));
    }

    #[test]
    fn test_extends_chained_inheritance() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: grandparent
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: haiku
    mcp_tools: [tool_a]
    preapproved_cli_tools: [Bash]
  - name: parent
    extends: grandparent
    model: sonnet
    mcp_tools: [tool_b]
  - name: child
    extends: parent
    model: opus
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let child = parsed
            .agents
            .iter()
            .find(|a| a.name == "child")
            .expect("child should exist");

        // model from child
        assert_eq!(child.model.as_deref(), Some("opus"));
        // mcp_tools from parent (overrides grandparent)
        assert_eq!(child.allowed_mcp_tools, vec!["tool_b"]);
        // system_prompt_file from grandparent (inherited through chain)
        assert_eq!(child.system_prompt_file, "ralphx-plugin/agents/worker.md");
        // preapproved_cli_tools from grandparent
        assert!(child.preapproved_cli_tools.contains(&"Bash".to_string()));
    }

    #[test]
    fn test_no_extends_backward_compatible() {
        // Agents without extends should work exactly as before
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: standalone
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: [Write, Bash]
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let agent = parsed
            .agents
            .iter()
            .find(|a| a.name == "standalone")
            .expect("standalone should exist");
        assert_eq!(agent.model.as_deref(), Some("sonnet"));
        assert!(agent.resolved_cli_tools.contains(&"Write".to_string()));
    }

    // ── Process mapping + team constraints integration tests ────────

    #[test]
    fn test_process_mapping_parsed_from_full_config() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
process_mapping:
  execution:
    default: ralphx-worker
    team: ralphx-worker-team
  ideation:
    default: orchestrator-ideation
agents:
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        assert_eq!(parsed.process_mapping.slots.len(), 2);
        assert_eq!(
            parsed.process_mapping.slots["execution"].default,
            "ralphx-worker"
        );
        assert_eq!(
            parsed.process_mapping.slots["execution"]
                .variants
                .get("team")
                .unwrap(),
            "ralphx-worker-team"
        );
    }

    #[test]
    fn test_team_constraints_parsed_from_full_config() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
team_constraints:
  _defaults:
    max_teammates: 5
    model_cap: sonnet
  execution:
    max_teammates: 3
    mode: dynamic
    timeout_minutes: 30
agents:
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        let defaults = parsed.team_constraints.defaults.as_ref().unwrap();
        assert_eq!(defaults.max_teammates, 5);
        let exec = &parsed.team_constraints.processes["execution"];
        assert_eq!(exec.max_teammates, 3);
        assert_eq!(exec.timeout_minutes, 30);
    }

    #[test]
    fn test_missing_process_mapping_uses_empty_default() {
        let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
"#;
        let parsed = parse_config(yaml).expect("config should parse");
        assert!(parsed.process_mapping.slots.is_empty());
        assert!(parsed.team_constraints.processes.is_empty());
        assert!(parsed.team_constraints.defaults.is_none());
    }
}
