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
    /// Conversation ID associated with this state (for executing/reviewing states)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Agent run ID that was started for this state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_run_id: Option<String>,
}

impl StatusTransition {
    /// Create a new status transition record
    pub fn new(from: InternalStatus, to: InternalStatus, trigger: impl Into<String>) -> Self {
        Self {
            from,
            to,
            trigger: trigger.into(),
            timestamp: Utc::now(),
            conversation_id: None,
            agent_run_id: None,
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
            conversation_id: None,
            agent_run_id: None,
        }
    }

    /// Create a status transition with full metadata (including conversation tracking)
    pub fn with_metadata(
        from: InternalStatus,
        to: InternalStatus,
        trigger: impl Into<String>,
        timestamp: DateTime<Utc>,
        conversation_id: Option<String>,
        agent_run_id: Option<String>,
    ) -> Self {
        Self {
            from,
            to,
            trigger: trigger.into(),
            timestamp,
            conversation_id,
            agent_run_id,
        }
    }
}

#[cfg(test)]
#[path = "status_transition_tests.rs"]
mod tests;
