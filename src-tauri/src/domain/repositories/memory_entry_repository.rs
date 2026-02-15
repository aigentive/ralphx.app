// MemoryEntryRepository trait - domain layer abstraction for memory entry persistence
//
// This trait defines the contract for memory entry data persistence.
// Implementations can use SQLite, in-memory storage, etc.

use async_trait::async_trait;

use crate::domain::entities::types::ProjectId;
use crate::domain::entities::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus};
use crate::error::AppResult;

/// Repository trait for MemoryEntry persistence.
/// Provides CRUD operations and queries for memory entries.
#[async_trait]
pub trait MemoryEntryRepository: Send + Sync {
    /// Create a new memory entry
    async fn create(&self, entry: MemoryEntry) -> AppResult<MemoryEntry>;

    /// Get memory entry by ID
    async fn get_by_id(&self, id: &MemoryEntryId) -> AppResult<Option<MemoryEntry>>;

    /// Find memory entry by content hash (for deduplication)
    async fn find_by_content_hash(
        &self,
        project_id: &ProjectId,
        bucket: &MemoryBucket,
        content_hash: &str,
    ) -> AppResult<Option<MemoryEntry>>;

    /// Get all memory entries for a project
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryEntry>>;

    /// Get memory entries by project and status
    async fn get_by_project_and_status(
        &self,
        project_id: &ProjectId,
        status: MemoryStatus,
    ) -> AppResult<Vec<MemoryEntry>>;

    /// Get memory entries by project and bucket
    async fn get_by_project_and_bucket(
        &self,
        project_id: &ProjectId,
        bucket: MemoryBucket,
    ) -> AppResult<Vec<MemoryEntry>>;

    /// Get memory entries linked to a rule file (by source_rule_file)
    async fn get_by_rule_file(
        &self,
        project_id: &ProjectId,
        rule_file: &str,
    ) -> AppResult<Vec<MemoryEntry>>;

    /// Get all memory entries with a specific content hash
    async fn get_by_content_hash(&self, content_hash: &str) -> AppResult<Vec<MemoryEntry>>;

    /// Update memory entry status
    async fn update_status(&self, id: &MemoryEntryId, status: MemoryStatus) -> AppResult<()>;

    /// Update memory entry
    async fn update(&self, entry: &MemoryEntry) -> AppResult<()>;

    /// Delete memory entry
    async fn delete(&self, id: &MemoryEntryId) -> AppResult<()>;

    /// Get all memories matching any of the given paths (glob pattern matching)
    async fn get_by_paths(
        &self,
        project_id: &ProjectId,
        paths: &[String],
    ) -> AppResult<Vec<MemoryEntry>>;
}
