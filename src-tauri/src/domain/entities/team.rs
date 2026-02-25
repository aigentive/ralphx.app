// Domain entities for team history persistence
// Maps to team_sessions and team_messages tables (v37 migration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::application::team_state_tracker::TeammateCost;

/// Unique identifier for a TeamSession
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamSessionId(pub String);

impl TeamSessionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TeamSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TeamSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a TeamMessage
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamMessageId(pub String);

impl TeamMessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TeamMessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TeamMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Snapshot of a teammate's state at a point in time
/// Stored as JSON in team_sessions.teammate_json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateSnapshot {
    pub name: String,
    pub color: String,
    pub model: String,
    pub role: String,
    pub status: String,
    pub cost: TeammateCost,
    pub spawned_at: String,
    pub last_activity_at: String,
    /// Conversation ID linking to this teammate's chat_conversations row.
    /// Added after v37 — `#[serde(default)]` ensures existing JSON blobs
    /// without this field deserialize as None.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// A team session — one row per active/historical team
/// Maps to team_sessions table (v37 migration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSession {
    pub id: TeamSessionId,
    pub team_name: String,
    pub context_id: String,
    pub context_type: String,
    pub lead_name: Option<String>,
    pub phase: String,
    pub teammates: Vec<TeammateSnapshot>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub disbanded_at: Option<DateTime<Utc>>,
}

impl TeamSession {
    pub fn new(
        team_name: impl Into<String>,
        context_id: impl Into<String>,
        context_type: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TeamSessionId::new(),
            team_name: team_name.into(),
            context_id: context_id.into(),
            context_type: context_type.into(),
            lead_name: None,
            phase: "forming".to_string(),
            teammates: Vec::new(),
            created_at: now,
            updated_at: now,
            disbanded_at: None,
        }
    }
}

/// A single message in a team session
/// Maps to team_messages table (v37 migration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessageRecord {
    pub id: TeamMessageId,
    pub team_session_id: TeamSessionId,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub message_type: String,
    pub created_at: DateTime<Utc>,
}

impl TeamMessageRecord {
    pub fn new(
        team_session_id: TeamSessionId,
        sender: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: TeamMessageId::new(),
            team_session_id,
            sender: sender.into(),
            recipient: None,
            content: content.into(),
            message_type: "teammate_message".to_string(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
#[path = "team_tests.rs"]
mod tests;
