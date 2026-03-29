use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeationPlanMode {
    /// Plan must exist before proposals can be created
    Required,
    /// Plan is optional, orchestrator suggests for complex features
    Optional,
    /// Plan and proposals created together, changes suggest sync
    Parallel,
}

impl Default for IdeationPlanMode {
    fn default() -> Self {
        Self::Optional
    }
}

/// Ideation-specific settings (separate from QA settings)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdeationSettings {
    /// How implementation plans are created in ideation flow
    pub plan_mode: IdeationPlanMode,
    /// In Required mode, whether explicit approval is needed before proposals
    pub require_plan_approval: bool,
    /// Whether to show plan suggestions for complex features (in Optional mode)
    pub suggest_plans_for_complex: bool,
    /// Auto-link proposals to session plan when created
    pub auto_link_proposals: bool,
    /// If true, plans must be verified (or skipped) before accepting proposals
    #[serde(default)]
    pub require_verification_for_accept: bool,
    /// If true, plans must be verified (or skipped) before proposals can be created
    #[serde(default)]
    pub require_verification_for_proposals: bool,
    /// If true, finalize_proposals pauses for human acceptance before applying proposals
    #[serde(default)]
    pub require_accept_for_finalize: bool,
}

impl Default for IdeationSettings {
    fn default() -> Self {
        Self {
            plan_mode: IdeationPlanMode::Optional,
            require_plan_approval: false, // Plan existence is sufficient by default
            suggest_plans_for_complex: true,
            auto_link_proposals: true,
            require_verification_for_accept: false, // Opt-in feature
            require_verification_for_proposals: false, // Opt-in feature
            require_accept_for_finalize: false, // Opt-in feature
        }
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
