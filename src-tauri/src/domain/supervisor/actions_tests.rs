use super::*;

use super::*;

#[test]
fn test_severity_display() {
    assert_eq!(Severity::Low.to_string(), "low");
    assert_eq!(Severity::Critical.to_string(), "critical");
}

#[test]
fn test_severity_ordering() {
    assert!(Severity::Low < Severity::Medium);
    assert!(Severity::Medium < Severity::High);
    assert!(Severity::High < Severity::Critical);
}

#[test]
fn test_supervisor_action_log() {
    let action = SupervisorAction::log(Severity::Medium, "Test warning");
    assert_eq!(action.severity(), Severity::Medium);
    assert!(!action.is_intervention());
}

#[test]
fn test_supervisor_action_inject_guidance() {
    let action = SupervisorAction::inject_guidance("Try something else");
    assert_eq!(action.severity(), Severity::Medium);
    assert!(action.is_intervention());
}

#[test]
fn test_supervisor_action_pause() {
    let action = SupervisorAction::pause("Need human input");
    assert_eq!(action.severity(), Severity::High);
    assert!(action.is_intervention());
}

#[test]
fn test_supervisor_action_kill() {
    let action = SupervisorAction::kill("Fatal error", "Task cannot continue");
    assert_eq!(action.severity(), Severity::Critical);
    assert!(action.is_intervention());
}

#[test]
fn test_supervisor_action_none() {
    let action = SupervisorAction::None;
    assert_eq!(action.severity(), Severity::Low);
    assert!(!action.is_intervention());
}

#[test]
fn test_action_for_detection_loop_high() {
    let detection = DetectionResult::new(Pattern::InfiniteLoop, 95, "Test", 6);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::Kill { .. }));
}

#[test]
fn test_action_for_detection_loop_medium() {
    let detection = DetectionResult::new(Pattern::InfiniteLoop, 80, "Test", 4);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::Pause { .. }));
}

#[test]
fn test_action_for_detection_loop_low() {
    let detection = DetectionResult::new(Pattern::InfiniteLoop, 70, "Test", 3);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::InjectGuidance { .. }));
}

#[test]
fn test_action_for_detection_stuck() {
    let detection = DetectionResult::new(Pattern::Stuck, 80, "Test", 10);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::Kill { .. }));
}

#[test]
fn test_action_for_detection_poor_task() {
    let detection = DetectionResult::new(Pattern::PoorTaskDefinition, 90, "Test", 5);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::Pause { .. }));
}

#[test]
fn test_action_for_detection_repeating_error() {
    let detection = DetectionResult::new(Pattern::RepeatingError, 80, "Test", 4);
    let action = action_for_detection(&detection);
    assert!(matches!(action, SupervisorAction::Pause { .. }));
}

#[test]
fn test_action_for_severity() {
    assert!(matches!(
        action_for_severity(Severity::Low, "test"),
        SupervisorAction::Log { .. }
    ));
    assert!(matches!(
        action_for_severity(Severity::Medium, "test"),
        SupervisorAction::InjectGuidance { .. }
    ));
    assert!(matches!(
        action_for_severity(Severity::High, "test"),
        SupervisorAction::Pause { .. }
    ));
    assert!(matches!(
        action_for_severity(Severity::Critical, "test"),
        SupervisorAction::Kill { .. }
    ));
}

#[test]
fn test_supervisor_action_serialize() {
    let action = SupervisorAction::pause("Testing");
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"pause\""));
    assert!(json.contains("\"reason\":\"Testing\""));
}

#[test]
fn test_supervisor_action_deserialize() {
    let json = r#"{"type": "inject_guidance", "message": "Try again"}"#;
    let action: SupervisorAction = serde_json::from_str(json).unwrap();
    assert!(
        matches!(action, SupervisorAction::InjectGuidance { message } if message == "Try again")
    );
}

#[test]
fn test_severity_serialize() {
    let json = serde_json::to_string(&Severity::High).unwrap();
    assert_eq!(json, "\"high\"");
}

#[test]
fn test_severity_deserialize() {
    let severity: Severity = serde_json::from_str("\"critical\"").unwrap();
    assert_eq!(severity, Severity::Critical);
}
