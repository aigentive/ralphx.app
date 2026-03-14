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
pub mod session_context;
pub mod session_link;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use assessment::*;
pub use chat::*;
pub use graph::*;
pub use proposal::TaskProposal;
pub use session_context::*;
pub use session_link::*;
pub use types::*;

use chrono::{DateTime, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{ArtifactId, IdeationSessionId, ProjectId, TaskId};

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
    /// The implementation plan artifact for this session (owned by this session)
    pub plan_artifact_id: Option<ArtifactId>,
    /// Plan artifact inherited from parent session (read-only; child cannot modify)
    pub inherited_plan_artifact_id: Option<ArtifactId>,
    /// Optional reference to a draft task that seeded this session
    pub seed_task_id: Option<TaskId>,
    /// Optional parent session for session linking (follow-on work, etc.)
    pub parent_session_id: Option<IdeationSessionId>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// When the session was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
    /// When all proposals were converted to tasks (if applicable)
    pub converted_at: Option<DateTime<Utc>>,
    /// Team mode: "solo" | "research" | "debate"
    pub team_mode: Option<String>,
    /// Serialized JSON team configuration
    pub team_config_json: Option<String>,
    /// Title source: "auto" (session-namer) | "user" (manual rename). None treated as "auto".
    pub title_source: Option<String>,
    /// Verification status of this session's plan
    #[serde(default)]
    pub verification_status: VerificationStatus,
    /// Whether a verification loop is currently active
    #[serde(default)]
    pub verification_in_progress: bool,
    /// JSON-serialized VerificationMetadata (round history, gaps, convergence reason)
    #[serde(default)]
    pub verification_metadata: Option<String>,
    /// Generation counter for zombie protection (incremented on each auto-verify trigger)
    #[serde(default)]
    pub verification_generation: i32,
    /// Source project ID when this session was imported from another project
    pub source_project_id: Option<String>,
    /// Source session ID when this session was imported from another project
    pub source_session_id: Option<String>,
}

/// Builder for creating IdeationSession instances
#[derive(Debug, Default)]
pub struct IdeationSessionBuilder {
    id: Option<IdeationSessionId>,
    project_id: Option<ProjectId>,
    title: Option<String>,
    status: Option<IdeationSessionStatus>,
    plan_artifact_id: Option<ArtifactId>,
    inherited_plan_artifact_id: Option<ArtifactId>,
    seed_task_id: Option<TaskId>,
    parent_session_id: Option<IdeationSessionId>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    converted_at: Option<DateTime<Utc>>,
    team_mode: Option<String>,
    team_config_json: Option<String>,
    title_source: Option<String>,
    verification_status: Option<VerificationStatus>,
    verification_in_progress: Option<bool>,
    verification_metadata: Option<String>,
    verification_generation: Option<i32>,
    source_project_id: Option<String>,
    source_session_id: Option<String>,
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

    /// Set the inherited plan artifact ID (read-only, from parent session)
    pub fn inherited_plan_artifact_id(mut self, inherited_plan_artifact_id: ArtifactId) -> Self {
        self.inherited_plan_artifact_id = Some(inherited_plan_artifact_id);
        self
    }

    /// Set the seed task ID
    pub fn seed_task_id(mut self, seed_task_id: TaskId) -> Self {
        self.seed_task_id = Some(seed_task_id);
        self
    }

    /// Set the parent session ID
    pub fn parent_session_id(mut self, parent_session_id: IdeationSessionId) -> Self {
        self.parent_session_id = Some(parent_session_id);
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

    /// Set the team mode
    pub fn team_mode(mut self, team_mode: impl Into<String>) -> Self {
        self.team_mode = Some(team_mode.into());
        self
    }

    /// Set the team config JSON
    pub fn team_config_json(mut self, team_config_json: impl Into<String>) -> Self {
        self.team_config_json = Some(team_config_json.into());
        self
    }

    /// Set the title source
    pub fn title_source(mut self, title_source: impl Into<String>) -> Self {
        self.title_source = Some(title_source.into());
        self
    }

    /// Set the verification status
    pub fn verification_status(mut self, verification_status: VerificationStatus) -> Self {
        self.verification_status = Some(verification_status);
        self
    }

    /// Set the verification generation counter
    pub fn verification_generation(mut self, generation: i32) -> Self {
        self.verification_generation = Some(generation);
        self
    }

    /// Set the source project ID (cross-project import provenance)
    pub fn source_project_id(mut self, source_project_id: impl Into<String>) -> Self {
        self.source_project_id = Some(source_project_id.into());
        self
    }

    /// Set the source session ID (cross-project import provenance)
    pub fn source_session_id(mut self, source_session_id: impl Into<String>) -> Self {
        self.source_session_id = Some(source_session_id.into());
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
            inherited_plan_artifact_id: self.inherited_plan_artifact_id,
            seed_task_id: self.seed_task_id,
            parent_session_id: self.parent_session_id,
            created_at: self.created_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
            archived_at: self.archived_at,
            converted_at: self.converted_at,
            team_mode: self.team_mode,
            team_config_json: self.team_config_json,
            title_source: self.title_source,
            verification_status: self.verification_status.unwrap_or_default(),
            verification_in_progress: self.verification_in_progress.unwrap_or(false),
            verification_metadata: self.verification_metadata,
            verification_generation: self.verification_generation.unwrap_or(0),
            source_project_id: self.source_project_id,
            source_session_id: self.source_session_id,
        }
    }
}

