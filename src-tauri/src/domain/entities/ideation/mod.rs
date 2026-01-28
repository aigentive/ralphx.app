//! Ideation system entities - brainstorming sessions and task proposals
//!
//! This module contains:
//! - IdeationSession: the main session entity
//! - TaskProposal: task proposals generated during ideation
//! - ChatMessage: messages in ideation conversations
//! - Priority assessment system with detailed factors
//! - Dependency graph for tracking proposal relationships

mod assessment;
mod chat;
mod graph;
mod proposal;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use assessment::*;
pub use chat::*;
pub use graph::*;
pub use proposal::TaskProposal;
pub use types::*;

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{ArtifactId, IdeationSessionId, ProjectId};

/// An ideation session - a brainstorming conversation that produces task proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationSession {
    /// Unique identifier for this session
    pub id: IdeationSessionId,
    /// Project this session belongs to
    pub project_id: ProjectId,
    /// Human-readable title (auto-generated or user-defined)
    pub title: Option<String>,
    /// Current status of the session
    pub status: IdeationSessionStatus,
    /// The implementation plan artifact for this session
    pub plan_artifact_id: Option<ArtifactId>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// When the session was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
    /// When all proposals were converted to tasks (if applicable)
    pub converted_at: Option<DateTime<Utc>>,
}

/// Builder for creating IdeationSession instances
#[derive(Debug, Default)]
pub struct IdeationSessionBuilder {
    id: Option<IdeationSessionId>,
    project_id: Option<ProjectId>,
    title: Option<String>,
    status: Option<IdeationSessionStatus>,
    plan_artifact_id: Option<ArtifactId>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    converted_at: Option<DateTime<Utc>>,
}

impl IdeationSessionBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session ID
    pub fn id(mut self, id: IdeationSessionId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the project ID
    pub fn project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: IdeationSessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the plan artifact ID
    pub fn plan_artifact_id(mut self, plan_artifact_id: ArtifactId) -> Self {
        self.plan_artifact_id = Some(plan_artifact_id);
        self
    }

    /// Set the created_at timestamp
    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// Set the updated_at timestamp
    pub fn updated_at(mut self, updated_at: DateTime<Utc>) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// Set the archived_at timestamp
    pub fn archived_at(mut self, archived_at: DateTime<Utc>) -> Self {
        self.archived_at = Some(archived_at);
        self
    }

    /// Set the converted_at timestamp
    pub fn converted_at(mut self, converted_at: DateTime<Utc>) -> Self {
        self.converted_at = Some(converted_at);
        self
    }

    /// Build the IdeationSession
    /// Panics if project_id is not set
    pub fn build(self) -> IdeationSession {
        let now = Utc::now();
        IdeationSession {
            id: self.id.unwrap_or_default(),
            project_id: self.project_id.expect("project_id is required"),
            title: self.title,
            status: self.status.unwrap_or_default(),
            plan_artifact_id: self.plan_artifact_id,
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            archived_at: self.archived_at,
            converted_at: self.converted_at,
        }
    }
}

impl IdeationSession {
    /// Creates a new active session for a project
    pub fn new(project_id: ProjectId) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .build()
    }

    /// Creates a new active session with a title
    pub fn new_with_title(project_id: ProjectId, title: impl Into<String>) -> Self {
        IdeationSessionBuilder::new()
            .project_id(project_id)
            .title(title)
            .build()
    }

    /// Creates a builder for more complex session creation
    pub fn builder() -> IdeationSessionBuilder {
        IdeationSessionBuilder::new()
    }

    /// Returns true if the session is active
    pub fn is_active(&self) -> bool {
        self.status == IdeationSessionStatus::Active
    }

    /// Returns true if the session has been archived
    pub fn is_archived(&self) -> bool {
        self.status == IdeationSessionStatus::Archived
    }

    /// Returns true if all proposals have been converted
    pub fn is_converted(&self) -> bool {
        self.status == IdeationSessionStatus::Converted
    }

    /// Archives the session
    pub fn archive(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Archived;
        self.archived_at = Some(now);
        self.updated_at = now;
    }

    /// Marks the session as converted (all proposals applied)
    pub fn mark_converted(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Converted;
        self.converted_at = Some(now);
        self.updated_at = now;
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserialize an IdeationSession from a SQLite row
    /// Expects columns: id, project_id, title, status, plan_artifact_id, created_at, updated_at, archived_at, converted_at
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: IdeationSessionId::from_string(row.get::<_, String>("id")?),
            project_id: ProjectId::from_string(row.get::<_, String>("project_id")?),
            title: row.get("title")?,
            status: row
                .get::<_, String>("status")?
                .parse()
                .unwrap_or(IdeationSessionStatus::Active),
            plan_artifact_id: row
                .get::<_, Option<String>>("plan_artifact_id")?
                .map(ArtifactId::from_string),
            created_at: Self::parse_datetime(row.get("created_at")?),
            updated_at: Self::parse_datetime(row.get("updated_at")?),
            archived_at: row
                .get::<_, Option<String>>("archived_at")?
                .map(Self::parse_datetime),
            converted_at: row
                .get::<_, Option<String>>("converted_at")?
                .map(Self::parse_datetime),
        })
    }

    /// Parse a datetime string from SQLite into a DateTime<Utc>
    /// Handles both RFC3339 format and SQLite's CURRENT_TIMESTAMP format
    fn parse_datetime(s: String) -> DateTime<Utc> {
        parse_datetime_helper(s)
    }
}
