// Artifact flow repository trait - domain layer abstraction
//
// This trait defines the contract for artifact flow persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{ArtifactFlow, ArtifactFlowId};
use crate::error::AppResult;

/// Repository trait for ArtifactFlow persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ArtifactFlowRepository: Send + Sync {
    /// Create a new artifact flow
    async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow>;

    /// Get artifact flow by ID
    async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>>;

    /// Get all artifact flows
    async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>>;

    /// Get all active artifact flows (is_active = true)
    async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>>;

    /// Update an artifact flow
    async fn update(&self, flow: &ArtifactFlow) -> AppResult<()>;

    /// Delete an artifact flow
    async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()>;

    /// Set the active state of a flow
    async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()>;

    /// Check if a flow exists
    async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "artifact_flow_repository_tests.rs"]
mod tests;
