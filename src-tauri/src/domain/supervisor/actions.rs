// Supervisor actions
// Actions that the supervisor can take in response to detected patterns

use serde::{Deserialize, Serialize};
use super::patterns::{DetectionResult, Pattern};

/// Severity level of an issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Minor issue, log and continue
    Low,
    /// Notable issue, may need attention
    Medium,
    /// Serious issue, requires intervention
    High,
    /// Critical issue, must stop immediately
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Medium => write!(f, "medium"),
            Severity::High => write!(f, "high"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// Actions the supervisor can take
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SupervisorAction {
    /// Log a warning and continue monitoring
    Log {
        severity: Severity,
        message: String,
    },
    /// Inject guidance into agent context
    InjectGuidance {
        message: String,
    },
    /// Pause the task execution
    Pause {
        reason: String,
    },
    /// Kill the task and mark as failed
    Kill {
        reason: String,
        analysis: String,
    },
    /// No action needed
    None,
}

impl SupervisorAction {
    /// Create a Log action
    pub fn log(severity: Severity, message: impl Into<String>) -> Self {
        Self::Log {
            severity,
            message: message.into(),
        }
    }

    /// Create an InjectGuidance action
    pub fn inject_guidance(message: impl Into<String>) -> Self {
        Self::InjectGuidance {
            message: message.into(),
        }
    }

    /// Create a Pause action
    pub fn pause(reason: impl Into<String>) -> Self {
        Self::Pause {
            reason: reason.into(),
        }
    }

    /// Create a Kill action
    pub fn kill(reason: impl Into<String>, analysis: impl Into<String>) -> Self {
        Self::Kill {
            reason: reason.into(),
            analysis: analysis.into(),
        }
    }

    /// Get the severity of this action
    pub fn severity(&self) -> Severity {
        match self {
            Self::None => Severity::Low,
            Self::Log { severity, .. } => *severity,
            Self::InjectGuidance { .. } => Severity::Medium,
            Self::Pause { .. } => Severity::High,
            Self::Kill { .. } => Severity::Critical,
        }
    }

    /// Check if this is an intervention action (not just logging)
    pub fn is_intervention(&self) -> bool {
        matches!(self, Self::InjectGuidance { .. } | Self::Pause { .. } | Self::Kill { .. })
    }
}

/// Determine the appropriate action for a detection result
pub fn action_for_detection(detection: &DetectionResult) -> SupervisorAction {
    let confidence = detection.confidence;
    let occurrences = detection.occurrences;

    match detection.pattern {
        Pattern::InfiniteLoop => {
            if confidence >= 90 && occurrences >= 5 {
                SupervisorAction::kill(
                    "Infinite loop detected with high confidence",
                    format!("Tool called {} times with identical arguments. Pattern: {}",
                        occurrences, detection.description),
                )
            } else if confidence >= 80 || occurrences >= 4 {
                SupervisorAction::pause(format!(
                    "Possible infinite loop: {} (confidence: {}%)",
                    detection.description, confidence
                ))
            } else if confidence >= 70 {
                SupervisorAction::inject_guidance(
                    "You may be repeating the same action. Try a different approach or verify your progress."
                )
            } else {
                SupervisorAction::log(
                    Severity::Low,
                    format!("Possible loop pattern: {}", detection.description),
                )
            }
        }
        Pattern::Stuck => {
            if occurrences >= 10 {
                SupervisorAction::kill(
                    "Agent appears completely stuck",
                    format!("No progress for {} consecutive checks. The task may be impossible or poorly defined.",
                        occurrences),
                )
            } else if occurrences >= 7 {
                SupervisorAction::pause(format!(
                    "Agent stuck for {} checks. Manual intervention may be needed.",
                    occurrences
                ))
            } else if occurrences >= 5 {
                SupervisorAction::inject_guidance(
                    "Progress appears stalled. Consider: 1) Breaking the task into smaller steps, 2) Trying a different approach, 3) Requesting clarification."
                )
            } else {
                SupervisorAction::log(
                    Severity::Low,
                    format!("Progress slow: {}", detection.description),
                )
            }
        }
        Pattern::PoorTaskDefinition => {
            if confidence >= 90 {
                SupervisorAction::pause(
                    "Task definition appears too vague or incomplete. Please provide clearer requirements."
                )
            } else if confidence >= 80 {
                SupervisorAction::inject_guidance(
                    "The task requirements may be unclear. Consider asking for more specific acceptance criteria."
                )
            } else {
                SupervisorAction::log(
                    Severity::Medium,
                    format!("Possible task definition issue: {}", detection.description),
                )
            }
        }
        Pattern::RepeatingError => {
            if occurrences >= 4 {
                SupervisorAction::pause(format!(
                    "Same error occurring repeatedly ({} times). The issue may not be resolvable with current approach.",
                    occurrences
                ))
            } else if occurrences >= 3 {
                SupervisorAction::inject_guidance(
                    "The same error keeps occurring. Try a fundamentally different approach to solve the problem."
                )
            } else {
                SupervisorAction::log(
                    Severity::Medium,
                    format!("Repeating error: {}", detection.description),
                )
            }
        }
    }
}

/// Determine action based on severity level alone
pub fn action_for_severity(severity: Severity, message: impl Into<String>) -> SupervisorAction {
    let msg = message.into();
    match severity {
        Severity::Low => SupervisorAction::log(Severity::Low, msg),
        Severity::Medium => SupervisorAction::inject_guidance(msg),
        Severity::High => SupervisorAction::pause(msg),
        Severity::Critical => SupervisorAction::kill(msg.clone(), msg),
    }
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(action_for_severity(Severity::Low, "test"), SupervisorAction::Log { .. }));
        assert!(matches!(action_for_severity(Severity::Medium, "test"), SupervisorAction::InjectGuidance { .. }));
        assert!(matches!(action_for_severity(Severity::High, "test"), SupervisorAction::Pause { .. }));
        assert!(matches!(action_for_severity(Severity::Critical, "test"), SupervisorAction::Kill { .. }));
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
        assert!(matches!(action, SupervisorAction::InjectGuidance { message } if message == "Try again"));
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
}
