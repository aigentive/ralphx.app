// Tauri commands for getting and updating ideation effort settings.
//
// Resolution chain (4 levels, first non-inherit wins):
//   1. Per-project DB row (if project_id is Some and row exists and value != inherit)
//   2. Global DB row (if row exists and value != inherit)
//   3. YAML agent-specific effort (if AgentConfig.effort is Some)
//   4. YAML default_effort (ClaudeRuntimeConfig.default_effort)

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

use crate::application::AppState;
use crate::domain::ideation::EffortLevel;
use crate::infrastructure::agents::claude::{claude_runtime_config, get_agent_config};

// Representative agents for each bucket — used to resolve YAML effort values.
const PRIMARY_REPR_AGENT: &str = "orchestrator-ideation";
const VERIFIER_REPR_AGENT: &str = "plan-verifier";

// ============================================================================
// Response type
// ============================================================================

/// Response returned by both get and update commands.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeationEffortResponse {
    /// Stored value for the primary bucket (may be "inherit").
    pub primary_effort: String,
    /// Stored value for the verifier bucket (may be "inherit").
    pub verifier_effort: String,
    /// Resolved effective effort for the primary bucket (never "inherit").
    pub effective_primary: String,
    /// Resolved effective effort for the verifier bucket (never "inherit").
    pub effective_verifier: String,
    /// Source label for how effective_primary was determined.
    /// One of: "user" | "global" | "yaml" | "yaml_default"
    pub primary_source: String,
    /// Source label for how effective_verifier was determined.
    /// One of: "user" | "global" | "yaml" | "yaml_default"
    pub verifier_source: String,
}

// ============================================================================
// Input type
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIdeationEffortInput {
    /// None = update global row; Some = update per-project row.
    pub project_id: Option<String>,
    /// New primary effort level. Only updated if provided.
    pub primary_effort: Option<String>,
    /// New verifier effort level. Only updated if provided.
    pub verifier_effort: Option<String>,
}

// ============================================================================
// Resolution helpers
// ============================================================================

/// Resolve the effective effort string and its source label for one bucket.
///
/// The 4-level chain:
///   1. project_value (if Some and != inherit) → source "user"
///   2. global_value  (if Some and != inherit) → source "global"
///   3. YAML agent-specific effort              → source "yaml"
///   4. YAML default_effort                     → source "yaml_default"
fn resolve_effort_with_source(
    project_value: Option<&EffortLevel>,
    global_value: Option<&EffortLevel>,
    repr_agent: &str,
) -> (String, String) {
    // Level 1 — per-project row
    if let Some(level) = project_value {
        if *level != EffortLevel::Inherit {
            return (level.to_string(), "user".to_string());
        }
    }

    // Level 2 — global row
    if let Some(level) = global_value {
        if *level != EffortLevel::Inherit {
            return (level.to_string(), "global".to_string());
        }
    }

    // Level 3 — YAML agent-specific effort
    if let Some(config) = get_agent_config(repr_agent) {
        if let Some(effort) = &config.effort {
            return (effort.clone(), "yaml".to_string());
        }
    }

    // Level 4 — YAML default_effort
    let default = claude_runtime_config().default_effort.clone();
    (default, "yaml_default".to_string())
}

// ============================================================================
// Commands
// ============================================================================

