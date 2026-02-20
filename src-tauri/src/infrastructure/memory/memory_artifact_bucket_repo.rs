// Memory-based ArtifactBucketRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{ArtifactBucket, ArtifactBucketId};
use crate::domain::repositories::ArtifactBucketRepository;
use crate::error::{AppError, AppResult};

/// In-memory implementation of ArtifactBucketRepository for testing
pub struct MemoryArtifactBucketRepository {
    buckets: Arc<RwLock<HashMap<ArtifactBucketId, ArtifactBucket>>>,
}

impl Default for MemoryArtifactBucketRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryArtifactBucketRepository {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_buckets(buckets: Vec<ArtifactBucket>) -> Self {
        let map: HashMap<ArtifactBucketId, ArtifactBucket> =
            buckets.into_iter().map(|b| (b.id.clone(), b)).collect();
        Self {
            buckets: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl ArtifactBucketRepository for MemoryArtifactBucketRepository {
    async fn create(&self, bucket: ArtifactBucket) -> AppResult<ArtifactBucket> {
        let mut buckets = self.buckets.write().await;
        buckets.insert(bucket.id.clone(), bucket.clone());
        Ok(bucket)
    }

    async fn get_by_id(&self, id: &ArtifactBucketId) -> AppResult<Option<ArtifactBucket>> {
        let buckets = self.buckets.read().await;
        Ok(buckets.get(id).cloned())
    }

    async fn get_all(&self) -> AppResult<Vec<ArtifactBucket>> {
        let buckets = self.buckets.read().await;
        let mut result: Vec<ArtifactBucket> = buckets.values().cloned().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_system_buckets(&self) -> AppResult<Vec<ArtifactBucket>> {
        let buckets = self.buckets.read().await;
        Ok(buckets.values().filter(|b| b.is_system).cloned().collect())
    }

    async fn update(&self, bucket: &ArtifactBucket) -> AppResult<()> {
        let mut buckets = self.buckets.write().await;
        buckets.insert(bucket.id.clone(), bucket.clone());
        Ok(())
    }

    async fn delete(&self, id: &ArtifactBucketId) -> AppResult<()> {
        let buckets = self.buckets.read().await;
        if let Some(bucket) = buckets.get(id) {
            if bucket.is_system {
                return Err(AppError::Validation(
                    "Cannot delete system bucket".to_string(),
                ));
            }
        }
        drop(buckets);

        let mut buckets = self.buckets.write().await;
        buckets.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &ArtifactBucketId) -> AppResult<bool> {
        let buckets = self.buckets.read().await;
        Ok(buckets.contains_key(id))
    }
}

#[cfg(test)]
#[path = "memory_artifact_bucket_repo_tests.rs"]
mod tests;
