// Status transition record for audit logging

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::entities::InternalStatus;

/// Record of a status transition for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    /// Previous status
    pub from: InternalStatus,
    /// New status
    pub to: InternalStatus,
    /// What triggered this transition (e.g., "user", "agent", "system")
    pub trigger: String,
    /// When the transition occurred
    pub timestamp: DateTime<Utc>,
}

impl StatusTransition {
    /// Create a new status transition record
    pub fn new(from: InternalStatus, to: InternalStatus, trigger: impl Into<String>) -> Self {
        Self {
            from,
            to,
            trigger: trigger.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a status transition with a specific timestamp (for deserialization)
    pub fn with_timestamp(
        from: InternalStatus,
        to: InternalStatus,
        trigger: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            from,
            to,
            trigger: trigger.into(),
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transition_new() {
        let transition = StatusTransition::new(
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "user",
        );

        assert_eq!(transition.from, InternalStatus::Backlog);
        assert_eq!(transition.to, InternalStatus::Ready);
        assert_eq!(transition.trigger, "user");
    }

    #[test]
    fn test_status_transition_trigger_from_string() {
        let trigger = String::from("agent_worker");
        let transition = StatusTransition::new(
            InternalStatus::Ready,
            InternalStatus::Executing,
            trigger,
        );

        assert_eq!(transition.trigger, "agent_worker");
    }

    #[test]
    fn test_status_transition_with_timestamp() {
        let timestamp = Utc::now();
        let transition = StatusTransition::with_timestamp(
            InternalStatus::Executing,
            InternalStatus::QaRefining,
            "system",
            timestamp,
        );

        assert_eq!(transition.timestamp, timestamp);
    }

    #[test]
    fn test_status_transition_clone() {
        let transition = StatusTransition::new(
            InternalStatus::QaTesting,
            InternalStatus::QaPassed,
            "qa_agent",
        );

        let cloned = transition.clone();
        assert_eq!(cloned.from, transition.from);
        assert_eq!(cloned.to, transition.to);
        assert_eq!(cloned.trigger, transition.trigger);
    }

    #[test]
    fn test_status_transition_debug() {
        let transition = StatusTransition::new(
            InternalStatus::PendingReview,
            InternalStatus::Approved,
            "reviewer",
        );

        let debug_str = format!("{:?}", transition);
        assert!(debug_str.contains("PendingReview"));
        assert!(debug_str.contains("Approved"));
        assert!(debug_str.contains("reviewer"));
    }

    #[test]
    fn test_status_transition_serializes_to_json() {
        let transition = StatusTransition::new(
            InternalStatus::Backlog,
            InternalStatus::Ready,
            "user",
        );

        let json = serde_json::to_string(&transition).unwrap();
        assert!(json.contains("\"from\":\"backlog\""));
        assert!(json.contains("\"to\":\"ready\""));
        assert!(json.contains("\"trigger\":\"user\""));
    }

    #[test]
    fn test_status_transition_deserializes_from_json() {
        let json = r#"{
            "from": "executing",
            "to": "qa_refining",
            "trigger": "agent",
            "timestamp": "2026-01-24T12:00:00Z"
        }"#;

        let transition: StatusTransition = serde_json::from_str(json).unwrap();
        assert_eq!(transition.from, InternalStatus::Executing);
        assert_eq!(transition.to, InternalStatus::QaRefining);
        assert_eq!(transition.trigger, "agent");
    }

    #[test]
    fn test_status_transition_roundtrip() {
        let original = StatusTransition::new(
            InternalStatus::QaFailed,
            InternalStatus::Executing,
            "retry",
        );

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: StatusTransition = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.from, original.from);
        assert_eq!(deserialized.to, original.to);
        assert_eq!(deserialized.trigger, original.trigger);
    }
}