/// Get ideation effort settings for a project (or global if project_id is None).
///
/// Returns both the stored values and the resolved effective values with source labels.
///
/// # Errors
///
/// Returns a string error if the repository lookup fails.
#[tauri::command]
pub async fn get_ideation_effort_settings(
    project_id: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<IdeationEffortResponse, String> {
    // Fetch per-project row (if project_id is Some)
    let project_row = if project_id.is_some() {
        app_state
            .ideation_effort_settings_repo
            .get_by_project_id(project_id.as_deref())
            .await
            .map_err(|e| format!("Failed to fetch per-project effort settings: {e}"))?
    } else {
        None
    };

    // Fetch global row
    let global_row = app_state
        .ideation_effort_settings_repo
        .get_by_project_id(None)
        .await
        .map_err(|e| format!("Failed to fetch global effort settings: {e}"))?;

    // Stored values come from the project row when present; otherwise global row; otherwise inherit.
    let stored_primary = project_row
        .as_ref()
        .map(|r| r.primary_effort.to_string())
        .or_else(|| global_row.as_ref().map(|r| r.primary_effort.to_string()))
        .unwrap_or_else(|| EffortLevel::Inherit.to_string());

    let stored_verifier = project_row
        .as_ref()
        .map(|r| r.verifier_effort.to_string())
        .or_else(|| global_row.as_ref().map(|r| r.verifier_effort.to_string()))
        .unwrap_or_else(|| EffortLevel::Inherit.to_string());

    // Resolve effective values
    let project_primary = project_row.as_ref().map(|r| &r.primary_effort);
    let project_verifier = project_row.as_ref().map(|r| &r.verifier_effort);
    let global_primary = global_row.as_ref().map(|r| &r.primary_effort);
    let global_verifier = global_row.as_ref().map(|r| &r.verifier_effort);

    let (effective_primary, primary_source) =
        resolve_effort_with_source(project_primary, global_primary, PRIMARY_REPR_AGENT);
    let (effective_verifier, verifier_source) =
        resolve_effort_with_source(project_verifier, global_verifier, VERIFIER_REPR_AGENT);

    Ok(IdeationEffortResponse {
        primary_effort: stored_primary,
        verifier_effort: stored_verifier,
        effective_primary,
        effective_verifier,
        primary_source,
        verifier_source,
    })
}

/// Update ideation effort settings for a project (or global if project_id is None).
///
/// Only provided fields are updated; omitted fields fall back to the current stored value
/// (or "inherit" if no row exists yet).
///
/// # Errors
///
/// Returns a string error if any provided effort value is invalid or the upsert fails.
#[tauri::command]
pub async fn update_ideation_effort_settings(
    input: UpdateIdeationEffortInput,
    app_state: State<'_, AppState>,
) -> Result<IdeationEffortResponse, String> {
    // Validate any provided values upfront.
    if let Some(ref v) = input.primary_effort {
        EffortLevel::from_str(v).map_err(|e| format!("Invalid primaryEffort: {e}"))?;
    }
    if let Some(ref v) = input.verifier_effort {
        EffortLevel::from_str(v).map_err(|e| format!("Invalid verifierEffort: {e}"))?;
    }

    // Fetch the existing row so we can merge (keep old values for unspecified fields).
    let existing = app_state
        .ideation_effort_settings_repo
        .get_by_project_id(input.project_id.as_deref())
        .await
        .map_err(|e| format!("Failed to fetch current effort settings: {e}"))?;

    let current_primary = existing
        .as_ref()
        .map(|r| r.primary_effort.to_string())
        .unwrap_or_else(|| EffortLevel::Inherit.to_string());

    let current_verifier = existing
        .as_ref()
        .map(|r| r.verifier_effort.to_string())
        .unwrap_or_else(|| EffortLevel::Inherit.to_string());

    let new_primary = input.primary_effort.as_deref().unwrap_or(&current_primary);
    let new_verifier = input.verifier_effort.as_deref().unwrap_or(&current_verifier);

    // Upsert the row.
    app_state
        .ideation_effort_settings_repo
        .upsert(input.project_id.as_deref(), new_primary, new_verifier)
        .await
        .map_err(|e| format!("Failed to save effort settings: {e}"))?;

    // Return freshly-resolved values by delegating to the get command logic.
    get_ideation_effort_settings(input.project_id, app_state).await
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ideation::IdeationEffortSettings;
    use chrono::Utc;

    fn make_settings(primary: EffortLevel, verifier: EffortLevel) -> IdeationEffortSettings {
        IdeationEffortSettings {
            id: 1,
            project_id: None,
            primary_effort: primary,
            verifier_effort: verifier,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn resolve_effort_project_row_wins() {
        let project = make_settings(EffortLevel::Low, EffortLevel::High);
        let global = make_settings(EffortLevel::Max, EffortLevel::Max);

        let (eff, src) = resolve_effort_with_source(
            Some(&project.primary_effort),
            Some(&global.primary_effort),
            PRIMARY_REPR_AGENT,
        );
        assert_eq!(eff, "low");
        assert_eq!(src, "user");
    }

    #[test]
    fn resolve_effort_falls_through_to_global() {
        let project = make_settings(EffortLevel::Inherit, EffortLevel::Inherit);
        let global = make_settings(EffortLevel::Medium, EffortLevel::High);

        let (eff, src) = resolve_effort_with_source(
            Some(&project.primary_effort),
            Some(&global.primary_effort),
            PRIMARY_REPR_AGENT,
        );
        assert_eq!(eff, "medium");
        assert_eq!(src, "global");
    }

    #[test]
    fn resolve_effort_no_rows_uses_yaml_fallback() {
        // Both None → should reach YAML level (yaml or yaml_default).
        let (_, src) = resolve_effort_with_source(None, None, PRIMARY_REPR_AGENT);
        assert!(
            src == "yaml" || src == "yaml_default",
            "Expected yaml or yaml_default, got: {src}"
        );
    }

    #[test]
    fn resolve_effort_inherit_only_falls_through() {
        let settings = make_settings(EffortLevel::Inherit, EffortLevel::Inherit);

        // Inherit-only project row, no global row → YAML fallback
        let (_, src) =
            resolve_effort_with_source(Some(&settings.primary_effort), None, PRIMARY_REPR_AGENT);
        assert!(
            src == "yaml" || src == "yaml_default",
            "Expected yaml/yaml_default after inherit, got: {src}"
        );
    }
}
