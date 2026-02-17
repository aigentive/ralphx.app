// Team configuration: process mapping, team constraints, and validation.
//
// Implements configurable agent variants from the product brief:
// - ProcessMapping: maps logical processes to agent variants (solo, team, readonly)
// - TeamConstraints: guardrails for dynamic team composition per process
// - TeamMode: Dynamic (lead chooses) vs Constrained (presets only)
// - Validation: enforce constraints on team plans before spawning
//
// NOTE: Many public functions here are foundational APIs consumed by later tasks
// (state machine integration, HTTP handlers, MCP tools). Allow dead_code until wired up.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Default value helpers ───────────────────────────────────────────────

fn default_max_teammates() -> u8 {
    5
}
fn default_model_cap() -> String {
    "sonnet".to_string()
}
fn default_mode() -> TeamMode {
    TeamMode::Dynamic
}
fn default_timeout() -> u32 {
    20
}

// ── Core types ──────────────────────────────────────────────────────────

/// A single process slot with a default agent and optional named variants.
///
/// ```yaml
/// execution:
///   default: ralphx-worker
///   team: ralphx-worker-team
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProcessSlot {
    pub default: String,
    #[serde(flatten)]
    pub variants: HashMap<String, String>,
}

/// Maps logical process names to their agent slots.
///
/// ```yaml
/// process_mapping:
///   ideation:
///     default: orchestrator-ideation
///     team: ideation-team-lead
///   execution:
///     default: ralphx-worker
///     team: ralphx-worker-team
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProcessMapping {
    #[serde(flatten)]
    pub slots: HashMap<String, ProcessSlot>,
}

/// Dynamic vs Constrained team mode.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TeamMode {
    Dynamic,
    Constrained,
}

impl Default for TeamMode {
    fn default() -> Self {
        TeamMode::Dynamic
    }
}

/// Per-process guardrails for team composition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamConstraints {
    #[serde(default = "default_max_teammates")]
    pub max_teammates: u8,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub allowed_mcp_tools: Vec<String>,
    #[serde(default = "default_model_cap")]
    pub model_cap: String,
    #[serde(default = "default_mode")]
    pub mode: TeamMode,
    #[serde(default)]
    pub presets: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_minutes: u32,
    #[serde(default)]
    pub budget_limit: Option<f64>,
}

impl Default for TeamConstraints {
    fn default() -> Self {
        Self {
            max_teammates: default_max_teammates(),
            allowed_tools: Vec::new(),
            allowed_mcp_tools: Vec::new(),
            model_cap: default_model_cap(),
            mode: default_mode(),
            presets: Vec::new(),
            timeout_minutes: default_timeout(),
            budget_limit: None,
        }
    }
}

/// All process constraints plus global defaults.
///
/// ```yaml
/// team_constraints:
///   _defaults:
///     max_teammates: 5
///     model_cap: sonnet
///   execution:
///     max_teammates: 5
///     allowed_tools: [Read, Write, Edit, Bash]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TeamConstraintsConfig {
    #[serde(rename = "_defaults", default)]
    pub defaults: Option<TeamConstraints>,
    #[serde(flatten)]
    pub processes: HashMap<String, TeamConstraints>,
}

/// A single teammate in a spawn request from a team lead.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeammateSpawnRequest {
    pub role: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mcp_tools: Vec<String>,
    #[serde(default = "default_model_cap")]
    pub model: String,
    #[serde(default)]
    pub prompt_summary: Option<String>,
}

/// A validated teammate in an approved team plan.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovedTeammate {
    pub role: String,
    pub approved_tools: Vec<String>,
    pub approved_mcp_tools: Vec<String>,
    pub approved_model: String,
    pub from_preset: bool,
}

/// Backend-approved team composition.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovedTeamPlan {
    pub plan_id: String,
    pub process: String,
    pub teammates: Vec<ApprovedTeammate>,
}

/// Errors from team constraint validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamConstraintError {
    MaxTeammatesExceeded {
        max: u8,
        requested: usize,
    },
    ToolNotAllowed {
        tool: String,
        role: String,
    },
    McpToolNotAllowed {
        tool: String,
        role: String,
    },
    ModelExceedsCap {
        requested: String,
        cap: String,
    },
    PresetRequired {
        role: String,
    },
    AgentNotInPresets {
        agent: String,
        allowed: Vec<String>,
    },
    UnknownProcess {
        process: String,
    },
}

impl std::fmt::Display for TeamConstraintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxTeammatesExceeded { max, requested } => {
                write!(f, "Max teammates exceeded: {requested} > {max}")
            }
            Self::ToolNotAllowed { tool, role } => {
                write!(f, "Tool '{tool}' not allowed for role '{role}'")
            }
            Self::McpToolNotAllowed { tool, role } => {
                write!(f, "MCP tool '{tool}' not allowed for role '{role}'")
            }
            Self::ModelExceedsCap { requested, cap } => {
                write!(f, "Model '{requested}' exceeds cap '{cap}'")
            }
            Self::PresetRequired { role } => {
                write!(
                    f,
                    "Constrained mode requires preset for role '{role}'"
                )
            }
            Self::AgentNotInPresets { agent, allowed } => {
                write!(
                    f,
                    "Agent '{agent}' not in allowed presets: {allowed:?}"
                )
            }
            Self::UnknownProcess { process } => {
                write!(f, "Unknown process: '{process}'")
            }
        }
    }
}

impl std::error::Error for TeamConstraintError {}

// ── Model tier ordering ─────────────────────────────────────────────────

/// Model tier for comparison. Higher ordinal = more capable.
fn model_tier(model: &str) -> u8 {
    match model.to_lowercase().as_str() {
        "haiku" => 1,
        "sonnet" => 2,
        "opus" => 3,
        _ => 0, // unknown models get lowest tier
    }
}

