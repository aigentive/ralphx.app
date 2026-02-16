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
mod tests {
    use super::*;

    #[test]
    fn team_session_id_generates_unique() {
        let id1 = TeamSessionId::new();
        let id2 = TeamSessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn team_session_id_from_string() {
        let id = TeamSessionId::from_string("test-id");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn team_message_id_generates_unique() {
        let id1 = TeamMessageId::new();
        let id2 = TeamMessageId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn team_session_new_defaults() {
        let session = TeamSession::new("my-team", "ctx-123", "task");
        assert_eq!(session.team_name, "my-team");
        assert_eq!(session.context_id, "ctx-123");
        assert_eq!(session.context_type, "task");
        assert_eq!(session.phase, "forming");
        assert!(session.teammates.is_empty());
        assert!(session.disbanded_at.is_none());
        assert!(session.lead_name.is_none());
    }

    #[test]
    fn team_message_record_new_defaults() {
        let session_id = TeamSessionId::new();
        let msg = TeamMessageRecord::new(session_id.clone(), "worker-1", "hello");
        assert_eq!(msg.team_session_id, session_id);
        assert_eq!(msg.sender, "worker-1");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.message_type, "teammate_message");
        assert!(msg.recipient.is_none());
    }

    #[test]
    fn teammate_snapshot_serializes() {
        let snap = TeammateSnapshot {
            name: "worker-1".to_string(),
            color: "#ff6b35".to_string(),
            model: "sonnet".to_string(),
            role: "coder".to_string(),
            status: "idle".to_string(),
            cost: TeammateCost {
                input_tokens: 1000,
                output_tokens: 500,
                cache_creation_tokens: 200,
                cache_read_tokens: 100,
                estimated_usd: 0.05,
            },
            spawned_at: "2024-01-01T00:00:00Z".to_string(),
            last_activity_at: "2024-01-01T00:01:00Z".to_string(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("worker-1"));

        let parsed: TeammateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "worker-1");
        assert_eq!(parsed.color, "#ff6b35");
        assert_eq!(parsed.role, "coder");
        assert_eq!(parsed.cost.input_tokens, 1000);
    }

    #[test]
    fn team_session_id_display() {
        let id = TeamSessionId::from_string("display-test");
        assert_eq!(format!("{}", id), "display-test");
    }

    #[test]
    fn team_message_id_display() {
        let id = TeamMessageId::from_string("msg-display");
        assert_eq!(format!("{}", id), "msg-display");
    }

    #[test]
    fn team_session_id_serializes() {
        let id = TeamSessionId::from_string("ser-test");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"ser-test\"");
    }

    #[test]
    fn team_message_id_serializes() {
        let id = TeamMessageId::from_string("msg-ser");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"msg-ser\"");
    }
}
