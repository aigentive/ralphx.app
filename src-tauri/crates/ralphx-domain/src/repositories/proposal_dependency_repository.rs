// Proposal dependency repository trait - domain layer abstraction
//
// This trait defines the contract for proposal dependency persistence.
// Dependencies track which proposals depend on other proposals within a session.

use async_trait::async_trait;

use crate::domain::entities::{IdeationSessionId, TaskProposalId};
use crate::error::AppResult;

/// Repository trait for proposal dependency persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ProposalDependencyRepository: Send + Sync {
    /// Add a dependency (proposal_id depends on depends_on_id)
    /// source: "auto" for AI-suggested, "manual" for user-created, defaults to "auto"
    async fn add_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
        reason: Option<&str>,
        source: Option<&str>,
    ) -> AppResult<()>;

    /// Remove a dependency
    async fn remove_dependency(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<()>;

    /// Get all proposals that this proposal depends on
    async fn get_dependencies(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<Vec<TaskProposalId>>;

    /// Get all proposals that depend on this proposal
    async fn get_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>>;

    /// Get all dependency relationships for a session
    /// Returns tuples of (proposal_id, depends_on_proposal_id, reason)
    async fn get_all_for_session(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>)>>;

    /// Get all dependency relationships for a session with source field
    /// Returns tuples of (proposal_id, depends_on_proposal_id, reason, source)
    async fn get_all_for_session_with_source(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<(TaskProposalId, TaskProposalId, Option<String>, String)>>;

    /// Check if adding a dependency would create a cycle
    async fn would_create_cycle(
        &self,
        proposal_id: &TaskProposalId,
        depends_on_id: &TaskProposalId,
    ) -> AppResult<bool>;

    /// Clear all dependencies for a proposal (both directions)
    async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()>;

    /// Clear all dependencies for all proposals in a session
    async fn clear_session_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Clear only auto-suggested dependencies for all proposals in a session
    /// Preserves manually-added dependencies (source != 'auto')
    async fn clear_auto_dependencies(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Count dependencies for a proposal (how many it depends on)
    async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32>;

    /// Count dependents for a proposal (how many depend on it)
    async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32>;
}

#[cfg(test)]
#[path = "proposal_dependency_repository_tests.rs"]
mod tests;
