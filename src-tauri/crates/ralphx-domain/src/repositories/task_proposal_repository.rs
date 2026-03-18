// Task proposal repository trait - domain layer abstraction
//
// This trait defines the contract for task proposal persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{
    ArtifactId, IdeationSessionId, PriorityAssessment, TaskId, TaskProposal, TaskProposalId,
};
use crate::error::AppResult;

/// Repository trait for TaskProposal persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskProposalRepository: Send + Sync {
    /// Create a new task proposal
    async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal>;

    /// Get proposal by ID
    async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>>;

    /// Get all proposals for a session, ordered by sort_order
    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>>;

    /// Update an existing proposal
    async fn update(&self, proposal: &TaskProposal) -> AppResult<()>;

    /// Update priority assessment for a proposal
    async fn update_priority(
        &self,
        id: &TaskProposalId,
        assessment: &PriorityAssessment,
    ) -> AppResult<()>;

    /// Update selection state for a proposal
    async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()>;

    /// Set the created task ID after converting proposal to task
    async fn set_created_task_id(&self, id: &TaskProposalId, task_id: &TaskId) -> AppResult<()>;

    /// Delete a proposal
    async fn delete(&self, id: &TaskProposalId) -> AppResult<()>;

    /// Reorder proposals within a session
    /// Updates sort_order for each proposal based on position in the provided list
    async fn reorder(
        &self,
        session_id: &IdeationSessionId,
        proposal_ids: Vec<TaskProposalId>,
    ) -> AppResult<()>;

    /// Get selected proposals for a session
    async fn get_selected_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<TaskProposal>>;

    /// Count proposals by session
    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;

    /// Count selected proposals by session
    async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;

    /// Get proposals linked to a plan artifact
    /// Used by proactive sync to find proposals that may need updating when a plan changes
    async fn get_by_plan_artifact_id(
        &self,
        artifact_id: &ArtifactId,
    ) -> AppResult<Vec<TaskProposal>>;

    /// Clear the created_task_id for all proposals in a session
    /// Used when reopening an ideation session to unlink proposals from deleted tasks
    async fn clear_created_task_ids_by_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<()>;

    /// Archive a proposal (soft delete)
    async fn archive(&self, id: &TaskProposalId) -> AppResult<TaskProposal>;

    /// Archive a proposal within an existing transaction (synchronous)
    ///
    /// # Default implementation
    /// Panics — only SQLite repositories support transactional sync operations.
    /// Memory repositories use `archive()` instead.
    fn archive_sync(
        &self,
        _conn: &rusqlite::Connection,
        _id: &TaskProposalId,
    ) -> AppResult<TaskProposal> {
        unimplemented!("archive_sync is only supported by SQLite repositories")
    }
}

#[cfg(test)]
#[path = "task_proposal_repository_tests.rs"]
mod tests;
