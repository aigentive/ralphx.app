pub mod runtime_config;
pub mod team_config;
mod ui_config;
pub use ui_config::{UiConfig, UiFeatureFlagsConfig};

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;

#[allow(unused_imports)]
pub use team_config::{
    ApprovedTeamPlan, ApprovedTeammate, ProcessMapping, ProcessSlot, TeamConstraintError,
    TeamConstraints, TeamConstraintsConfig, TeamMode, TeammateSpawnRequest,
};

pub use runtime_config::{
    validate_external_mcp_config, AllRuntimeConfig, ExternalMcpConfig, GitRuntimeConfig,
    LimitsConfig, ReconciliationConfig, SchedulerConfig, StreamTimeoutsConfig,
    SupervisorRuntimeConfig, VerificationConfig,
};

const VALID_EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "max"];

fn validate_effort(value: &str, agent_name: &str) -> bool {
    if VALID_EFFORT_LEVELS.contains(&value) {
        true
    } else {
        tracing::warn!(agent = %agent_name, effort = %value, "Invalid effort level; ignoring");
        false
    }
}

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
    /// Optional per-agent effort level override (e.g. "max"). Validated at parse time.
    pub effort: Option<String>,
    /// Optional per-agent permission mode override (e.g. "acceptEdits"). None means inherit global.
    pub permission_mode: Option<String>,
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
    /// Global default effort level for all agents (e.g. "medium"). Validated at parse time.
    pub default_effort: String,
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
    effort: Option<String>,
    #[serde(default)]
    permission_mode: Option<String>,
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
    #[serde(default)]
    default_effort: Option<String>,
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
            default_effort: None,
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
    /// Write tracing output to a per-launch log file in addition to console.
    #[serde(default = "default_file_logging")]
    file_logging: bool,
    // ── Runtime config sections ──────────────────────────────────────
    #[serde(default)]
    timeouts: runtime_config::TimeoutsWrapper,
    #[serde(default)]
    reconciliation: ReconciliationConfig,
    #[serde(default)]
    git: GitRuntimeConfig,
    #[serde(default)]
    scheduler: SchedulerConfig,
    #[serde(default)]
    supervisor: SupervisorRuntimeConfig,
    #[serde(default)]
    limits: LimitsConfig,
    #[serde(default)]
    ideation: runtime_config::IdeationConfigWrapper,
    #[serde(default)]
    external_mcp: ExternalMcpConfig,
    #[serde(default)]
    ui: Option<UiConfig>,
}

const EMBEDDED_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ralphx.yaml"));

fn default_defer_merge_enabled() -> bool {
    true
}

fn default_file_logging() -> bool {
    true
}

struct LoadedConfig {
    agents: Vec<AgentConfig>,
    claude: ClaudeRuntimeConfig,
    process_mapping: ProcessMapping,
    team_constraints: TeamConstraintsConfig,
    defer_merge_enabled: bool,
    file_logging: bool,
    runtime: AllRuntimeConfig,
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

/// Resolve file_logging setting for early use (before tracing subscriber init).
/// Priority: RALPHX_FILE_LOGGING env > ralphx.yaml `file_logging` field > default (true).
///
/// This does a lightweight YAML parse — the full config is loaded lazily later.
pub fn resolve_file_logging_early() -> bool {
    if let Ok(val) = std::env::var("RALPHX_FILE_LOGGING") {
        return matches!(val.to_lowercase().as_str(), "true" | "1" | "yes");
    }

    #[derive(Deserialize)]
    struct MinimalConfig {
        #[serde(default = "default_file_logging_true")]
        file_logging: bool,
    }
    fn default_file_logging_true() -> bool {
        true
    }

    let path = config_path();
    if let Ok(contents) = std::fs::read_to_string(path) {
        if let Ok(cfg) = serde_yaml::from_str::<MinimalConfig>(&contents) {
            return cfg.file_logging;
        }
    }

    true
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
        effort: child.effort.clone().or_else(|| parent.effort.clone()),
        permission_mode: child.permission_mode.clone().or_else(|| parent.permission_mode.clone()),
    }
}

fn parse_config_with_lookup(
    yaml: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<LoadedConfig> {
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
        runtime_settings_profile_override_with(lookup).or_else(|| parsed.claude.settings_profile.clone());
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
            runtime_settings_profile_override_for_agent_with(&raw.name, lookup)
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
            effort: raw.effort.clone().filter(|v| validate_effort(v, &raw.name)),
            permission_mode: raw.permission_mode.clone(),
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
        default_effort: parsed
            .claude
            .default_effort
            .filter(|v| VALID_EFFORT_LEVELS.contains(&v.as_str()))
            .unwrap_or_else(|| "medium".to_string()),
    };

    let ui_feature_flags = parsed
        .ui
        .as_ref()
        .and_then(|u| u.feature_flags.clone())
        .unwrap_or_default();
    let mut runtime = AllRuntimeConfig {
        stream: parsed.timeouts.stream,
        reconciliation: parsed.reconciliation,
        git: parsed.git,
        scheduler: parsed.scheduler,
        supervisor: parsed.supervisor,
        limits: parsed.limits,
        verification: parsed.ideation.verification,
        external_mcp: parsed.external_mcp,
        child_session_activity_threshold_secs: parsed.ideation.child_session_activity_threshold_secs,
        ui_feature_flags,
    };
    if runtime.external_mcp.max_external_ideation_sessions != 1 {
        tracing::warn!(
            value = runtime.external_mcp.max_external_ideation_sessions,
            "ralphx.yaml: external_mcp.max_external_ideation_sessions is deprecated and has no \
             effect. The session gate was removed; sessions are always created. Remove this field."
        );
    }
    runtime_config::apply_env_overrides(&mut runtime);

    Some(LoadedConfig {
        agents: resolved,
        claude,
        process_mapping: parsed.process_mapping,
        team_constraints: parsed.team_constraints,
        defer_merge_enabled: parsed.defer_merge_enabled,
        file_logging: parsed.file_logging,
        runtime,
    })
}

