use super::*;

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
