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
mod tests {
    use super::*;
    use crate::domain::entities::ArtifactType;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockArtifactBucketRepository {
        return_bucket: Option<ArtifactBucket>,
        buckets: Vec<ArtifactBucket>,
    }

    impl MockArtifactBucketRepository {
        fn new() -> Self {
            Self {
                return_bucket: None,
                buckets: vec![],
            }
        }

        fn with_bucket(bucket: ArtifactBucket) -> Self {
            Self {
                return_bucket: Some(bucket.clone()),
                buckets: vec![bucket],
            }
        }

        fn with_buckets(buckets: Vec<ArtifactBucket>) -> Self {
            Self {
                return_bucket: buckets.first().cloned(),
                buckets,
            }
        }
    }

    #[async_trait]
    impl ArtifactBucketRepository for MockArtifactBucketRepository {
        async fn create(&self, bucket: ArtifactBucket) -> AppResult<ArtifactBucket> {
            Ok(bucket)
        }

        async fn get_by_id(&self, _id: &ArtifactBucketId) -> AppResult<Option<ArtifactBucket>> {
            Ok(self.return_bucket.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<ArtifactBucket>> {
            Ok(self.buckets.clone())
        }

        async fn get_system_buckets(&self) -> AppResult<Vec<ArtifactBucket>> {
            Ok(self.buckets.iter().filter(|b| b.is_system).cloned().collect())
        }

        async fn update(&self, _bucket: &ArtifactBucket) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ArtifactBucketId) -> AppResult<()> {
            Ok(())
        }

        async fn exists(&self, id: &ArtifactBucketId) -> AppResult<bool> {
            Ok(self.buckets.iter().any(|b| b.id == *id))
        }
    }

    fn create_test_bucket() -> ArtifactBucket {
        ArtifactBucket::new("Test Bucket")
            .accepts(ArtifactType::Prd)
            .accepts(ArtifactType::DesignDoc)
            .with_writer("user")
            .with_writer("orchestrator")
    }

    fn create_system_bucket() -> ArtifactBucket {
        ArtifactBucket::system("research-outputs", "Research Outputs")
            .accepts(ArtifactType::ResearchDocument)
            .accepts(ArtifactType::Findings)
            .with_writer("deep-researcher")
    }

