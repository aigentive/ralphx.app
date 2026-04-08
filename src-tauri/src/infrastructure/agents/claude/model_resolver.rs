// Ideation model resolution for Claude agent spawns.
//
// Resolves the `--model` value for ideation agents using a 4-level priority chain:
// per-project DB row → global DB row → YAML agent config → hardcoded default ("sonnet").

use crate::domain::ideation::model_settings::{model_bucket_for_agent, ModelLevel};
use crate::domain::repositories::IdeationModelSettingsRepository;

/// The resolved model string and its resolution source.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModel {
    /// The resolved model string (e.g. "sonnet", "opus", "haiku").
    pub model: String,
    /// Where this model came from: "user" | "global" | "yaml" | "default".
    pub source: String,
}

/// Resolve the `--model` value for an ideation agent using a 4-level chain:
///
/// 1. Per-project DB row for `project_id` (if `Some`) — bucket model if not `Inherit`
/// 2. Global DB row (`project_id = NULL`) — bucket model if not `Inherit`
/// 3. YAML agent-level config (`AgentConfig.model`)
/// 4. Hardcoded default: `"sonnet"`
///
/// If the agent is not an ideation agent (bucket = `None`), falls through directly
/// to `resolve_model_with_source(Some(agent_name))` (levels 3–4).
pub async fn resolve_ideation_model(
    agent_name: &str,
    project_id: Option<&str>,
    repo: &dyn IdeationModelSettingsRepository,
) -> ResolvedModel {
    let bucket = match model_bucket_for_agent(agent_name) {
        Some(b) => b,
        None => {
            let (model, source) = resolve_model_with_source(Some(agent_name));
            return ResolvedModel { model, source };
        }
    };

    // Level 1: per-project override
    if let Some(pid) = project_id {
        if let Ok(Some(settings)) = repo.get_for_project(pid).await {
            let level = settings.model_for_bucket(&bucket);
            if *level != ModelLevel::Inherit {
                return ResolvedModel {
                    model: level.to_string(),
                    source: "user".to_string(),
                };
            }
        }
    }

    // Level 2: global row
    if let Ok(Some(settings)) = repo.get_global().await {
        let level = settings.model_for_bucket(&bucket);
        if *level != ModelLevel::Inherit {
            return ResolvedModel {
                model: level.to_string(),
                source: "global".to_string(),
            };
        }
    }

    // Levels 3–4: YAML agent config + hardcoded default
    let (model, source) = resolve_model_with_source(Some(agent_name));
    ResolvedModel { model, source }
}

/// Resolve the effective verifier-subagent model string and its source label.
///
/// 3-level fallback chain (no YAML level — hardcoded default is always "haiku"):
///   1. project_value (if Some and != inherit) → source "user"
///   2. global_value  (if Some and != inherit) → source "global"
///   3. Hardcoded default "haiku"               → source "default"
pub fn resolve_verifier_subagent_model_with_source(
    project_value: Option<&ModelLevel>,
    global_value: Option<&ModelLevel>,
) -> (String, String) {
    resolve_model_with_fallback(project_value, global_value, None, "haiku")
}

/// Resolve the effective ideation-subagent model string and its source label.
///
/// 3-level fallback chain (no YAML level — hardcoded default is always "haiku"):
///   1. project_value (if Some and != inherit) → source "user"
///   2. global_value  (if Some and != inherit) → source "global"
///   3. Hardcoded default "haiku"               → source "default"
pub fn resolve_ideation_subagent_model_with_source(
    project_value: Option<&ModelLevel>,
    global_value: Option<&ModelLevel>,
) -> (String, String) {
    resolve_model_with_fallback(project_value, global_value, None, "haiku")
}

/// Shared resolution helper: project → global → yaml → fallback.
///
/// Returns the first non-inherit value found, with its source label:
///   "user"    — from project_value
///   "global"  — from global_value
///   "yaml"    — from yaml_value (when Some)
///   "default" — hardcoded fallback
fn resolve_model_with_fallback(
    project_value: Option<&ModelLevel>,
    global_value: Option<&ModelLevel>,
    yaml_value: Option<&str>,
    fallback: &str,
) -> (String, String) {
    // Level 1 — per-project row
    if let Some(level) = project_value {
        if *level != ModelLevel::Inherit {
            return (level.to_string(), "user".to_string());
        }
    }

    // Level 2 — global row
    if let Some(level) = global_value {
        if *level != ModelLevel::Inherit {
            return (level.to_string(), "global".to_string());
        }
    }

    // Level 3 — YAML agent config
    if let Some(m) = yaml_value {
        return (m.to_string(), "yaml".to_string());
    }

    // Level 4 — hardcoded fallback
    (fallback.to_string(), "default".to_string())
}

/// Resolve model from YAML config and return (model, source).
///
/// Returns `(yaml_model, "yaml")` if an explicit YAML model is configured for the agent,
/// or `("sonnet", "default")` as the hardcoded fallback.
pub fn resolve_model_with_source(agent_type: Option<&str>) -> (String, String) {
    use super::get_agent_config;
    let yaml_model = agent_type
        .and_then(|n| get_agent_config(n))
        .and_then(|c| c.model.clone());
    resolve_model_with_fallback(None, None, yaml_model.as_deref(), "sonnet")
}

#[cfg(test)]
#[path = "model_resolver_tests.rs"]
mod tests;
