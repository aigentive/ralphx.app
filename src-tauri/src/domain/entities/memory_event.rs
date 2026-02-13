// Memory event entity for audit trail

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;

use super::ProjectId;

/// Unique identifier for a memory event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryEventId(pub String);

impl MemoryEventId {
    /// Creates a new MemoryEventId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a MemoryEventId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MemoryEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of actor that triggered the memory event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryActorType {
    System,
    MemoryMaintainer,
    MemoryCapture,
}

impl fmt::Display for MemoryActorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryActorType::System => write!(f, "system"),
            MemoryActorType::MemoryMaintainer => write!(f, "memory-maintainer"),
            MemoryActorType::MemoryCapture => write!(f, "memory-capture"),
        }
    }
}

impl std::str::FromStr for MemoryActorType {
    type Err = ParseMemoryActorTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(MemoryActorType::System),
            "memory-maintainer" => Ok(MemoryActorType::MemoryMaintainer),
            "memory-capture" => Ok(MemoryActorType::MemoryCapture),
            _ => Err(ParseMemoryActorTypeError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseMemoryActorTypeError(String);

impl fmt::Display for ParseMemoryActorTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid memory actor type: {}", self.0)
    }
}

impl std::error::Error for ParseMemoryActorTypeError {}

/// Memory event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub id: MemoryEventId,
    pub project_id: ProjectId,
    pub event_type: String,
    pub actor_type: MemoryActorType,
    pub details: JsonValue,
    pub created_at: DateTime<Utc>,
}

impl MemoryEvent {
    /// Create a new memory event
    pub fn new(
        project_id: ProjectId,
        event_type: impl Into<String>,
        actor_type: MemoryActorType,
        details: JsonValue,
    ) -> Self {
        Self {
            id: MemoryEventId::new(),
            project_id,
            event_type: event_type.into(),
            actor_type,
            details,
            created_at: Utc::now(),
        }
    }
}
