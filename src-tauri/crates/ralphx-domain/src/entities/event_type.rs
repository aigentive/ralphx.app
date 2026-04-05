// IMPORTANT: Pure value enum only. NO infrastructure dependencies (no reqwest, DashMap, DB types).
// This preserves clean architecture - domain has no infra deps.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Task events (5)
    TaskCreated,
    TaskStatusChanged,
    TaskStepCompleted,
    TaskExecutionStarted,
    TaskExecutionCompleted,
    // Review events (4)
    ReviewReady,
    ReviewApproved,
    ReviewChangesRequested,
    ReviewEscalated,
    // Merge events (4)
    MergeReady,
    MergeCompleted,
    MergeConflict,
    PlanDelivered,
    // Ideation events (7)
    IdeationSessionCreated,
    IdeationPlanCreated,
    IdeationVerified,
    IdeationProposalsReady,
    IdeationSessionAccepted,
    IdeationAutoProposeSent,
    IdeationAutoProposeFailed,
    // System events (2)
    SystemWebhookUnhealthy,
    SystemRateLimitWarning,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EventType::TaskCreated => "task:created",
            EventType::TaskStatusChanged => "task:status_changed",
            EventType::TaskStepCompleted => "task:step_completed",
            EventType::TaskExecutionStarted => "task:execution_started",
            EventType::TaskExecutionCompleted => "task:execution_completed",
            EventType::ReviewReady => "review:ready",
            EventType::ReviewApproved => "review:approved",
            EventType::ReviewChangesRequested => "review:changes_requested",
            EventType::ReviewEscalated => "review:escalated",
            EventType::MergeReady => "merge:ready",
            EventType::MergeCompleted => "merge:completed",
            EventType::MergeConflict => "merge:conflict",
            EventType::PlanDelivered => "plan:delivered",
            EventType::IdeationSessionCreated => "ideation:session_created",
            EventType::IdeationPlanCreated => "ideation:plan_created",
            EventType::IdeationVerified => "ideation:verified",
            EventType::IdeationProposalsReady => "ideation:proposals_ready",
            EventType::IdeationSessionAccepted => "ideation:session_accepted",
            EventType::IdeationAutoProposeSent => "ideation:auto_propose_sent",
            EventType::IdeationAutoProposeFailed => "ideation:auto_propose_failed",
            EventType::SystemWebhookUnhealthy => "system:webhook_unhealthy",
            EventType::SystemRateLimitWarning => "system:rate_limit_warning",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseEventTypeError(String);

impl fmt::Display for ParseEventTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown event type: {}", self.0)
    }
}

impl std::error::Error for ParseEventTypeError {}

impl FromStr for EventType {
    type Err = ParseEventTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task:created" => Ok(EventType::TaskCreated),
            "task:status_changed" => Ok(EventType::TaskStatusChanged),
            "task:step_completed" => Ok(EventType::TaskStepCompleted),
            "task:execution_started" => Ok(EventType::TaskExecutionStarted),
            "task:execution_completed" => Ok(EventType::TaskExecutionCompleted),
            "review:ready" => Ok(EventType::ReviewReady),
            "review:approved" => Ok(EventType::ReviewApproved),
            "review:changes_requested" => Ok(EventType::ReviewChangesRequested),
            "review:escalated" => Ok(EventType::ReviewEscalated),
            "merge:ready" => Ok(EventType::MergeReady),
            "merge:completed" => Ok(EventType::MergeCompleted),
            "merge:conflict" => Ok(EventType::MergeConflict),
            "plan:delivered" => Ok(EventType::PlanDelivered),
            "ideation:session_created" => Ok(EventType::IdeationSessionCreated),
            "ideation:plan_created" => Ok(EventType::IdeationPlanCreated),
            "ideation:verified" => Ok(EventType::IdeationVerified),
            "ideation:proposals_ready" => Ok(EventType::IdeationProposalsReady),
            "ideation:session_accepted" => Ok(EventType::IdeationSessionAccepted),
            "ideation:auto_propose_sent" => Ok(EventType::IdeationAutoProposeSent),
            "ideation:auto_propose_failed" => Ok(EventType::IdeationAutoProposeFailed),
            "system:webhook_unhealthy" => Ok(EventType::SystemWebhookUnhealthy),
            "system:rate_limit_warning" => Ok(EventType::SystemRateLimitWarning),
            other => Err(ParseEventTypeError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_produces_colon_format() {
        assert_eq!(EventType::TaskCreated.to_string(), "task:created");
        assert_eq!(
            EventType::TaskStatusChanged.to_string(),
            "task:status_changed"
        );
        assert_eq!(EventType::ReviewReady.to_string(), "review:ready");
        assert_eq!(EventType::MergeCompleted.to_string(), "merge:completed");
        assert_eq!(
            EventType::IdeationSessionCreated.to_string(),
            "ideation:session_created"
        );
        assert_eq!(
            EventType::IdeationSessionAccepted.to_string(),
            "ideation:session_accepted"
        );
        assert_eq!(
            EventType::SystemWebhookUnhealthy.to_string(),
            "system:webhook_unhealthy"
        );
        assert_eq!(
            EventType::SystemRateLimitWarning.to_string(),
            "system:rate_limit_warning"
        );
    }

    #[test]
    fn from_str_roundtrips_all_variants() {
        let variants = [
            EventType::TaskCreated,
            EventType::TaskStatusChanged,
            EventType::TaskStepCompleted,
            EventType::TaskExecutionStarted,
            EventType::TaskExecutionCompleted,
            EventType::ReviewReady,
            EventType::ReviewApproved,
            EventType::ReviewChangesRequested,
            EventType::ReviewEscalated,
            EventType::MergeReady,
            EventType::MergeCompleted,
            EventType::MergeConflict,
            EventType::PlanDelivered,
            EventType::IdeationSessionCreated,
            EventType::IdeationPlanCreated,
            EventType::IdeationVerified,
            EventType::IdeationProposalsReady,
            EventType::IdeationSessionAccepted,
            EventType::IdeationAutoProposeSent,
            EventType::IdeationAutoProposeFailed,
            EventType::SystemWebhookUnhealthy,
            EventType::SystemRateLimitWarning,
        ];
        for variant in &variants {
            let s = variant.to_string();
            let parsed: EventType = s.parse().expect("should parse back");
            assert_eq!(parsed, *variant, "roundtrip failed for {:?}", variant);
        }
    }

    #[test]
    fn from_str_error_on_unknown() {
        let result = "unknown:event".parse::<EventType>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseEventTypeError("unknown:event".to_string())
        );
    }

    #[test]
    fn serde_roundtrip() {
        let event = EventType::TaskStatusChanged;
        let json = serde_json::to_string(&event).unwrap();
        // serde uses snake_case per #[serde(rename_all = "snake_case")]
        assert_eq!(json, "\"task_status_changed\"");
        let back: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}
