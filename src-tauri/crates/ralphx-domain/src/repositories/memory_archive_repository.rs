// MemoryArchiveRepository trait - domain layer abstraction for archive job persistence
//
// This trait defines the contract for memory archive job data persistence.
// Implementations can use SQLite, in-memory storage, etc.

use async_trait::async_trait;

use crate::domain::entities::types::ProjectId;
use crate::domain::entities::{
    ArchiveJobStatus, ArchiveJobType, MemoryArchiveJob, MemoryArchiveJobId,
};
use crate::error::AppResult;

/// Repository trait for MemoryArchiveJob persistence.
/// Provides CRUD operations and job queue management.
#[async_trait]
pub trait MemoryArchiveRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create a new archive job
    async fn create(&self, job: MemoryArchiveJob) -> AppResult<MemoryArchiveJob>;

    /// Get job by ID
    async fn get_by_id(&self, id: &MemoryArchiveJobId) -> AppResult<Option<MemoryArchiveJob>>;

    /// Update an archive job
    async fn update(&self, job: &MemoryArchiveJob) -> AppResult<()>;

    /// Delete a job
    async fn delete(&self, id: &MemoryArchiveJobId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // Query Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Get all jobs for a project
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryArchiveJob>>;

    /// Get jobs by status
    async fn get_by_status(&self, status: ArchiveJobStatus) -> AppResult<Vec<MemoryArchiveJob>>;

    /// Get jobs by status for a specific project
    async fn get_by_project_and_status(
        &self,
        project_id: &ProjectId,
        status: ArchiveJobStatus,
    ) -> AppResult<Vec<MemoryArchiveJob>>;

    /// Get jobs by type for a specific project
    async fn get_by_project_and_type(
        &self,
        project_id: &ProjectId,
        job_type: ArchiveJobType,
    ) -> AppResult<Vec<MemoryArchiveJob>>;

    // ═══════════════════════════════════════════════════════════════════════
    // Job Queue Management
    // ═══════════════════════════════════════════════════════════════════════

    /// Claim the next pending or failed job for processing
    /// Atomically transitions the job to 'running' status
    /// Returns None if no claimable jobs exist
    async fn claim_next(&self) -> AppResult<Option<MemoryArchiveJob>>;

    /// Claim the next pending or failed job for a specific project
    /// Atomically transitions the job to 'running' status
    /// Returns None if no claimable jobs exist
    async fn claim_next_for_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Option<MemoryArchiveJob>>;

    /// Get count of jobs by status
    async fn count_by_status(&self, status: ArchiveJobStatus) -> AppResult<u32>;

    /// Get count of pending or failed jobs (claimable jobs)
    async fn count_claimable(&self) -> AppResult<u32>;

    /// Get count of claimable jobs for a specific project
    async fn count_claimable_for_project(&self, project_id: &ProjectId) -> AppResult<u32>;

    // ═══════════════════════════════════════════════════════════════════════
    // Cleanup Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Delete all completed jobs older than the specified days
    async fn delete_completed_older_than(&self, days: u32) -> AppResult<u32>;

    /// Delete all jobs for a project
    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()>;
}