impl IdeationSession {
    /// Creates a new active session for a project
    pub fn new(project_id: ProjectId) -> Self {
        IdeationSessionBuilder::new().project_id(project_id).build()
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

    /// Returns true if all proposals have been accepted and applied
    pub fn is_accepted(&self) -> bool {
        self.status == IdeationSessionStatus::Accepted
    }

    /// Archives the session
    pub fn archive(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Archived;
        self.archived_at = Some(now);
        self.updated_at = now;
    }

    /// Marks the session as accepted (all proposals applied)
    pub fn mark_accepted(&mut self) {
        let now = Utc::now();
        self.status = IdeationSessionStatus::Accepted;
        self.converted_at = Some(now);
        self.updated_at = now;
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Validates that setting a parent session ID won't create a circular reference
    /// This is a domain validation that checks the proposed parent chain
    /// In practice, this would be called before persisting and would need access to a repository
    /// to walk the parent chain. The actual database-backed validation happens in the repository layer.
    pub fn validate_no_circular_parent(&self, proposed_parent_id: &IdeationSessionId) -> bool {
        // Self-reference is always invalid
        if self.id == *proposed_parent_id {
            return false;
        }

        // This method validates the logical constraint.
        // The actual parent chain walk happens at the repository level
        // where we have access to fetch parent sessions from the database.
        true
    }

    /// Deserialize an IdeationSession from a SQLite row
    /// Expects columns: id, project_id, title, status, plan_artifact_id, seed_task_id, parent_session_id, created_at, updated_at, archived_at, converted_at
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
            inherited_plan_artifact_id: row
                .get::<_, Option<String>>("inherited_plan_artifact_id")?
                .map(ArtifactId::from_string),
            seed_task_id: row
                .get::<_, Option<String>>("seed_task_id")?
                .map(TaskId::from_string),
            parent_session_id: row
                .get::<_, Option<String>>("parent_session_id")?
                .map(IdeationSessionId::from_string),
            created_at: Self::parse_datetime(row.get("created_at")?),
            updated_at: Self::parse_datetime(row.get("updated_at")?),
            archived_at: row
                .get::<_, Option<String>>("archived_at")?
                .map(Self::parse_datetime),
            converted_at: row
                .get::<_, Option<String>>("converted_at")?
                .map(Self::parse_datetime),
            team_mode: row.get::<_, Option<String>>("team_mode").unwrap_or(None),
            team_config_json: row
                .get::<_, Option<String>>("team_config_json")
                .unwrap_or(None),
            title_source: row.get::<_, Option<String>>("title_source").unwrap_or(None),
            verification_status: row
                .get::<_, Option<String>>("verification_status")
                .unwrap_or(None)
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            verification_in_progress: row
                .get::<_, Option<i64>>("verification_in_progress")
                .unwrap_or(None)
                .map(|v| v != 0)
                .unwrap_or(false),
            verification_metadata: row
                .get::<_, Option<String>>("verification_metadata")
                .unwrap_or(None),
            verification_generation: row
                .get::<_, Option<i64>>("verification_generation")
                .unwrap_or(None)
                .map(|v| v as i32)
                .unwrap_or(0),
            source_project_id: row
                .get::<_, Option<String>>("source_project_id")
                .unwrap_or(None),
            source_session_id: row
                .get::<_, Option<String>>("source_session_id")
                .unwrap_or(None),
        })
    }

    /// Parse a datetime string from SQLite into a DateTime<Utc>
    /// Handles both RFC3339 format and SQLite's CURRENT_TIMESTAMP format
    fn parse_datetime(s: String) -> DateTime<Utc> {
        parse_datetime_helper(s)
    }
}
