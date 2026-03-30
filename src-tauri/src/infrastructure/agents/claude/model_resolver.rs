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
    /// Where this model came from: "user" | "global" | "yaml" | "yaml_default".
    pub source: &'static str,
}

/// Resolve the `--model` value for an ideation agent using a 4-level chain:
///
/// 1. Per-project DB row for `project_id` (if `Some`) — bucket model if not `Inherit`
/// 2. Global DB row (`project_id = NULL`) — bucket model if not `Inherit`
/// 3. YAML agent-level config (`AgentConfig.model`)
/// 4. Hardcoded default: `"sonnet"`
///
/// If the agent is not an ideation agent (bucket = `None`), falls through directly
/// to `resolve_model(Some(agent_name))` (levels 3–4).
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
                    source: "user",
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
                source: "global",
            };
        }
    }

    // Levels 3–4: YAML agent config + hardcoded default
    let (model, source) = resolve_model_with_source(Some(agent_name));
    ResolvedModel { model, source }
}

/// Internal helper: resolve model from YAML config and return (model, source).
fn resolve_model_with_source(agent_type: Option<&str>) -> (String, &'static str) {
    use super::get_agent_config;
    // Check if there is an explicit YAML agent model config for this agent.
    let yaml_model = agent_type
        .and_then(|n| get_agent_config(n))
        .and_then(|c| c.model.clone());
    if let Some(m) = yaml_model {
        (m, "yaml")
    } else {
        // No agent-level YAML model → use hardcoded default
        ("sonnet".to_string(), "yaml_default")
    }
}

#[cfg(test)]
#[path = "model_resolver_tests.rs"]
mod tests;
