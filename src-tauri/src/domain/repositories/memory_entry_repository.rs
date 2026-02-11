// Memory entry repository trait

use async_trait::async_trait;

use crate::domain::entities::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus, ProcessId};
use crate::error::AppResult;

/// Repository trait for MemoryEntry persistence
#[async_trait]
pub trait MemoryEntryRepository: Send + Sync {
    /// Create a new memory entry
    async fn create(&self, entry: MemoryEntry) -> AppResult<MemoryEntry>;

    /// Get memory entry by ID
    async fn get_by_id(&self, id: &MemoryEntryId) -> AppResult<Option<MemoryEntry>>;

    /// Find memory entry by content hash
    async fn find_by_content_hash(
        &self,
        project_id: &ProcessId,
        bucket: &MemoryBucket,
        content_hash: &str,
    ) -> AppResult<Option<MemoryEntry>>;

    /// Get all active memory entries for a project
    async fn get_by_project(&self, project_id: &ProcessId) -> AppResult<Vec<MemoryEntry>>;

    /// Get active memory entries by project and bucket
    async fn get_by_project_and_bucket(
        &self,
        project_id: &ProcessId,
        bucket: &MemoryBucket,
    ) -> AppResult<Vec<MemoryEntry>>;

    /// Update memory entry status
    async fn update_status(&self, id: &MemoryEntryId, status: MemoryStatus) -> AppResult<()>;

    /// Update memory entry
    async fn update(&self, entry: &MemoryEntry) -> AppResult<()>;

    /// Delete memory entry
    async fn delete(&self, id: &MemoryEntryId) -> AppResult<()>;

    /// Get all memories matching any of the given paths (glob pattern matching)
    async fn get_by_paths(
        &self,
        project_id: &ProcessId,
        paths: &[String],
    ) -> AppResult<Vec<MemoryEntry>>;
}
