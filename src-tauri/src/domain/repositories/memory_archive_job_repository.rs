// Memory archive job repository trait (legacy, kept for backward compatibility)
// New code should use MemoryArchiveRepository from memory_archive_repository.rs

use async_trait::async_trait;

use crate::domain::entities::{
    ArchiveJobStatus, MemoryArchiveJob, MemoryArchiveJobId,
};
use crate::domain::entities::types::ProjectId;
use crate::error::AppResult;

/// Legacy repository trait for MemoryArchiveJob persistence
/// New code should use MemoryArchiveRepository instead
#[async_trait]
pub trait MemoryArchiveJobRepository: Send + Sync {
    /// Create a new archive job
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob>;

    /// Get job by ID
    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>>;

    /// Get all pending jobs for a project
    async fn get_pending_by_project(&self, project_id: &ProjectId)
        -> AppResult<Vec<MemoryArchiveJob>>;

    /// Update job status
    async fn update_status(
        &self,
        id: &MemoryArchiveJobId,
        status: ArchiveJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()>;
}