    #[test]
    fn test_artifact_bucket_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn ArtifactBucketRepository> =
            Arc::new(MockArtifactBucketRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_create() {
        let repo = MockArtifactBucketRepository::new();
        let bucket = create_test_bucket();

        let result = repo.create(bucket.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, bucket.id);
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_by_id_returns_none() {
        let repo = MockArtifactBucketRepository::new();
        let bucket_id = ArtifactBucketId::new();

        let result = repo.get_by_id(&bucket_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_by_id_returns_bucket() {
        let bucket = create_test_bucket();
        let repo = MockArtifactBucketRepository::with_bucket(bucket.clone());

        let result = repo.get_by_id(&bucket.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, bucket.id);
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_all_empty() {
        let repo = MockArtifactBucketRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_all_with_buckets() {
        let bucket1 = create_test_bucket();
        let bucket2 = create_system_bucket();
        let repo = MockArtifactBucketRepository::with_buckets(vec![bucket1.clone(), bucket2.clone()]);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let buckets = result.unwrap();
        assert_eq!(buckets.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_system_buckets_empty() {
        let bucket = create_test_bucket(); // not a system bucket
        let repo = MockArtifactBucketRepository::with_bucket(bucket);

        let result = repo.get_system_buckets().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_get_system_buckets_returns_only_system() {
        let custom = create_test_bucket();
        let system = create_system_bucket();
        let repo = MockArtifactBucketRepository::with_buckets(vec![custom, system.clone()]);

        let result = repo.get_system_buckets().await;
        assert!(result.is_ok());
        let buckets = result.unwrap();
        assert_eq!(buckets.len(), 1);
        assert!(buckets[0].is_system);
        assert_eq!(buckets[0].id, system.id);
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_update() {
        let repo = MockArtifactBucketRepository::new();
        let bucket = create_test_bucket();

        let result = repo.update(&bucket).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_delete() {
        let repo = MockArtifactBucketRepository::new();
        let bucket_id = ArtifactBucketId::new();

        let result = repo.delete(&bucket_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_exists_true() {
        let bucket = create_test_bucket();
        let repo = MockArtifactBucketRepository::with_bucket(bucket.clone());

        let result = repo.exists(&bucket.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_bucket_repository_exists_false() {
        let repo = MockArtifactBucketRepository::new();
        let bucket_id = ArtifactBucketId::new();

        let result = repo.exists(&bucket_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_bucket_repository_trait_object_in_arc() {
        let bucket = create_test_bucket();
        let repo: Arc<dyn ArtifactBucketRepository> =
            Arc::new(MockArtifactBucketRepository::with_bucket(bucket.clone()));

        // Use through trait object
        let result = repo.get_by_id(&bucket.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_bucket_with_accepted_types() {
        let bucket = ArtifactBucket::new("Code Bucket")
            .accepts(ArtifactType::CodeChange)
            .accepts(ArtifactType::Diff)
            .accepts(ArtifactType::TestResult);

        assert!(bucket.accepts_type(ArtifactType::CodeChange));
        assert!(bucket.accepts_type(ArtifactType::Diff));
        assert!(bucket.accepts_type(ArtifactType::TestResult));
        assert!(!bucket.accepts_type(ArtifactType::Prd));
    }

    #[tokio::test]
    async fn test_bucket_with_writers() {
        let bucket = ArtifactBucket::new("Worker Bucket")
            .with_writer("worker")
            .with_writer("reviewer");

        assert!(bucket.can_write("worker"));
        assert!(bucket.can_write("reviewer"));
        assert!(!bucket.can_write("user"));
    }

    #[tokio::test]
    async fn test_bucket_with_readers() {
        let bucket = ArtifactBucket::new("Private Bucket")
            .with_reader("orchestrator");

        // Default has "all", so it can read
        assert!(bucket.can_read("orchestrator"));
        assert!(bucket.can_read("all"));
    }

    #[tokio::test]
    async fn test_system_bucket_creation() {
        let bucket = ArtifactBucket::system("work-context", "Work Context");

        assert_eq!(bucket.id.as_str(), "work-context");
        assert_eq!(bucket.name, "Work Context");
        assert!(bucket.is_system);
    }

    #[tokio::test]
    async fn test_all_four_system_buckets() {
        let system_buckets = ArtifactBucket::system_buckets();

        assert_eq!(system_buckets.len(), 4);

        let ids: Vec<&str> = system_buckets.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&"research-outputs"));
        assert!(ids.contains(&"work-context"));
        assert!(ids.contains(&"code-changes"));
        assert!(ids.contains(&"prd-library"));

        // All should be system buckets
        for bucket in &system_buckets {
            assert!(bucket.is_system);
        }
    }

    #[tokio::test]
    async fn test_research_outputs_bucket_config() {
        let buckets = ArtifactBucket::system_buckets();
        let research = buckets.iter().find(|b| b.id.as_str() == "research-outputs").unwrap();

        assert!(research.accepts_type(ArtifactType::ResearchDocument));
        assert!(research.accepts_type(ArtifactType::Findings));
        assert!(research.accepts_type(ArtifactType::Recommendations));
        assert!(research.can_write("deep-researcher"));
        assert!(research.can_write("orchestrator"));
    }

    #[tokio::test]
    async fn test_work_context_bucket_config() {
        let buckets = ArtifactBucket::system_buckets();
        let work = buckets.iter().find(|b| b.id.as_str() == "work-context").unwrap();

        assert!(work.accepts_type(ArtifactType::Context));
        assert!(work.accepts_type(ArtifactType::TaskSpec));
        assert!(work.accepts_type(ArtifactType::PreviousWork));
        assert!(work.can_write("orchestrator"));
        assert!(work.can_write("system"));
    }

    #[tokio::test]
    async fn test_code_changes_bucket_config() {
        let buckets = ArtifactBucket::system_buckets();
        let code = buckets.iter().find(|b| b.id.as_str() == "code-changes").unwrap();

        assert!(code.accepts_type(ArtifactType::CodeChange));
        assert!(code.accepts_type(ArtifactType::Diff));
        assert!(code.accepts_type(ArtifactType::TestResult));
        assert!(code.can_write("worker"));
    }

    #[tokio::test]
    async fn test_prd_library_bucket_config() {
        let buckets = ArtifactBucket::system_buckets();
        let prd = buckets.iter().find(|b| b.id.as_str() == "prd-library").unwrap();

        assert!(prd.accepts_type(ArtifactType::Prd));
        assert!(prd.accepts_type(ArtifactType::Specification));
        assert!(prd.accepts_type(ArtifactType::DesignDoc));
        assert!(prd.can_write("orchestrator"));
        assert!(prd.can_write("user"));
    }
}