fn parse_config(yaml: &str) -> Option<LoadedConfig> {
    parse_config_with_lookup(yaml, &|name| std::env::var(name).ok())
}

#[cfg(test)]
fn parse_config_no_env_overrides(yaml: &str) -> Option<LoadedConfig> {
    parse_config_with_lookup(yaml, &|_| None)
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

fn normalize_profile_name_for_env(profile_name: &str) -> String {
    let mut out = String::with_capacity(profile_name.len());
    for ch in profile_name.chars() {
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
    let selected_profile = if let Some(profile_name) = profile_selection {
        Some(profile_name)
    } else if raw.settings_profiles.contains_key("default") {
        Some("default")
    } else {
        None
    };

    let mut selected = if let Some(profile_name) = selected_profile {
        resolve_profile_settings(raw, profile_name)
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
        apply_prefixed_env_overrides(value, selected_profile);
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

fn apply_prefixed_env_overrides(
    settings: &mut serde_json::Value,
    profile_name: Option<&str>,
) {
    apply_prefixed_env_overrides_with(settings, profile_name, &|name| std::env::var(name).ok());
}

fn apply_prefixed_env_overrides_with(
    settings: &mut serde_json::Value,
    profile_name: Option<&str>,
    lookup: &dyn Fn(&str) -> Option<String>,
) {
    let Some(env_settings) = settings.get_mut("env").and_then(|v| v.as_object_mut()) else {
        return;
    };

    let normalized_profile = profile_name.map(normalize_profile_name_for_env);
    for (target_key, target_value) in env_settings.iter_mut() {
        let profile_source_key =
            normalized_profile.as_ref().map(|profile| format!("RALPHX_{profile}_{target_key}"));
        let generic_source_key = format!("RALPHX_{target_key}");

        let value = profile_source_key
            .as_deref()
            .and_then(lookup)
            .or_else(|| lookup(&generic_source_key));

        if let Some(value) = value {
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

    parse_config(EMBEDDED_CONFIG).unwrap_or_else(|| {
        let mut runtime = AllRuntimeConfig {
            stream: StreamTimeoutsConfig::default(),
            reconciliation: ReconciliationConfig::default(),
            git: GitRuntimeConfig::default(),
            scheduler: SchedulerConfig::default(),
            supervisor: SupervisorRuntimeConfig::default(),
            limits: LimitsConfig::default(),
            verification: VerificationConfig::default(),
            external_mcp: ExternalMcpConfig::default(),
            child_session_activity_threshold_secs: None,
            ui_feature_flags: UiFeatureFlagsConfig::default(),
        };
        runtime_config::apply_env_overrides(&mut runtime);
        LoadedConfig {
            agents: Vec::new(),
            claude: ClaudeRuntimeConfig {
                mcp_server_name: "ralphx".to_string(),
                setting_sources: None,
                permission_mode: "default".to_string(),
                dangerously_skip_permissions: false,
                permission_prompt_tool: "mcp__ralphx__permission_request".to_string(),
                use_append_system_prompt_file: true,
                settings: None,
                default_effort: "medium".to_string(),
            },
            process_mapping: ProcessMapping::default(),
            team_constraints: TeamConstraintsConfig::default(),
            defer_merge_enabled: true,
            file_logging: true,
            runtime,
        }
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
    &LOADED_CONFIG_CELL.get_or_init(load_config).team_constraints
}

pub fn defer_merge_enabled() -> bool {
    LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .defer_merge_enabled
}

pub fn file_logging_enabled() -> bool {
    LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .file_logging
}

pub fn stream_timeouts() -> &'static StreamTimeoutsConfig {
    &LOADED_CONFIG_CELL.get_or_init(load_config).runtime.stream
}

pub fn reconciliation_config() -> &'static ReconciliationConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .reconciliation
}

pub fn git_runtime_config() -> &'static GitRuntimeConfig {
    &LOADED_CONFIG_CELL.get_or_init(load_config).runtime.git
}

pub fn scheduler_config() -> &'static SchedulerConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .scheduler
}

pub fn supervisor_runtime_config() -> &'static SupervisorRuntimeConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .supervisor
}

pub fn limits_config() -> &'static LimitsConfig {
    &LOADED_CONFIG_CELL.get_or_init(load_config).runtime.limits
}

pub fn verification_config() -> &'static VerificationConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .verification
}

pub fn ui_feature_flags_config() -> &'static UiFeatureFlagsConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .ui_feature_flags
}

#[allow(dead_code)]
pub fn external_mcp_config() -> &'static ExternalMcpConfig {
    &LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .external_mcp
}

/// Returns the activity threshold (seconds) used by `get_child_session_status` to derive
/// `estimated_status`. Reads from `AllRuntimeConfig.child_session_activity_threshold_secs`,
/// defaulting to 10 if unset.
#[allow(dead_code)]
pub fn ideation_activity_threshold_secs() -> u64 {
    LOADED_CONFIG_CELL
        .get_or_init(load_config)
        .runtime
        .child_session_activity_threshold_secs
        .unwrap_or(10)
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

        // Always inject permission_request — required infrastructure tool, not agent-scoped.
        let permission_tool = format!("mcp__{}__permission_request", mcp_server);
        if !seen.contains(&permission_tool) {
            tools.push(permission_tool);
        }

        if tools.is_empty() {
            None
        } else {
            Some(tools.join(","))
        }
    })
}

#[cfg(test)]
mod tests;
