// Artifact repository trait - domain layer abstraction
//
// This trait defines the contract for artifact persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactId, ArtifactRelation, ArtifactRelationType, ArtifactType,
    ProcessId, TaskId,
};
use crate::error::AppResult;

/// Repository trait for Artifact persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    /// Create a new artifact
    async fn create(&self, artifact: Artifact) -> AppResult<Artifact>;

    /// Get artifact by ID
    async fn get_by_id(&self, id: &ArtifactId) -> AppResult<Option<Artifact>>;

    /// Get artifact at a specific version by traversing the version history
    /// Returns None if the artifact doesn't exist or the version is not found
    async fn get_by_id_at_version(&self, id: &ArtifactId, version: u32) -> AppResult<Option<Artifact>>;

    /// Get all artifacts in a bucket
    async fn get_by_bucket(&self, bucket_id: &ArtifactBucketId) -> AppResult<Vec<Artifact>>;

    /// Get all artifacts of a specific type
    async fn get_by_type(&self, artifact_type: ArtifactType) -> AppResult<Vec<Artifact>>;

    /// Get all artifacts for a task
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<Artifact>>;

    /// Get all artifacts for a process
    async fn get_by_process(&self, process_id: &ProcessId) -> AppResult<Vec<Artifact>>;

    /// Update an artifact
    async fn update(&self, artifact: &Artifact) -> AppResult<()>;

    /// Delete an artifact
    async fn delete(&self, id: &ArtifactId) -> AppResult<()>;

    /// Get all artifacts that this artifact was derived from
    async fn get_derived_from(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>>;

    /// Get all related artifacts (both directions)
    async fn get_related(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>>;

    /// Add a relation between two artifacts
    async fn add_relation(&self, relation: ArtifactRelation) -> AppResult<ArtifactRelation>;

    /// Get relations for an artifact
    async fn get_relations(
        &self,
        artifact_id: &ArtifactId,
    ) -> AppResult<Vec<ArtifactRelation>>;

    /// Get relations of a specific type for an artifact
    async fn get_relations_by_type(
        &self,
        artifact_id: &ArtifactId,
        relation_type: ArtifactRelationType,
    ) -> AppResult<Vec<ArtifactRelation>>;

    /// Delete a relation
    async fn delete_relation(
        &self,
        from_id: &ArtifactId,
        to_id: &ArtifactId,
    ) -> AppResult<()>;

    /// Create a new artifact with a link to the previous version
    /// This is used for version chaining in plan artifacts
    async fn create_with_previous_version(
        &self,
        artifact: Artifact,
        previous_version_id: ArtifactId,
    ) -> AppResult<Artifact>;

    /// Get version history for an artifact by walking the previous_version_id chain
    /// Returns summaries in order from newest to oldest
    async fn get_version_history(&self, id: &ArtifactId) -> AppResult<Vec<ArtifactVersionSummary>>;

    /// Walk the version chain forward from any artifact ID to find the latest version.
    /// Given A(v1) → B(v2) → C(v3): resolve_latest(A) → C, resolve_latest(C) → C.
    async fn resolve_latest_artifact_id(&self, id: &ArtifactId) -> AppResult<ArtifactId>;
}

/// Summary of an artifact version for history display
#[derive(Debug, Clone)]
pub struct ArtifactVersionSummary {
    pub id: ArtifactId,
    pub version: u32,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ArtifactContent;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockArtifactRepository {
        return_artifact: Option<Artifact>,
    }

    impl MockArtifactRepository {
        fn new() -> Self {
            Self {
                return_artifact: None,
            }
        }

        fn with_artifact(artifact: Artifact) -> Self {
            Self {
                return_artifact: Some(artifact),
            }
        }
    }

    #[async_trait]
    impl ArtifactRepository for MockArtifactRepository {
        async fn create(&self, artifact: Artifact) -> AppResult<Artifact> {
            Ok(artifact)
        }

        async fn get_by_id(&self, _id: &ArtifactId) -> AppResult<Option<Artifact>> {
            Ok(self.return_artifact.clone())
        }

        async fn get_by_id_at_version(&self, _id: &ArtifactId, _version: u32) -> AppResult<Option<Artifact>> {
            Ok(self.return_artifact.clone())
        }

        async fn get_by_bucket(&self, _bucket_id: &ArtifactBucketId) -> AppResult<Vec<Artifact>> {
            match &self.return_artifact {
                Some(a) => Ok(vec![a.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn get_by_type(&self, _artifact_type: ArtifactType) -> AppResult<Vec<Artifact>> {
            match &self.return_artifact {
                Some(a) => Ok(vec![a.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn get_by_task(&self, _task_id: &TaskId) -> AppResult<Vec<Artifact>> {
            match &self.return_artifact {
                Some(a) => Ok(vec![a.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn get_by_process(&self, _process_id: &ProcessId) -> AppResult<Vec<Artifact>> {
            match &self.return_artifact {
                Some(a) => Ok(vec![a.clone()]),
                None => Ok(vec![]),
            }
        }

        async fn update(&self, _artifact: &Artifact) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ArtifactId) -> AppResult<()> {
            Ok(())
        }

        async fn get_derived_from(&self, _artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn get_related(&self, _artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn add_relation(&self, relation: ArtifactRelation) -> AppResult<ArtifactRelation> {
            Ok(relation)
        }

        async fn get_relations(
            &self,
            _artifact_id: &ArtifactId,
        ) -> AppResult<Vec<ArtifactRelation>> {
            Ok(vec![])
        }

        async fn get_relations_by_type(
            &self,
            _artifact_id: &ArtifactId,
            _relation_type: ArtifactRelationType,
        ) -> AppResult<Vec<ArtifactRelation>> {
            Ok(vec![])
        }

        async fn delete_relation(
            &self,
            _from_id: &ArtifactId,
            _to_id: &ArtifactId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn create_with_previous_version(
            &self,
            artifact: Artifact,
            _previous_version_id: ArtifactId,
        ) -> AppResult<Artifact> {
            Ok(artifact)
        }

        async fn get_version_history(&self, _id: &ArtifactId) -> AppResult<Vec<ArtifactVersionSummary>> {
            Ok(vec![])
        }

        async fn resolve_latest_artifact_id(&self, id: &ArtifactId) -> AppResult<ArtifactId> {
            Ok(id.clone())
        }
    }

    fn create_test_artifact() -> Artifact {
        Artifact::new_inline("Test PRD", ArtifactType::Prd, "Test content", "user")
    }

    #[test]
    fn test_artifact_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn ArtifactRepository> = Arc::new(MockArtifactRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_create() {
        let repo = MockArtifactRepository::new();
        let artifact = create_test_artifact();

        let result = repo.create(artifact.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, artifact.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_id_returns_none() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo.get_by_id(&artifact_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_id_returns_artifact() {
        let artifact = create_test_artifact();
        let repo = MockArtifactRepository::with_artifact(artifact.clone());

        let result = repo.get_by_id(&artifact.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, artifact.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_bucket_empty() {
        let repo = MockArtifactRepository::new();
        let bucket_id = ArtifactBucketId::new();

        let result = repo.get_by_bucket(&bucket_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_bucket_with_artifact() {
        let artifact = create_test_artifact();
        let repo = MockArtifactRepository::with_artifact(artifact.clone());
        let bucket_id = ArtifactBucketId::new();

        let result = repo.get_by_bucket(&bucket_id).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].id, artifact.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_type_empty() {
        let repo = MockArtifactRepository::new();

        let result = repo.get_by_type(ArtifactType::Prd).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_type_with_artifact() {
        let artifact = create_test_artifact();
        let repo = MockArtifactRepository::with_artifact(artifact.clone());

        let result = repo.get_by_type(ArtifactType::Prd).await;
        assert!(result.is_ok());
        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_task_empty() {
        let repo = MockArtifactRepository::new();
        let task_id = TaskId::from_string("task-1".to_string());

        let result = repo.get_by_task(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_task_with_artifact() {
        let artifact = create_test_artifact();
        let repo = MockArtifactRepository::with_artifact(artifact.clone());
        let task_id = TaskId::from_string("task-1".to_string());

        let result = repo.get_by_task(&task_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_by_process_empty() {
        let repo = MockArtifactRepository::new();
        let process_id = ProcessId::new();

        let result = repo.get_by_process(&process_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_update() {
        let repo = MockArtifactRepository::new();
        let artifact = create_test_artifact();

        let result = repo.update(&artifact).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_delete() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo.delete(&artifact_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_derived_from() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo.get_derived_from(&artifact_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_related() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo.get_related(&artifact_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_add_relation() {
        let repo = MockArtifactRepository::new();
        let relation = ArtifactRelation::derived_from(
            ArtifactId::from_string("derived"),
            ArtifactId::from_string("source"),
        );

        let result = repo.add_relation(relation.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, relation.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_relations() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo.get_relations(&artifact_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_get_relations_by_type() {
        let repo = MockArtifactRepository::new();
        let artifact_id = ArtifactId::new();

        let result = repo
            .get_relations_by_type(&artifact_id, ArtifactRelationType::DerivedFrom)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_repository_delete_relation() {
        let repo = MockArtifactRepository::new();
        let from_id = ArtifactId::new();
        let to_id = ArtifactId::new();

        let result = repo.delete_relation(&from_id, &to_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_artifact_repository_trait_object_in_arc() {
        let artifact = create_test_artifact();
        let repo: Arc<dyn ArtifactRepository> =
            Arc::new(MockArtifactRepository::with_artifact(artifact.clone()));

        // Use through trait object
        let result = repo.get_by_id(&artifact.id).await;
        assert!(result.is_ok());

        let by_type = repo.get_by_type(ArtifactType::Prd).await;
        assert!(by_type.is_ok());
        assert_eq!(by_type.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_artifact_content_inline_stored_correctly() {
        let artifact = Artifact::new_inline(
            "Test",
            ArtifactType::Prd,
            "inline content here",
            "user",
        );
        assert!(artifact.content.is_inline());
        if let ArtifactContent::Inline { text } = &artifact.content {
            assert_eq!(text, "inline content here");
        }
    }

    #[tokio::test]
    async fn test_artifact_content_file_stored_correctly() {
        let artifact = Artifact::new_file(
            "Test",
            ArtifactType::Prd,
            "/path/to/file.md",
            "user",
        );
        assert!(artifact.content.is_file());
        if let ArtifactContent::File { path } = &artifact.content {
            assert_eq!(path, "/path/to/file.md");
        }
    }

    #[tokio::test]
    async fn test_artifact_with_bucket_association() {
        let bucket_id = ArtifactBucketId::from_string("prd-library");
        let artifact = Artifact::new_inline("Test", ArtifactType::Prd, "content", "user")
            .with_bucket(bucket_id.clone());

        assert_eq!(artifact.bucket_id, Some(bucket_id));
    }

    #[tokio::test]
    async fn test_artifact_with_task_association() {
        let task_id = TaskId::from_string("task-123".to_string());
        let artifact = Artifact::new_inline("Test", ArtifactType::CodeChange, "diff", "worker")
            .with_task(task_id.clone());

        assert_eq!(artifact.metadata.task_id, Some(task_id));
    }

    #[tokio::test]
    async fn test_artifact_with_process_association() {
        let process_id = ProcessId::from_string("process-123");
        let artifact = Artifact::new_inline("Test", ArtifactType::Findings, "findings", "researcher")
            .with_process(process_id.clone());

        assert_eq!(artifact.metadata.process_id, Some(process_id));
    }

    #[tokio::test]
    async fn test_artifact_derived_from_chain() {
        let parent1 = ArtifactId::from_string("parent-1");
        let parent2 = ArtifactId::from_string("parent-2");

        let artifact = Artifact::new_inline("Test", ArtifactType::Recommendations, "recs", "agent")
            .derived_from_artifact(parent1.clone())
            .derived_from_artifact(parent2.clone());

        assert_eq!(artifact.derived_from.len(), 2);
        assert!(artifact.derived_from.contains(&parent1));
        assert!(artifact.derived_from.contains(&parent2));
    }
}
