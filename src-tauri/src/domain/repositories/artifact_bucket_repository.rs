// Artifact bucket repository trait - domain layer abstraction
//
// This trait defines the contract for artifact bucket persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{ArtifactBucket, ArtifactBucketId};
use crate::error::AppResult;

/// Repository trait for ArtifactBucket persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ArtifactBucketRepository: Send + Sync {
    /// Create a new bucket
    async fn create(&self, bucket: ArtifactBucket) -> AppResult<ArtifactBucket>;

    /// Get bucket by ID
    async fn get_by_id(&self, id: &ArtifactBucketId) -> AppResult<Option<ArtifactBucket>>;

    /// Get all buckets
    async fn get_all(&self) -> AppResult<Vec<ArtifactBucket>>;

    /// Get all system buckets (is_system = true)
    async fn get_system_buckets(&self) -> AppResult<Vec<ArtifactBucket>>;

    /// Update a bucket
    async fn update(&self, bucket: &ArtifactBucket) -> AppResult<()>;

    /// Delete a bucket (fails if system bucket)
    async fn delete(&self, id: &ArtifactBucketId) -> AppResult<()>;

    /// Check if a bucket exists
    async fn exists(&self, id: &ArtifactBucketId) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "artifact_bucket_repository_tests.rs"]
mod tests;
