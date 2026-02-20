use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{ChatConversation, ChatConversationId};

/// Unique identifier for an agent run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentRunId(Uuid);

impl AgentRunId {
    /// Create a new random agent run ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Get as string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Create from string (for database deserialization)
    pub fn from_string(s: impl Into<String>) -> Self {
        let s = s.into();
        Self(Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::nil()))
    }
}

impl Default for AgentRunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentRunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AgentRunId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<AgentRunId> for String {
    fn from(id: AgentRunId) -> Self {
        id.0.to_string()
    }
}

impl std::str::FromStr for AgentRunId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Status of an agent run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRunStatus {
    /// Agent is currently running
    Running,
    /// Agent completed successfully
    Completed,
    /// Agent failed with an error
    Failed,
    /// Agent was cancelled by user
    Cancelled,
}

impl fmt::Display for AgentRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentRunStatus::Running => write!(f, "running"),
            AgentRunStatus::Completed => write!(f, "completed"),
            AgentRunStatus::Failed => write!(f, "failed"),
            AgentRunStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for AgentRunStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(AgentRunStatus::Running),
            "completed" => Ok(AgentRunStatus::Completed),
            "failed" => Ok(AgentRunStatus::Failed),
            "cancelled" => Ok(AgentRunStatus::Cancelled),
            _ => Err(format!("Invalid agent run status: {}", s)),
        }
    }
}

impl AgentRunStatus {
    /// Check if the run is in a terminal state (completed, failed, or cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentRunStatus::Completed | AgentRunStatus::Failed | AgentRunStatus::Cancelled
        )
    }

    /// Check if the run is still active
    pub fn is_active(&self) -> bool {
        matches!(self, AgentRunStatus::Running)
    }
}

/// An agent run tracks the execution of a Claude agent for a conversation
///
/// This enables:
/// - Streaming persistence (messages saved as they arrive)
/// - Leave-and-come-back (user can navigate away and return)
/// - Message queueing (queue messages while agent is running)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    /// Unique identifier for this run
    pub id: AgentRunId,
    /// The conversation this run belongs to
    pub conversation_id: ChatConversationId,
    /// Current status of the run
    pub status: AgentRunStatus,
    /// When the run started
    pub started_at: DateTime<Utc>,
    /// When the run completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Correlation ID linking all runs in a single message chain
    /// (initial run + all queue continuations via --resume)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_chain_id: Option<String>,
    /// The agent_run ID that triggered this continuation (None for initial runs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
}

impl AgentRun {
    /// Create a new agent run in the running state with a fresh run_chain_id
    pub fn new(conversation_id: ChatConversationId) -> Self {
        let chain_id = Uuid::new_v4().to_string();
        Self {
            id: AgentRunId::new(),
            conversation_id,
            status: AgentRunStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
            run_chain_id: Some(chain_id),
            parent_run_id: None,
        }
    }

    /// Create a continuation run inheriting the chain from a parent run
    pub fn new_continuation(
        conversation_id: ChatConversationId,
        run_chain_id: String,
        parent_run_id: String,
    ) -> Self {
        Self {
            id: AgentRunId::new(),
            conversation_id,
            status: AgentRunStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
            run_chain_id: Some(run_chain_id),
            parent_run_id: Some(parent_run_id),
        }
    }

    /// Mark the run as completed
    pub fn complete(&mut self) {
        self.status = AgentRunStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Mark the run as failed with an error message
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = AgentRunStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.into());
    }

    /// Mark the run as cancelled
    pub fn cancel(&mut self) {
        self.status = AgentRunStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.error_message = None;
    }

    /// Get the duration of the run (if completed)
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed.signed_duration_since(self.started_at))
    }

    /// Check if this run is still active
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Check if this run is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// A conversation that was interrupted during app shutdown
///
/// Contains the conversation that was interrupted along with the
/// last agent run that was orphaned. Used by ChatResumptionRunner
/// to resume conversations on app startup.
#[derive(Debug, Clone)]
pub struct InterruptedConversation {
    /// The conversation that was interrupted
    pub conversation: ChatConversation,
    /// The last agent run that was orphaned
    pub last_run: AgentRun,
}

#[cfg(test)]
#[path = "agent_run_tests.rs"]
mod tests;