/// Check if a requested model is within the allowed cap.
pub fn model_within_cap(requested: &str, cap: &str) -> bool {
    model_tier(requested) <= model_tier(cap)
}

/// Return the lesser (more restrictive) of two model caps.
/// Ordering: haiku < sonnet < opus (lower tier = more restrictive).
/// Unknown models are treated as tier 0 (most restrictive).
pub fn min_model_cap(a: &str, b: &str) -> String {
    let a_tier = model_tier(a);
    let b_tier = model_tier(b);
    if a_tier <= b_tier {
        a.to_lowercase()
    } else {
        b.to_lowercase()
    }
}

// ── Constraint resolution ───────────────────────────────────────────────

/// Get effective constraints for a process, merging with defaults.
///
/// Priority: process-specific > `_defaults` > hardcoded defaults.
pub fn get_team_constraints(
    config: &TeamConstraintsConfig,
    process: &str,
) -> TeamConstraints {
    let defaults = config.defaults.as_ref();
    let process_specific = config.processes.get(process);

    match (process_specific, defaults) {
        (Some(proc), Some(def)) => merge_constraints(def, proc),
        (Some(proc), None) => proc.clone(),
        (None, Some(def)) => def.clone(),
        (None, None) => TeamConstraints::default(),
    }
}

/// Merge default constraints with process-specific overrides.
/// Process-specific values take precedence; empty collections fall through to defaults.
fn merge_constraints(defaults: &TeamConstraints, specific: &TeamConstraints) -> TeamConstraints {
    TeamConstraints {
        max_teammates: specific.max_teammates,
        allowed_tools: if specific.allowed_tools.is_empty() {
            defaults.allowed_tools.clone()
        } else {
            specific.allowed_tools.clone()
        },
        allowed_mcp_tools: if specific.allowed_mcp_tools.is_empty() {
            defaults.allowed_mcp_tools.clone()
        } else {
            specific.allowed_mcp_tools.clone()
        },
        model_cap: specific.model_cap.clone(),
        mode: specific.mode.clone(),
        presets: if specific.presets.is_empty() {
            defaults.presets.clone()
        } else {
            specific.presets.clone()
        },
        timeout_minutes: specific.timeout_minutes,
        budget_limit: specific.budget_limit.or(defaults.budget_limit),
    }
}

/// Validate child session's team config against a constraint ceiling.
///
/// Caps values at min(resolved, ceiling) to prevent privilege escalation.
/// Used when a child session inherits team config from its parent.
///
/// # Rules
/// - `max_teammates`: min(resolved, ceiling)
/// - `model_cap`: lesser model tier (haiku < sonnet < opus)
/// - `allowed_tools`: intersection of both lists (empty ceiling = no restriction)
/// - `allowed_mcp_tools`: intersection of both lists
/// - `presets`: intersection of both lists
/// - `timeout_minutes`: min(resolved, ceiling)
/// - `budget_limit`: min(resolved, ceiling); None is most restrictive
/// - `mode`: inherited from resolved (not capped)
pub fn validate_child_team_config(
    resolved: &TeamConstraints,
    ceiling: &TeamConstraints,
) -> TeamConstraints {
    TeamConstraints {
        max_teammates: resolved.max_teammates.min(ceiling.max_teammates),
        allowed_tools: intersect_lists(&resolved.allowed_tools, &ceiling.allowed_tools),
        allowed_mcp_tools: intersect_lists(&resolved.allowed_mcp_tools, &ceiling.allowed_mcp_tools),
        model_cap: min_model_cap(&resolved.model_cap, &ceiling.model_cap),
        mode: resolved.mode.clone(),
        presets: intersect_lists(&resolved.presets, &ceiling.presets),
        timeout_minutes: resolved.timeout_minutes.min(ceiling.timeout_minutes),
        budget_limit: min_budget(resolved.budget_limit, ceiling.budget_limit),
    }
}

/// Intersect two lists. If ceiling is empty, return resolved (no restriction).
fn intersect_lists(resolved: &[String], ceiling: &[String]) -> Vec<String> {
    if ceiling.is_empty() {
        return resolved.to_vec();
    }
    resolved
        .iter()
        .filter(|item| ceiling.contains(item))
        .cloned()
        .collect()
}

/// Return the lesser of two optional budgets. None = no limit (most permissive).
fn min_budget(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a_val), Some(b_val)) => Some(a_val.min(b_val)),
        (Some(a_val), None) => Some(a_val),
        (None, Some(_)) => None, // None = no budget allowed (most restrictive)
        (None, None) => None,
    }
}

// ── Process agent resolution ────────────────────────────────────────────

/// Resolve which agent to use for a process + variant combination.
///
/// Fallback chain: process_mapping variant → process_mapping default → None.
pub fn resolve_process_agent(
    mapping: &ProcessMapping,
    process: &str,
    variant: &str,
) -> Option<String> {
    let slot = mapping.slots.get(process)?;

    if variant == "default" {
        return Some(slot.default.clone());
    }

    slot.variants
        .get(variant)
        .cloned()
        .or_else(|| Some(slot.default.clone()))
}

// ── Team plan validation ────────────────────────────────────────────────

/// Validate an entire team composition plan against constraints.
pub fn validate_team_plan(
    constraints: &TeamConstraints,
    process: &str,
    teammates: &[TeammateSpawnRequest],
) -> Result<ApprovedTeamPlan, TeamConstraintError> {
    // 1. Check teammate count
    if teammates.len() > constraints.max_teammates as usize {
        return Err(TeamConstraintError::MaxTeammatesExceeded {
            max: constraints.max_teammates,
            requested: teammates.len(),
        });
    }

    // 2. Per-teammate validation
    let mut approved = Vec::with_capacity(teammates.len());
    for req in teammates {
        let teammate = validate_teammate(constraints, req)?;
        approved.push(teammate);
    }

    Ok(ApprovedTeamPlan {
        plan_id: format!("tp-{}", uuid::Uuid::new_v4().simple()),
        process: process.to_string(),
        teammates: approved,
    })
}

