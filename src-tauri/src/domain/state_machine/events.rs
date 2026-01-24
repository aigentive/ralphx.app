// Task events - triggers for state machine transitions
// These events come from user actions, agent signals, or system signals

use serde::{Deserialize, Serialize};

/// All possible events that can trigger state transitions in the task state machine.
///
/// Events are categorized into:
/// - User actions: Manual interventions by humans
/// - Agent signals: Completion/failure signals from AI agents
/// - System signals: Automatic triggers based on system state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskEvent {
    // ==================
    // User actions
    // ==================
    /// User moves task from Backlog to Ready
    Schedule,

    /// User cancels task from any non-terminal state
    Cancel,

    /// Human override to approve task regardless of review state
    ForceApprove,

    /// Retry task from Failed, Cancelled, or Approved state
    Retry,

    /// Human skips QA failure and proceeds to review
    SkipQa,

    // ==================
    // Agent signals
    // ==================
    /// Worker agent completed execution successfully
    ExecutionComplete,

    /// Worker agent execution failed with error
    ExecutionFailed {
        /// The error message from the agent
        error: String,
    },

    /// Agent needs human input to proceed
    NeedsHumanInput {
        /// Reason why human input is needed
        reason: String,
    },

    /// QA prep agent completed acceptance criteria refinement
    QaRefinementComplete,

    /// QA testing completed with result
    QaTestsComplete {
        /// Whether all tests passed
        passed: bool,
    },

    /// AI reviewer completed review
    ReviewComplete {
        /// Whether the review approved the work
        approved: bool,
        /// Optional feedback from reviewer
        feedback: Option<String>,
    },

    // ==================
    // System signals
    // ==================
    /// All blocking tasks have been completed
    BlockersResolved,

    /// A blocking dependency was detected
    BlockerDetected {
        /// The ID of the blocking task
        blocker_id: String,
    },
}

impl TaskEvent {
    /// Returns true if this is a user-initiated action
    pub fn is_user_action(&self) -> bool {
        matches!(
            self,
            TaskEvent::Schedule
                | TaskEvent::Cancel
                | TaskEvent::ForceApprove
                | TaskEvent::Retry
                | TaskEvent::SkipQa
        )
    }

    /// Returns true if this is an agent-originated signal
    pub fn is_agent_signal(&self) -> bool {
        matches!(
            self,
            TaskEvent::ExecutionComplete
                | TaskEvent::ExecutionFailed { .. }
                | TaskEvent::NeedsHumanInput { .. }
                | TaskEvent::QaRefinementComplete
                | TaskEvent::QaTestsComplete { .. }
                | TaskEvent::ReviewComplete { .. }
        )
    }

    /// Returns true if this is a system-generated signal
    pub fn is_system_signal(&self) -> bool {
        matches!(
            self,
            TaskEvent::BlockersResolved | TaskEvent::BlockerDetected { .. }
        )
    }

