//! SessionLink entity for linking ideation sessions

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::types::parse_datetime_helper;
use crate::entities::types::{IdeationSessionId, SessionLinkId};

/// Relationship type between parent and child sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRelationship {
    /// Follow-on session to address gaps or extend the parent plan
    FollowOn,
    /// Alternative approach to the same problem
    Alternative,
    /// Dependency relationship (child is prerequisite for parent)
    Dependency,
}

impl Default for SessionRelationship {
    fn default() -> Self {
        Self::FollowOn
    }
}

impl FromStr for SessionRelationship {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "follow_on" => Ok(Self::FollowOn),
            "alternative" => Ok(Self::Alternative),
            "dependency" => Ok(Self::Dependency),
            _ => Err(format!("Unknown session relationship: {}", s)),
        }
    }
}

impl std::fmt::Display for SessionRelationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FollowOn => "follow_on",
            Self::Alternative => "alternative",
            Self::Dependency => "dependency",
        };
        write!(f, "{}", s)
    }
}

/// Link between a parent session and a child session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLink {
    /// Unique identifier for this link
    pub id: SessionLinkId,
    /// Parent session ID
    pub parent_session_id: IdeationSessionId,
    /// Child session ID
    pub child_session_id: IdeationSessionId,
    /// Type of relationship
    pub relationship: SessionRelationship,
    /// Optional notes about why this link was created
    pub notes: Option<String>,
    /// When the link was created
    pub created_at: DateTime<Utc>,
}

impl SessionLink {
    /// Creates a new session link
    pub fn new(
        parent_session_id: IdeationSessionId,
        child_session_id: IdeationSessionId,
        relationship: SessionRelationship,
    ) -> Self {
        Self {
            id: SessionLinkId::new(),
            parent_session_id,
            child_session_id,
            relationship,
            notes: None,
            created_at: Utc::now(),
        }
    }

    /// Creates a new session link with notes
    pub fn with_notes(
        parent_session_id: IdeationSessionId,
        child_session_id: IdeationSessionId,
        relationship: SessionRelationship,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: SessionLinkId::new(),
            parent_session_id,
            child_session_id,
            relationship,
            notes: Some(notes.into()),
            created_at: Utc::now(),
        }
    }

    /// Deserialize a SessionLink from a SQLite row
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: SessionLinkId::from_string(row.get::<_, String>("id")?),
            parent_session_id: IdeationSessionId::from_string(
                row.get::<_, String>("parent_session_id")?,
            ),
            child_session_id: IdeationSessionId::from_string(
                row.get::<_, String>("child_session_id")?,
            ),
            relationship: row
                .get::<_, String>("relationship")?
                .parse()
                .unwrap_or(SessionRelationship::FollowOn),
            notes: row.get("notes")?,
            created_at: parse_datetime_helper(row.get("created_at")?),
        })
    }
}

#[cfg(test)]
#[path = "session_link_tests.rs"]
mod tests;