fn validate_teammate(
    constraints: &TeamConstraints,
    req: &TeammateSpawnRequest,
) -> Result<ApprovedTeammate, TeamConstraintError> {
    match constraints.mode {
        TeamMode::Constrained => {
            let preset = req.preset.as_ref().ok_or_else(|| {
                TeamConstraintError::PresetRequired {
                    role: req.role.clone(),
                }
            })?;
            if !constraints.presets.contains(preset) {
                return Err(TeamConstraintError::AgentNotInPresets {
                    agent: preset.clone(),
                    allowed: constraints.presets.clone(),
                });
            }
            Ok(ApprovedTeammate {
                role: req.role.clone(),
                approved_tools: Vec::new(), // filled from preset config at spawn time
                approved_mcp_tools: Vec::new(),
                approved_model: String::new(),
                from_preset: true,
            })
        }
        TeamMode::Dynamic => {
            // Validate CLI tools
            if !constraints.allowed_tools.is_empty() {
                for tool in &req.tools {
                    if !constraints.allowed_tools.contains(tool) {
                        return Err(TeamConstraintError::ToolNotAllowed {
                            tool: tool.clone(),
                            role: req.role.clone(),
                        });
                    }
                }
            }
            // Validate MCP tools
            if !constraints.allowed_mcp_tools.is_empty() {
                for tool in &req.mcp_tools {
                    if !constraints.allowed_mcp_tools.contains(tool) {
                        return Err(TeamConstraintError::McpToolNotAllowed {
                            tool: tool.clone(),
                            role: req.role.clone(),
                        });
                    }
                }
            }
            // Validate model tier
            if !model_within_cap(&req.model, &constraints.model_cap) {
                return Err(TeamConstraintError::ModelExceedsCap {
                    requested: req.model.clone(),
                    cap: constraints.model_cap.clone(),
                });
            }
            Ok(ApprovedTeammate {
                role: req.role.clone(),
                approved_tools: req.tools.clone(),
                approved_mcp_tools: req.mcp_tools.clone(),
                approved_model: req.model.clone(),
                from_preset: req.preset.is_some(),
            })
        }
    }
}

// ── Environment variable overrides ──────────────────────────────────────

/// Apply environment variable overrides to team constraints.
///
/// Supported vars:
/// - `RALPHX_TEAM_MODE_<PROCESS>` → override mode (dynamic|constrained)
/// - `RALPHX_TEAM_MAX_<PROCESS>` → override max_teammates
/// - `RALPHX_TEAM_MODEL_CAP_<PROCESS>` → override model_cap
pub fn apply_env_overrides(constraints: &mut TeamConstraints, process: &str) {
    apply_env_overrides_with(constraints, process, &|name| std::env::var(name).ok());
}

fn apply_env_overrides_with(
    constraints: &mut TeamConstraints,
    process: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) {
    let key = process.to_ascii_uppercase();

    if let Some(mode) = lookup(&format!("RALPHX_TEAM_MODE_{key}")) {
        match mode.to_lowercase().as_str() {
            "dynamic" => constraints.mode = TeamMode::Dynamic,
            "constrained" => constraints.mode = TeamMode::Constrained,
            _ => tracing::warn!(var = %format!("RALPHX_TEAM_MODE_{key}"), value = %mode, "Invalid team mode"),
        }
    }

    if let Some(max_str) = lookup(&format!("RALPHX_TEAM_MAX_{key}")) {
        if let Ok(max) = max_str.parse::<u8>() {
            constraints.max_teammates = max;
        }
    }

    if let Some(cap) = lookup(&format!("RALPHX_TEAM_MODEL_CAP_{key}")) {
        let trimmed = cap.trim().to_lowercase();
        if matches!(trimmed.as_str(), "haiku" | "sonnet" | "opus") {
            constraints.model_cap = trimmed;
        }
    }
}

/// Read process variant override from env.
///
/// `RALPHX_PROCESS_VARIANT_<PROCESS>` → variant name (e.g. "team")
pub fn env_variant_override(process: &str) -> Option<String> {
    env_variant_override_with(process, &|name| std::env::var(name).ok())
}

