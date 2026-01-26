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
}

impl Default for IdeationSettings {
    fn default() -> Self {
        Self {
            plan_mode: IdeationPlanMode::Optional,
            require_plan_approval: false, // Plan existence is sufficient by default
            suggest_plans_for_complex: true,
            auto_link_proposals: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideation_plan_mode_default() {
        assert_eq!(IdeationPlanMode::default(), IdeationPlanMode::Optional);
    }

    #[test]
    fn test_ideation_settings_default() {
        let settings = IdeationSettings::default();
        assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
        assert!(!settings.require_plan_approval);
        assert!(settings.suggest_plans_for_complex);
        assert!(settings.auto_link_proposals);
    }

    #[test]
    fn test_ideation_plan_mode_serialization() {
        let mode = IdeationPlanMode::Required;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"required\"");

        let mode = IdeationPlanMode::Optional;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"optional\"");

        let mode = IdeationPlanMode::Parallel;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"parallel\"");
    }

    #[test]
    fn test_ideation_settings_serialization() {
        let settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Required,
            require_plan_approval: true,
            suggest_plans_for_complex: false,
            auto_link_proposals: false,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: IdeationSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.plan_mode, IdeationPlanMode::Required);
        assert!(deserialized.require_plan_approval);
        assert!(!deserialized.suggest_plans_for_complex);
        assert!(!deserialized.auto_link_proposals);
    }
}
