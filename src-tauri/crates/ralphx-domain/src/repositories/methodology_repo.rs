// Methodology repository trait - domain layer abstraction
//
// This trait defines the contract for MethodologyExtension persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::methodology::{MethodologyExtension, MethodologyId};
use crate::error::AppResult;

/// Repository trait for MethodologyExtension persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait MethodologyRepository: Send + Sync {
    /// Create a new methodology extension
    async fn create(&self, methodology: MethodologyExtension) -> AppResult<MethodologyExtension>;

    /// Get methodology by ID
    async fn get_by_id(&self, id: &MethodologyId) -> AppResult<Option<MethodologyExtension>>;

    /// Get all methodologies
    async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>>;

    /// Get the currently active methodology (if any)
    async fn get_active(&self) -> AppResult<Option<MethodologyExtension>>;

    /// Activate a methodology (deactivates any currently active one)
    async fn activate(&self, id: &MethodologyId) -> AppResult<()>;

    /// Deactivate a methodology
    async fn deactivate(&self, id: &MethodologyId) -> AppResult<()>;

    /// Update a methodology
    async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()>;

    /// Delete a methodology
    async fn delete(&self, id: &MethodologyId) -> AppResult<()>;

    /// Check if a methodology exists
    async fn exists(&self, id: &MethodologyId) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "methodology_repo_tests.rs"]
mod tests;