fn env_variant_override_with(
    process: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    let key = format!("RALPHX_PROCESS_VARIANT_{}", process.to_ascii_uppercase());
    lookup(&key).and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    })
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Model tier tests ────────────────────────────────────────────

    #[test]
    fn test_model_within_cap_haiku_under_sonnet() {
        assert!(model_within_cap("haiku", "sonnet"));
    }

    #[test]
    fn test_model_within_cap_sonnet_equals_sonnet() {
        assert!(model_within_cap("sonnet", "sonnet"));
    }

    #[test]
    fn test_model_within_cap_opus_exceeds_sonnet() {
        assert!(!model_within_cap("opus", "sonnet"));
    }

    #[test]
    fn test_model_within_cap_opus_under_opus() {
        assert!(model_within_cap("opus", "opus"));
    }

    #[test]
    fn test_model_within_cap_unknown_model_always_within() {
        assert!(model_within_cap("unknown", "haiku"));
    }

    #[test]
    fn test_model_within_cap_case_insensitive() {
        assert!(model_within_cap("Haiku", "SONNET"));
    }

    // ── ProcessSlot deserialization tests ────────────────────────────

    #[test]
    fn test_process_slot_deserialize_with_variants() {
        let yaml = r#"
default: ralphx-worker
team: ralphx-worker-team
"#;
        let slot: ProcessSlot = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(slot.default, "ralphx-worker");
        assert_eq!(slot.variants.get("team").unwrap(), "ralphx-worker-team");
    }

    #[test]
    fn test_process_slot_deserialize_default_only() {
        let yaml = "default: ralphx-merger\n";
        let slot: ProcessSlot = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(slot.default, "ralphx-merger");
        assert!(slot.variants.is_empty());
    }

    // ── ProcessMapping deserialization tests ─────────────────────────

    #[test]
    fn test_process_mapping_deserialize_full() {
        let yaml = r#"
ideation:
  default: orchestrator-ideation
  readonly: orchestrator-ideation-readonly
  team: ideation-team-lead
execution:
  default: ralphx-worker
  team: ralphx-worker-team
merge:
  default: ralphx-merger
"#;
        let mapping: ProcessMapping = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(mapping.slots.len(), 3);
        assert_eq!(mapping.slots["ideation"].default, "orchestrator-ideation");
        assert_eq!(
            mapping.slots["ideation"].variants.get("team").unwrap(),
            "ideation-team-lead"
        );
        assert_eq!(mapping.slots["execution"].default, "ralphx-worker");
        assert_eq!(mapping.slots["merge"].default, "ralphx-merger");
    }

    #[test]
    fn test_process_mapping_empty_is_default() {
        let mapping = ProcessMapping::default();
        assert!(mapping.slots.is_empty());
    }

    // ── TeamConstraints deserialization tests ────────────────────────

    #[test]
    fn test_team_constraints_defaults() {
        let tc = TeamConstraints::default();
        assert_eq!(tc.max_teammates, 5);
        assert_eq!(tc.model_cap, "sonnet");
        assert_eq!(tc.mode, TeamMode::Dynamic);
        assert_eq!(tc.timeout_minutes, 20);
        assert!(tc.budget_limit.is_none());
        assert!(tc.allowed_tools.is_empty());
        assert!(tc.presets.is_empty());
    }

    #[test]
    fn test_team_constraints_deserialize_full() {
        let yaml = r#"
max_teammates: 3
allowed_tools: [Read, Write, Edit]
allowed_mcp_tools: [get_task_context]
model_cap: opus
mode: constrained
presets: [ralphx-coder, ralphx-reviewer]
timeout_minutes: 45
budget_limit: 10.50
"#;
        let tc: TeamConstraints = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.max_teammates, 3);
        assert_eq!(tc.allowed_tools, vec!["Read", "Write", "Edit"]);
        assert_eq!(tc.allowed_mcp_tools, vec!["get_task_context"]);
        assert_eq!(tc.model_cap, "opus");
        assert_eq!(tc.mode, TeamMode::Constrained);
        assert_eq!(tc.presets, vec!["ralphx-coder", "ralphx-reviewer"]);
        assert_eq!(tc.timeout_minutes, 45);
        assert_eq!(tc.budget_limit, Some(10.50));
    }

    #[test]
    fn test_team_constraints_deserialize_partial_uses_defaults() {
        let yaml = "max_teammates: 2\n";
        let tc: TeamConstraints = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(tc.max_teammates, 2);
        assert_eq!(tc.model_cap, "sonnet"); // default
        assert_eq!(tc.mode, TeamMode::Dynamic); // default
        assert_eq!(tc.timeout_minutes, 20); // default
    }

    // ── TeamConstraintsConfig deserialization tests ──────────────────

    #[test]
    fn test_team_constraints_config_with_defaults_and_processes() {
        let yaml = r#"
_defaults:
  max_teammates: 5
  model_cap: sonnet
  mode: dynamic
  timeout_minutes: 20
execution:
  max_teammates: 5
  allowed_tools: [Read, Write, Edit, Bash]
  model_cap: sonnet
  mode: dynamic
  timeout_minutes: 30
review:
  max_teammates: 2
  mode: constrained
  presets: [ralphx-reviewer]
"#;
        let config: TeamConstraintsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.defaults.is_some());
        assert_eq!(config.processes.len(), 2);
        assert_eq!(config.processes["execution"].max_teammates, 5);
        assert_eq!(config.processes["review"].mode, TeamMode::Constrained);
    }

    #[test]
    fn test_team_constraints_config_empty_is_default() {
        let config = TeamConstraintsConfig::default();
        assert!(config.defaults.is_none());
        assert!(config.processes.is_empty());
    }

    // ── get_team_constraints tests ──────────────────────────────────

    #[test]
    fn test_get_team_constraints_process_specific_found() {
        let config = TeamConstraintsConfig {
            defaults: Some(TeamConstraints {
                max_teammates: 10,
                ..TeamConstraints::default()
            }),
            processes: {
                let mut m = HashMap::new();
                m.insert(
                    "execution".to_string(),
                    TeamConstraints {
                        max_teammates: 3,
                        allowed_tools: vec!["Read".to_string()],
                        ..TeamConstraints::default()
                    },
                );
                m
            },
        };
        let tc = get_team_constraints(&config, "execution");
        assert_eq!(tc.max_teammates, 3);
        assert_eq!(tc.allowed_tools, vec!["Read"]);
    }

    #[test]
    fn test_get_team_constraints_falls_back_to_defaults() {
        let config = TeamConstraintsConfig {
            defaults: Some(TeamConstraints {
                max_teammates: 7,
                model_cap: "opus".to_string(),
                ..TeamConstraints::default()
            }),
            processes: HashMap::new(),
        };
        let tc = get_team_constraints(&config, "ideation");
        assert_eq!(tc.max_teammates, 7);
        assert_eq!(tc.model_cap, "opus");
    }

    #[test]
    fn test_get_team_constraints_no_config_uses_hardcoded_defaults() {
        let config = TeamConstraintsConfig::default();
        let tc = get_team_constraints(&config, "execution");
        assert_eq!(tc.max_teammates, 5);
        assert_eq!(tc.model_cap, "sonnet");
        assert_eq!(tc.mode, TeamMode::Dynamic);
    }

    #[test]
    fn test_merge_constraints_specific_tools_override_defaults() {
        let defaults = TeamConstraints {
            allowed_tools: vec!["Read".to_string(), "Grep".to_string()],
            ..TeamConstraints::default()
        };
        let specific = TeamConstraints {
            allowed_tools: vec!["Write".to_string()],
            ..TeamConstraints::default()
        };
        let merged = merge_constraints(&defaults, &specific);
        assert_eq!(merged.allowed_tools, vec!["Write"]);
    }

    #[test]
    fn test_merge_constraints_empty_specific_tools_inherits_defaults() {
        let defaults = TeamConstraints {
            allowed_tools: vec!["Read".to_string(), "Grep".to_string()],
            ..TeamConstraints::default()
        };
        let specific = TeamConstraints {
            max_teammates: 2,
            allowed_tools: Vec::new(),
            ..TeamConstraints::default()
        };
        let merged = merge_constraints(&defaults, &specific);
        assert_eq!(merged.allowed_tools, vec!["Read", "Grep"]);
        assert_eq!(merged.max_teammates, 2);
    }

    // ── resolve_process_agent tests ─────────────────────────────────

    #[test]
    fn test_resolve_process_agent_default_variant() {
        let mapping = build_test_mapping();
        let agent = resolve_process_agent(&mapping, "execution", "default");
        assert_eq!(agent, Some("ralphx-worker".to_string()));
    }

    #[test]
    fn test_resolve_process_agent_team_variant() {
        let mapping = build_test_mapping();
        let agent = resolve_process_agent(&mapping, "execution", "team");
        assert_eq!(agent, Some("ralphx-worker-team".to_string()));
    }

    #[test]
    fn test_resolve_process_agent_unknown_variant_falls_back_to_default() {
        let mapping = build_test_mapping();
        let agent = resolve_process_agent(&mapping, "execution", "nonexistent");
        assert_eq!(agent, Some("ralphx-worker".to_string()));
    }

    #[test]
    fn test_resolve_process_agent_unknown_process_returns_none() {
        let mapping = build_test_mapping();
        let agent = resolve_process_agent(&mapping, "unknown_process", "default");
        assert_eq!(agent, None);
    }

    #[test]
    fn test_resolve_process_agent_empty_mapping_returns_none() {
        let mapping = ProcessMapping::default();
        assert_eq!(resolve_process_agent(&mapping, "execution", "default"), None);
    }

    // ── validate_team_plan tests ────────────────────────────────────

    #[test]
    fn test_validate_team_plan_dynamic_valid() {
        let constraints = TeamConstraints {
            max_teammates: 3,
            allowed_tools: vec!["Read".to_string(), "Write".to_string(), "Grep".to_string()],
            allowed_mcp_tools: vec!["get_task_context".to_string()],
            model_cap: "sonnet".to_string(),
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![
            TeammateSpawnRequest {
                role: "coder".to_string(),
                tools: vec!["Read".to_string(), "Write".to_string()],
                mcp_tools: vec!["get_task_context".to_string()],
                model: "sonnet".to_string(),
                ..default_spawn_request()
            },
            TeammateSpawnRequest {
                role: "tester".to_string(),
                tools: vec!["Read".to_string(), "Grep".to_string()],
                mcp_tools: vec![],
                model: "haiku".to_string(),
                ..default_spawn_request()
            },
        ];

        let plan = validate_team_plan(&constraints, "execution", &teammates).unwrap();
        assert_eq!(plan.process, "execution");
        assert_eq!(plan.teammates.len(), 2);
        assert!(!plan.teammates[0].from_preset);
        assert_eq!(plan.teammates[0].approved_model, "sonnet");
        assert_eq!(plan.teammates[1].approved_model, "haiku");
    }

    #[test]
    fn test_validate_team_plan_exceeds_max_teammates() {
        let constraints = TeamConstraints {
            max_teammates: 1,
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![
            TeammateSpawnRequest {
                role: "a".to_string(),
                ..default_spawn_request()
            },
            TeammateSpawnRequest {
                role: "b".to_string(),
                ..default_spawn_request()
            },
        ];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::MaxTeammatesExceeded {
                max: 1,
                requested: 2
            }
        );
    }

    #[test]
    fn test_validate_team_plan_tool_not_allowed() {
        let constraints = TeamConstraints {
            max_teammates: 5,
            allowed_tools: vec!["Read".to_string()],
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "coder".to_string(),
            tools: vec!["Read".to_string(), "Write".to_string()],
            ..default_spawn_request()
        }];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::ToolNotAllowed {
                tool: "Write".to_string(),
                role: "coder".to_string(),
            }
        );
    }

    #[test]
    fn test_validate_team_plan_mcp_tool_not_allowed() {
        let constraints = TeamConstraints {
            max_teammates: 5,
            allowed_mcp_tools: vec!["get_task_context".to_string()],
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "coder".to_string(),
            mcp_tools: vec!["start_step".to_string()],
            ..default_spawn_request()
        }];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::McpToolNotAllowed {
                tool: "start_step".to_string(),
                role: "coder".to_string(),
            }
        );
    }

    #[test]
    fn test_validate_team_plan_model_exceeds_cap() {
        let constraints = TeamConstraints {
            max_teammates: 5,
            model_cap: "sonnet".to_string(),
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "researcher".to_string(),
            model: "opus".to_string(),
            ..default_spawn_request()
        }];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::ModelExceedsCap {
                requested: "opus".to_string(),
                cap: "sonnet".to_string(),
            }
        );
    }

    #[test]
    fn test_validate_team_plan_dynamic_empty_allowed_tools_permits_all() {
        // When allowed_tools is empty, no tool restriction is enforced
        let constraints = TeamConstraints {
            max_teammates: 5,
            allowed_tools: vec![],
            mode: TeamMode::Dynamic,
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "coder".to_string(),
            tools: vec!["Read".to_string(), "Write".to_string(), "Edit".to_string()],
            ..default_spawn_request()
        }];
        assert!(validate_team_plan(&constraints, "execution", &teammates).is_ok());
    }

    #[test]
    fn test_validate_team_plan_constrained_valid_preset() {
        let constraints = TeamConstraints {
            max_teammates: 3,
            mode: TeamMode::Constrained,
            presets: vec!["ralphx-coder".to_string(), "ralphx-reviewer".to_string()],
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "coder".to_string(),
            preset: Some("ralphx-coder".to_string()),
            ..default_spawn_request()
        }];
        let plan = validate_team_plan(&constraints, "execution", &teammates).unwrap();
        assert!(plan.teammates[0].from_preset);
    }

    #[test]
    fn test_validate_team_plan_constrained_no_preset_fails() {
        let constraints = TeamConstraints {
            max_teammates: 3,
            mode: TeamMode::Constrained,
            presets: vec!["ralphx-coder".to_string()],
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "coder".to_string(),
            preset: None,
            ..default_spawn_request()
        }];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::PresetRequired {
                role: "coder".to_string()
            }
        );
    }

    #[test]
    fn test_validate_team_plan_constrained_unknown_preset_fails() {
        let constraints = TeamConstraints {
            max_teammates: 3,
            mode: TeamMode::Constrained,
            presets: vec!["ralphx-coder".to_string()],
            ..TeamConstraints::default()
        };
        let teammates = vec![TeammateSpawnRequest {
            role: "hacker".to_string(),
            preset: Some("unknown-agent".to_string()),
            ..default_spawn_request()
        }];
        let err = validate_team_plan(&constraints, "execution", &teammates).unwrap_err();
        assert_eq!(
            err,
            TeamConstraintError::AgentNotInPresets {
                agent: "unknown-agent".to_string(),
                allowed: vec!["ralphx-coder".to_string()],
            }
        );
    }

    #[test]
    fn test_validate_team_plan_empty_teammates_ok() {
        let constraints = TeamConstraints::default();
        let plan = validate_team_plan(&constraints, "execution", &[]).unwrap();
        assert!(plan.teammates.is_empty());
    }

    // ── Environment variable override tests ─────────────────────────

    #[test]
    fn test_env_override_team_mode() {
        let mut tc = TeamConstraints::default();
        apply_env_overrides_with(&mut tc, "execution", &|name| match name {
            "RALPHX_TEAM_MODE_EXECUTION" => Some("constrained".to_string()),
            _ => None,
        });
        assert_eq!(tc.mode, TeamMode::Constrained);
    }

    #[test]
    fn test_env_override_max_teammates() {
        let mut tc = TeamConstraints::default();
        apply_env_overrides_with(&mut tc, "execution", &|name| match name {
            "RALPHX_TEAM_MAX_EXECUTION" => Some("8".to_string()),
            _ => None,
        });
        assert_eq!(tc.max_teammates, 8);
    }

    #[test]
    fn test_env_override_model_cap() {
        let mut tc = TeamConstraints::default();
        apply_env_overrides_with(&mut tc, "execution", &|name| match name {
            "RALPHX_TEAM_MODEL_CAP_EXECUTION" => Some("opus".to_string()),
            _ => None,
        });
        assert_eq!(tc.model_cap, "opus");
    }

    #[test]
    fn test_env_override_invalid_model_cap_ignored() {
        let mut tc = TeamConstraints::default();
        apply_env_overrides_with(&mut tc, "execution", &|name| match name {
            "RALPHX_TEAM_MODEL_CAP_EXECUTION" => Some("invalid".to_string()),
            _ => None,
        });
        assert_eq!(tc.model_cap, "sonnet"); // unchanged
    }

    #[test]
    fn test_env_variant_override() {
        let variant = env_variant_override_with("execution", &|name| match name {
            "RALPHX_PROCESS_VARIANT_EXECUTION" => Some("team".to_string()),
            _ => None,
        });
        assert_eq!(variant.as_deref(), Some("team"));
    }

    #[test]
    fn test_env_variant_override_blank_is_none() {
        let variant = env_variant_override_with("execution", &|name| match name {
            "RALPHX_PROCESS_VARIANT_EXECUTION" => Some("  ".to_string()),
            _ => None,
        });
        assert_eq!(variant, None);
    }

    #[test]
    fn test_env_variant_override_missing_is_none() {
        let variant = env_variant_override_with("execution", &|_| None);
        assert_eq!(variant, None);
    }

    // ── TeamMode serde tests ────────────────────────────────────────

    #[test]
    fn test_team_mode_deserialize_dynamic() {
        let mode: TeamMode = serde_yaml::from_str("dynamic").unwrap();
        assert_eq!(mode, TeamMode::Dynamic);
    }

    #[test]
    fn test_team_mode_deserialize_constrained() {
        let mode: TeamMode = serde_yaml::from_str("constrained").unwrap();
        assert_eq!(mode, TeamMode::Constrained);
    }

    #[test]
    fn test_team_mode_serialize_roundtrip() {
        let json = serde_json::to_string(&TeamMode::Dynamic).unwrap();
        assert_eq!(json, "\"dynamic\"");
        let json = serde_json::to_string(&TeamMode::Constrained).unwrap();
        assert_eq!(json, "\"constrained\"");
    }

    // ── TeamConstraintError display tests ───────────────────────────

    #[test]
    fn test_constraint_error_display() {
        let err = TeamConstraintError::MaxTeammatesExceeded {
            max: 3,
            requested: 5,
        };
        assert_eq!(err.to_string(), "Max teammates exceeded: 5 > 3");
    }

    // ── min_model_cap tests ──────────────────────────────────────────

    #[test]
    fn test_min_model_cap_haiku_vs_sonnet() {
        assert_eq!(min_model_cap("haiku", "sonnet"), "haiku");
        assert_eq!(min_model_cap("sonnet", "haiku"), "haiku");
    }

    #[test]
    fn test_min_model_cap_sonnet_vs_opus() {
        assert_eq!(min_model_cap("sonnet", "opus"), "sonnet");
        assert_eq!(min_model_cap("opus", "sonnet"), "sonnet");
    }

    #[test]
    fn test_min_model_cap_haiku_vs_opus() {
        assert_eq!(min_model_cap("haiku", "opus"), "haiku");
        assert_eq!(min_model_cap("opus", "haiku"), "haiku");
    }

    #[test]
    fn test_min_model_cap_equal_models() {
        assert_eq!(min_model_cap("sonnet", "sonnet"), "sonnet");
        assert_eq!(min_model_cap("opus", "opus"), "opus");
        assert_eq!(min_model_cap("haiku", "haiku"), "haiku");
    }

    #[test]
    fn test_min_model_cap_case_insensitive() {
        assert_eq!(min_model_cap("HAIKU", "Sonnet"), "haiku");
        assert_eq!(min_model_cap("OPUS", "sonnet"), "sonnet");
    }

    #[test]
    fn test_min_model_cap_unknown_treated_as_lowest() {
        // Unknown models have tier 0 (lowest), so the known model wins
        assert_eq!(min_model_cap("unknown", "haiku"), "unknown");
        assert_eq!(min_model_cap("haiku", "unknown"), "unknown");
    }

    // ── validate_child_team_config tests ─────────────────────────────

    #[test]
    fn test_validate_child_team_config_caps_max_teammates() {
        let resolved = TeamConstraints {
            max_teammates: 5,
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            max_teammates: 3,
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.max_teammates, 3);
    }

    #[test]
    fn test_validate_child_team_config_caps_model_cap() {
        let resolved = TeamConstraints {
            model_cap: "opus".to_string(),
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            model_cap: "sonnet".to_string(),
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.model_cap, "sonnet");
    }

    #[test]
    fn test_validate_child_team_config_caps_model_cap_parent_higher() {
        let resolved = TeamConstraints {
            model_cap: "haiku".to_string(),
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            model_cap: "opus".to_string(),
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.model_cap, "haiku");
    }

    #[test]
    fn test_validate_child_team_config_intersects_allowed_tools() {
        let resolved = TeamConstraints {
            allowed_tools: vec!["Read".to_string(), "Write".to_string(), "Edit".to_string()],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            allowed_tools: vec!["Read".to_string(), "Grep".to_string()],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.allowed_tools, vec!["Read"]);
    }

    #[test]
    fn test_validate_child_team_config_intersects_allowed_mcp_tools() {
        let resolved = TeamConstraints {
            allowed_mcp_tools: vec!["get_task_context".to_string(), "start_step".to_string()],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            allowed_mcp_tools: vec!["get_task_context".to_string(), "complete_step".to_string()],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.allowed_mcp_tools, vec!["get_task_context"]);
    }

    #[test]
    fn test_validate_child_team_config_intersects_presets() {
        let resolved = TeamConstraints {
            presets: vec!["ralphx-coder".to_string(), "ralphx-reviewer".to_string()],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            presets: vec!["ralphx-coder".to_string()],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.presets, vec!["ralphx-coder"]);
    }

    #[test]
    fn test_validate_child_team_config_presets_no_intersection() {
        let resolved = TeamConstraints {
            presets: vec!["ralphx-reviewer".to_string()],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            presets: vec!["ralphx-coder".to_string()],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert!(capped.presets.is_empty());
    }

    #[test]
    fn test_validate_child_team_config_caps_timeout() {
        let resolved = TeamConstraints {
            timeout_minutes: 60,
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            timeout_minutes: 30,
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.timeout_minutes, 30);
    }

    #[test]
    fn test_validate_child_team_config_caps_budget_some_vs_some() {
        let resolved = TeamConstraints {
            budget_limit: Some(50.0),
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            budget_limit: Some(30.0),
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.budget_limit, Some(30.0));
    }

    #[test]
    fn test_validate_child_team_config_caps_budget_some_vs_none() {
        // If ceiling has no budget limit, pass through resolved
        let resolved = TeamConstraints {
            budget_limit: Some(50.0),
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            budget_limit: None,
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.budget_limit, Some(50.0));
    }

    #[test]
    fn test_validate_child_team_config_caps_budget_none_vs_some() {
        // If resolved has no budget but ceiling does, use None (no budget is more restrictive)
        let resolved = TeamConstraints {
            budget_limit: None,
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            budget_limit: Some(30.0),
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.budget_limit, None);
    }

    #[test]
    fn test_validate_child_team_config_empty_ceiling_tools_passes_resolved() {
        // Empty ceiling tools = no restriction, pass through resolved
        let resolved = TeamConstraints {
            allowed_tools: vec!["Read".to_string(), "Write".to_string()],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            allowed_tools: vec![],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert_eq!(capped.allowed_tools, vec!["Read", "Write"]);
    }

    #[test]
    fn test_validate_child_team_config_empty_resolved_tools_gets_empty() {
        // Empty resolved tools = nothing to intersect
        let resolved = TeamConstraints {
            allowed_tools: vec![],
            ..TeamConstraints::default()
        };
        let ceiling = TeamConstraints {
            allowed_tools: vec!["Read".to_string()],
            ..TeamConstraints::default()
        };
        let capped = validate_child_team_config(&resolved, &ceiling);
        assert!(capped.allowed_tools.is_empty());
    }

    #[test]
    fn test_validate_child_team_config_all_fields_capped() {
        let resolved = TeamConstraints {
            max_teammates: 10,
            allowed_tools: vec!["Read".to_string(), "Write".to_string(), "Edit".to_string()],
            allowed_mcp_tools: vec!["get_task_context".to_string(), "start_step".to_string()],
            model_cap: "opus".to_string(),
            mode: TeamMode::Dynamic,
            presets: vec!["ralphx-coder".to_string(), "ralphx-reviewer".to_string()],
            timeout_minutes: 60,
            budget_limit: Some(100.0),
        };
        let ceiling = TeamConstraints {
            max_teammates: 3,
            allowed_tools: vec!["Read".to_string(), "Write".to_string()],
            allowed_mcp_tools: vec!["get_task_context".to_string()],
            model_cap: "sonnet".to_string(),
            mode: TeamMode::Constrained,
            presets: vec!["ralphx-coder".to_string()],
            timeout_minutes: 30,
            budget_limit: Some(25.0),
        };
        let capped = validate_child_team_config(&resolved, &ceiling);

        assert_eq!(capped.max_teammates, 3);
        assert_eq!(capped.allowed_tools, vec!["Read", "Write"]);
        assert_eq!(capped.allowed_mcp_tools, vec!["get_task_context"]);
        assert_eq!(capped.model_cap, "sonnet");
        // mode is NOT capped - it's inherited from resolved
        assert_eq!(capped.mode, TeamMode::Dynamic);
        assert_eq!(capped.presets, vec!["ralphx-coder"]);
        assert_eq!(capped.timeout_minutes, 30);
        assert_eq!(capped.budget_limit, Some(25.0));
    }

    #[test]
    fn test_validate_child_team_config_equal_constraints_unchanged() {
        let resolved = TeamConstraints {
            max_teammates: 5,
            allowed_tools: vec!["Read".to_string()],
            model_cap: "sonnet".to_string(),
            timeout_minutes: 30,
            budget_limit: Some(50.0),
            ..TeamConstraints::default()
        };
        let ceiling = resolved.clone();
        let capped = validate_child_team_config(&resolved, &ceiling);

        assert_eq!(capped.max_teammates, 5);
        assert_eq!(capped.allowed_tools, vec!["Read"]);
        assert_eq!(capped.model_cap, "sonnet");
        assert_eq!(capped.timeout_minutes, 30);
        assert_eq!(capped.budget_limit, Some(50.0));
    }

    // ── Integration: full YAML with process_mapping + team_constraints ──

    #[test]
    fn test_full_yaml_roundtrip() {
        let yaml = r#"
process_mapping:
  ideation:
    default: orchestrator-ideation
    readonly: orchestrator-ideation-readonly
    team: ideation-team-lead
  execution:
    default: ralphx-worker
    team: ralphx-worker-team
  merge:
    default: ralphx-merger

team_constraints:
  _defaults:
    max_teammates: 5
    model_cap: sonnet
    mode: dynamic
    timeout_minutes: 20
  execution:
    max_teammates: 5
    allowed_tools: [Read, Write, Edit, Bash]
    allowed_mcp_tools: [get_task_context, start_step]
    model_cap: sonnet
    mode: dynamic
    presets: [ralphx-coder]
    timeout_minutes: 30
  review:
    max_teammates: 2
    mode: constrained
    presets: [ralphx-reviewer]
"#;

        #[derive(Debug, Deserialize)]
        struct TestConfig {
            #[serde(default)]
            process_mapping: ProcessMapping,
            #[serde(default)]
            team_constraints: TeamConstraintsConfig,
        }

        let config: TestConfig = serde_yaml::from_str(yaml).unwrap();

        // Process mapping
        assert_eq!(config.process_mapping.slots.len(), 3);
        let ideation = &config.process_mapping.slots["ideation"];
        assert_eq!(ideation.default, "orchestrator-ideation");
        assert_eq!(ideation.variants.get("team").unwrap(), "ideation-team-lead");
        assert_eq!(ideation.variants.get("readonly").unwrap(), "orchestrator-ideation-readonly");

        // Team constraints
        let defaults = config.team_constraints.defaults.as_ref().unwrap();
        assert_eq!(defaults.max_teammates, 5);
        assert_eq!(defaults.mode, TeamMode::Dynamic);

        let exec = &config.team_constraints.processes["execution"];
        assert_eq!(exec.timeout_minutes, 30);
        assert_eq!(exec.allowed_tools.len(), 4);

        let review = &config.team_constraints.processes["review"];
        assert_eq!(review.mode, TeamMode::Constrained);
        assert_eq!(review.max_teammates, 2);

        // Test constraint resolution
        let exec_constraints = get_team_constraints(&config.team_constraints, "execution");
        assert_eq!(exec_constraints.max_teammates, 5);
        assert_eq!(exec_constraints.timeout_minutes, 30);

        // Ideation falls back to _defaults
        let ideation_constraints = get_team_constraints(&config.team_constraints, "ideation");
        assert_eq!(ideation_constraints.max_teammates, 5);
        assert_eq!(ideation_constraints.timeout_minutes, 20);

        // Process agent resolution
        assert_eq!(
            resolve_process_agent(&config.process_mapping, "execution", "team"),
            Some("ralphx-worker-team".to_string())
        );
        assert_eq!(
            resolve_process_agent(&config.process_mapping, "execution", "default"),
            Some("ralphx-worker".to_string())
        );
    }

    // ── Test helpers ────────────────────────────────────────────────

    fn default_spawn_request() -> TeammateSpawnRequest {
        TeammateSpawnRequest {
            role: String::new(),
            prompt: None,
            preset: None,
            tools: Vec::new(),
            mcp_tools: Vec::new(),
            model: "sonnet".to_string(),
            prompt_summary: None,
        }
    }

    fn build_test_mapping() -> ProcessMapping {
        let mut slots = HashMap::new();
        slots.insert(
            "execution".to_string(),
            ProcessSlot {
                default: "ralphx-worker".to_string(),
                variants: {
                    let mut v = HashMap::new();
                    v.insert("team".to_string(), "ralphx-worker-team".to_string());
                    v
                },
            },
        );
        slots.insert(
            "ideation".to_string(),
            ProcessSlot {
                default: "orchestrator-ideation".to_string(),
                variants: {
                    let mut v = HashMap::new();
                    v.insert("team".to_string(), "ideation-team-lead".to_string());
                    v.insert(
                        "readonly".to_string(),
                        "orchestrator-ideation-readonly".to_string(),
                    );
                    v
                },
            },
        );
        ProcessMapping { slots }
    }
}
