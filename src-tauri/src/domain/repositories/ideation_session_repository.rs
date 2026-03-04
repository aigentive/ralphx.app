// Ideation session repository trait - domain layer abstraction
//
// This trait defines the contract for ideation session persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId,
};
use crate::error::AppResult;

/// Repository trait for IdeationSession persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait IdeationSessionRepository: Send + Sync {
    /// Create a new ideation session
    async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession>;

    /// Get session by ID
    async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>>;

    /// Get all sessions for a project, ordered by updated_at DESC
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>>;

    /// Update session status with appropriate timestamp updates
    async fn update_status(
        &self,
        id: &IdeationSessionId,
        status: IdeationSessionStatus,
    ) -> AppResult<()>;

    /// Update session title and source ("auto" for session-namer, "user" for manual rename)
    async fn update_title(
        &self,
        id: &IdeationSessionId,
        title: Option<String>,
        title_source: &str,
    ) -> AppResult<()>;

    /// Update session plan artifact ID
    async fn update_plan_artifact_id(
        &self,
        id: &IdeationSessionId,
        plan_artifact_id: Option<String>,
    ) -> AppResult<()>;

    /// Delete session (cascades to proposals and messages)
    async fn delete(&self, id: &IdeationSessionId) -> AppResult<()>;

    /// Get active sessions for a project
    async fn get_active_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Count sessions by status for a project
    async fn count_by_status(
        &self,
        project_id: &ProjectId,
        status: IdeationSessionStatus,
    ) -> AppResult<u32>;

    /// Get sessions that have a specific plan artifact ID
    /// Used when updating a plan artifact to find sessions to re-link
    async fn get_by_plan_artifact_id(
        &self,
        plan_artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Get sessions that have a specific inherited plan artifact ID
    /// Used in update_plan_artifact to detect and reject attempts to modify inherited plans
    async fn get_by_inherited_plan_artifact_id(
        &self,
        artifact_id: &str,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Get all child sessions for a given parent session ID
    async fn get_children(&self, parent_id: &IdeationSessionId) -> AppResult<Vec<IdeationSession>>;

    /// Get the ancestor chain for a session (parents, grandparents, etc.)
    /// Returns sessions in order from direct parent to root ancestor
    async fn get_ancestor_chain(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<IdeationSession>>;

    /// Set the parent session ID for a session
    async fn set_parent(
        &self,
        id: &IdeationSessionId,
        parent_id: Option<&IdeationSessionId>,
    ) -> AppResult<()>;
}

#[cfg(test)]
#[path = "ideation_session_repository_tests.rs"]
mod tests;
