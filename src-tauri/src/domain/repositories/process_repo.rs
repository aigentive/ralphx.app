// Process repository trait - domain layer abstraction
//
// This trait defines the contract for ResearchProcess persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::research::{
    ResearchProcess, ResearchProcessId, ResearchProcessStatus,
};
use crate::error::AppResult;

/// Repository trait for ResearchProcess persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ProcessRepository: Send + Sync {
    /// Create a new research process
    async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess>;

    /// Get research process by ID
    async fn get_by_id(&self, id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>>;

    /// Get all research processes
    async fn get_all(&self) -> AppResult<Vec<ResearchProcess>>;

    /// Get research processes by status
    async fn get_by_status(&self, status: ResearchProcessStatus)
        -> AppResult<Vec<ResearchProcess>>;

    /// Get active research processes (pending or running)
    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>>;

    /// Update progress on a research process (iteration count, checkpoint, etc.)
    async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()>;

    /// Update the full research process
    async fn update(&self, process: &ResearchProcess) -> AppResult<()>;

    /// Mark a process as completed
    async fn complete(&self, id: &ResearchProcessId) -> AppResult<()>;

    /// Mark a process as failed with an error message
    async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()>;

    /// Delete a research process
    async fn delete(&self, id: &ResearchProcessId) -> AppResult<()>;

    /// Check if a process exists
    async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "process_repo_tests.rs"]
mod tests;
