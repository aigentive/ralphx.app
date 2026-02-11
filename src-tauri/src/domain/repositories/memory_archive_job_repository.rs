// Memory archive job repository trait

use async_trait::async_trait;

use crate::domain::entities::{
    MemoryArchiveJob, MemoryArchiveJobId, MemoryArchiveJobStatus, ProcessId,
};
use crate::error::AppResult;

/// Repository trait for MemoryArchiveJob persistence
#[async_trait]
pub trait MemoryArchiveJobRepository: Send + Sync {
    /// Create a new archive job
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob>;

    /// Get job by ID
    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>>;

    /// Get all pending jobs for a project
    async fn get_pending_by_project(&self, project_id: &ProcessId)
        -> AppResult<Vec<MemoryArchiveJob>>;

    /// Update job status
    async fn update_status(
        &self,
        id: &MemoryArchiveJobId,
        status: MemoryArchiveJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()>;
}