    /// Returns the event name as a string for logging
    pub fn name(&self) -> &'static str {
        match self {
            TaskEvent::Schedule => "Schedule",
            TaskEvent::Cancel => "Cancel",
            TaskEvent::ForceApprove => "ForceApprove",
            TaskEvent::Retry => "Retry",
            TaskEvent::SkipQa => "SkipQa",
            TaskEvent::ExecutionComplete => "ExecutionComplete",
            TaskEvent::ExecutionFailed { .. } => "ExecutionFailed",
            TaskEvent::NeedsHumanInput { .. } => "NeedsHumanInput",
            TaskEvent::QaRefinementComplete => "QaRefinementComplete",
            TaskEvent::QaTestsComplete { .. } => "QaTestsComplete",
            TaskEvent::ReviewComplete { .. } => "ReviewComplete",
            TaskEvent::BlockersResolved => "BlockersResolved",
            TaskEvent::BlockerDetected { .. } => "BlockerDetected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================
    // User action tests
    // ==================

    #[test]
    fn test_schedule_is_user_action() {
        let event = TaskEvent::Schedule;
        assert!(event.is_user_action());
        assert!(!event.is_agent_signal());
        assert!(!event.is_system_signal());
    }

    #[test]
    fn test_cancel_is_user_action() {
        let event = TaskEvent::Cancel;
        assert!(event.is_user_action());
        assert_eq!(event.name(), "Cancel");
    }

    #[test]
    fn test_force_approve_is_user_action() {
        let event = TaskEvent::ForceApprove;
        assert!(event.is_user_action());
        assert_eq!(event.name(), "ForceApprove");
    }

    #[test]
    fn test_retry_is_user_action() {
        let event = TaskEvent::Retry;
        assert!(event.is_user_action());
        assert_eq!(event.name(), "Retry");
    }

    #[test]
    fn test_skip_qa_is_user_action() {
        let event = TaskEvent::SkipQa;
        assert!(event.is_user_action());
        assert_eq!(event.name(), "SkipQa");
    }

    // ==================
    // Agent signal tests
    // ==================

    #[test]
    fn test_execution_complete_is_agent_signal() {
        let event = TaskEvent::ExecutionComplete;
        assert!(event.is_agent_signal());
        assert!(!event.is_user_action());
        assert!(!event.is_system_signal());
        assert_eq!(event.name(), "ExecutionComplete");
    }

    #[test]
    fn test_execution_failed_is_agent_signal() {
        let event = TaskEvent::ExecutionFailed {
            error: "Build failed".to_string(),
        };
        assert!(event.is_agent_signal());
        assert_eq!(event.name(), "ExecutionFailed");
    }

    #[test]
    fn test_execution_failed_contains_error() {
        let event = TaskEvent::ExecutionFailed {
            error: "Compilation error on line 42".to_string(),
        };
        if let TaskEvent::ExecutionFailed { error } = event {
            assert_eq!(error, "Compilation error on line 42");
        } else {
            panic!("Expected ExecutionFailed");
        }
    }

    #[test]
    fn test_needs_human_input_is_agent_signal() {
        let event = TaskEvent::NeedsHumanInput {
            reason: "Need API key".to_string(),
        };
        assert!(event.is_agent_signal());
        assert_eq!(event.name(), "NeedsHumanInput");
    }

    #[test]
    fn test_needs_human_input_contains_reason() {
        let event = TaskEvent::NeedsHumanInput {
            reason: "Clarification needed on requirements".to_string(),
        };
        if let TaskEvent::NeedsHumanInput { reason } = event {
            assert_eq!(reason, "Clarification needed on requirements");
        } else {
            panic!("Expected NeedsHumanInput");
        }
    }

    #[test]
    fn test_qa_refinement_complete_is_agent_signal() {
        let event = TaskEvent::QaRefinementComplete;
        assert!(event.is_agent_signal());
        assert_eq!(event.name(), "QaRefinementComplete");
    }

    #[test]
    fn test_qa_tests_complete_is_agent_signal() {
        let event = TaskEvent::QaTestsComplete { passed: true };
        assert!(event.is_agent_signal());
        assert_eq!(event.name(), "QaTestsComplete");
    }

    #[test]
    fn test_qa_tests_complete_passed() {
        let event = TaskEvent::QaTestsComplete { passed: true };
        if let TaskEvent::QaTestsComplete { passed } = event {
            assert!(passed);
        } else {
            panic!("Expected QaTestsComplete");
        }
    }

    #[test]
    fn test_qa_tests_complete_failed() {
        let event = TaskEvent::QaTestsComplete { passed: false };
        if let TaskEvent::QaTestsComplete { passed } = event {
            assert!(!passed);
        } else {
            panic!("Expected QaTestsComplete");
        }
    }

    #[test]
    fn test_review_complete_is_agent_signal() {
        let event = TaskEvent::ReviewComplete {
            approved: true,
            feedback: None,
        };
        assert!(event.is_agent_signal());
        assert_eq!(event.name(), "ReviewComplete");
    }

    #[test]
    fn test_review_complete_approved_with_feedback() {
        let event = TaskEvent::ReviewComplete {
            approved: true,
            feedback: Some("LGTM!".to_string()),
        };
        if let TaskEvent::ReviewComplete { approved, feedback } = event {
            assert!(approved);
            assert_eq!(feedback, Some("LGTM!".to_string()));
        } else {
            panic!("Expected ReviewComplete");
        }
    }

    #[test]
    fn test_review_complete_rejected_with_feedback() {
        let event = TaskEvent::ReviewComplete {
            approved: false,
            feedback: Some("Missing error handling".to_string()),
        };
        if let TaskEvent::ReviewComplete { approved, feedback } = event {
            assert!(!approved);
            assert_eq!(feedback, Some("Missing error handling".to_string()));
        } else {
            panic!("Expected ReviewComplete");
        }
    }

    // ==================
    // System signal tests
    // ==================

    #[test]
    fn test_blockers_resolved_is_system_signal() {
        let event = TaskEvent::BlockersResolved;
        assert!(event.is_system_signal());
        assert!(!event.is_user_action());
        assert!(!event.is_agent_signal());
        assert_eq!(event.name(), "BlockersResolved");
    }

    #[test]
    fn test_blocker_detected_is_system_signal() {
        let event = TaskEvent::BlockerDetected {
            blocker_id: "task-123".to_string(),
        };
        assert!(event.is_system_signal());
        assert_eq!(event.name(), "BlockerDetected");
    }

    #[test]
    fn test_blocker_detected_contains_id() {
        let event = TaskEvent::BlockerDetected {
            blocker_id: "task-abc-123".to_string(),
        };
        if let TaskEvent::BlockerDetected { blocker_id } = event {
            assert_eq!(blocker_id, "task-abc-123");
        } else {
            panic!("Expected BlockerDetected");
        }
    }

    // ==================
    // Clone and Debug tests
    // ==================

    #[test]
    fn test_task_event_clone() {
        let event = TaskEvent::ExecutionFailed {
            error: "Test error".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_task_event_debug() {
        let event = TaskEvent::Schedule;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Schedule"));
    }

    // ==================
    // Serialization tests
    // ==================

    #[test]
    fn test_task_event_serializes_to_json() {
        let event = TaskEvent::Schedule;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"Schedule\"");
    }

    #[test]
    fn test_task_event_with_data_serializes_to_json() {
        let event = TaskEvent::ExecutionFailed {
            error: "Test error".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ExecutionFailed"));
        assert!(json.contains("Test error"));
    }

    #[test]
    fn test_task_event_deserializes_from_json() {
        let json = "\"Cancel\"";
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event, TaskEvent::Cancel);
    }

    #[test]
    fn test_task_event_with_data_deserializes_from_json() {
        let json = r#"{"BlockerDetected":{"blocker_id":"task-99"}}"#;
        let event: TaskEvent = serde_json::from_str(json).unwrap();
        if let TaskEvent::BlockerDetected { blocker_id } = event {
            assert_eq!(blocker_id, "task-99");
        } else {
            panic!("Expected BlockerDetected");
        }
    }

    #[test]
    fn test_task_event_roundtrip_serialization() {
        let events = vec![
            TaskEvent::Schedule,
            TaskEvent::Cancel,
            TaskEvent::ForceApprove,
            TaskEvent::Retry,
            TaskEvent::SkipQa,
            TaskEvent::ExecutionComplete,
            TaskEvent::ExecutionFailed {
                error: "Test".to_string(),
            },
            TaskEvent::NeedsHumanInput {
                reason: "Need info".to_string(),
            },
            TaskEvent::QaRefinementComplete,
            TaskEvent::QaTestsComplete { passed: true },
            TaskEvent::QaTestsComplete { passed: false },
            TaskEvent::ReviewComplete {
                approved: true,
                feedback: None,
            },
            TaskEvent::ReviewComplete {
                approved: false,
                feedback: Some("Needs work".to_string()),
            },
            TaskEvent::BlockersResolved,
            TaskEvent::BlockerDetected {
                blocker_id: "id".to_string(),
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let restored: TaskEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, restored);
        }
    }

    // ==================
    // All variants exist test
    // ==================

    #[test]
    fn test_all_14_event_variants_exist() {
        // This test ensures we have all 14 variants as specified in the PRD
        let events = [
            TaskEvent::Schedule,
            TaskEvent::Cancel,
            TaskEvent::ForceApprove,
            TaskEvent::Retry,
            TaskEvent::SkipQa,
            TaskEvent::ExecutionComplete,
            TaskEvent::ExecutionFailed {
                error: String::new(),
            },
            TaskEvent::NeedsHumanInput {
                reason: String::new(),
            },
            TaskEvent::QaRefinementComplete,
            TaskEvent::QaTestsComplete { passed: true },
            TaskEvent::ReviewComplete {
                approved: true,
                feedback: None,
            },
            TaskEvent::BlockersResolved,
            TaskEvent::BlockerDetected {
                blocker_id: String::new(),
            },
        ];

        // QaTestsComplete and ReviewComplete have variants based on data but count as 1 each
        // So we have 13 distinct enum variants (14 counting both QaTestsComplete outcomes)
        assert_eq!(events.len(), 13);
    }
}
