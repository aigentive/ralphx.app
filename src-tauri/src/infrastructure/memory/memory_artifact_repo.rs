// Memory-based ArtifactRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactId, ArtifactRelation, ArtifactRelationType, ArtifactType,
    ProcessId, TaskId,
};
use crate::domain::repositories::ArtifactRepository;
use crate::error::AppResult;

/// In-memory implementation of ArtifactRepository for testing
pub struct MemoryArtifactRepository {
    artifacts: Arc<RwLock<HashMap<ArtifactId, Artifact>>>,
    relations: Arc<RwLock<HashMap<String, ArtifactRelation>>>,
}

impl Default for MemoryArtifactRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryArtifactRepository {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(RwLock::new(HashMap::new())),
            relations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_artifacts(artifacts: Vec<Artifact>) -> Self {
        let map: HashMap<ArtifactId, Artifact> =
            artifacts.into_iter().map(|a| (a.id.clone(), a)).collect();
        Self {
            artifacts: Arc::new(RwLock::new(map)),
            relations: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ArtifactRepository for MemoryArtifactRepository {
    async fn create(&self, artifact: Artifact) -> AppResult<Artifact> {
        let mut artifacts = self.artifacts.write().await;
        artifacts.insert(artifact.id.clone(), artifact.clone());
        Ok(artifact)
    }

    async fn get_by_id(&self, id: &ArtifactId) -> AppResult<Option<Artifact>> {
        let artifacts = self.artifacts.read().await;
        Ok(artifacts.get(id).cloned())
    }

    async fn get_by_id_at_version(&self, id: &ArtifactId, version: u32) -> AppResult<Option<Artifact>> {
        let artifacts = self.artifacts.read().await;
        if let Some(artifact) = artifacts.get(id) {
            // For memory implementation, just return the artifact if the version matches
            // In a real implementation, this would traverse version history
            if artifact.metadata.version == version {
                Ok(Some(artifact.clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn get_by_bucket(&self, bucket_id: &ArtifactBucketId) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        Ok(artifacts
            .values()
            .filter(|a| a.bucket_id.as_ref() == Some(bucket_id))
            .cloned()
            .collect())
    }

    async fn get_by_type(&self, artifact_type: ArtifactType) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        Ok(artifacts
            .values()
            .filter(|a| a.artifact_type == artifact_type)
            .cloned()
            .collect())
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        Ok(artifacts
            .values()
            .filter(|a| a.metadata.task_id.as_ref() == Some(task_id))
            .cloned()
            .collect())
    }

    async fn get_by_process(&self, process_id: &ProcessId) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        Ok(artifacts
            .values()
            .filter(|a| a.metadata.process_id.as_ref() == Some(process_id))
            .cloned()
            .collect())
    }

    async fn update(&self, artifact: &Artifact) -> AppResult<()> {
        let mut artifacts = self.artifacts.write().await;
        artifacts.insert(artifact.id.clone(), artifact.clone());
        Ok(())
    }

    async fn delete(&self, id: &ArtifactId) -> AppResult<()> {
        let mut artifacts = self.artifacts.write().await;
        artifacts.remove(id);
        Ok(())
    }

    async fn get_derived_from(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        let relations = self.relations.read().await;

        let parent_ids: Vec<ArtifactId> = relations
            .values()
            .filter(|r| {
                r.from_artifact_id == *artifact_id
                    && r.relation_type == ArtifactRelationType::DerivedFrom
            })
            .map(|r| r.to_artifact_id.clone())
            .collect();

        Ok(artifacts
            .values()
            .filter(|a| parent_ids.contains(&a.id))
            .cloned()
            .collect())
    }

    async fn get_related(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        let artifacts = self.artifacts.read().await;
        let relations = self.relations.read().await;

        let related_ids: Vec<ArtifactId> = relations
            .values()
            .filter(|r| r.from_artifact_id == *artifact_id || r.to_artifact_id == *artifact_id)
            .flat_map(|r| {
                if r.from_artifact_id == *artifact_id {
                    vec![r.to_artifact_id.clone()]
                } else {
                    vec![r.from_artifact_id.clone()]
                }
            })
            .collect();

        Ok(artifacts
            .values()
            .filter(|a| related_ids.contains(&a.id) && a.id != *artifact_id)
            .cloned()
            .collect())
    }

    async fn add_relation(&self, relation: ArtifactRelation) -> AppResult<ArtifactRelation> {
        let mut relations = self.relations.write().await;
        let key = format!("{}-{}", relation.from_artifact_id, relation.to_artifact_id);
        relations.insert(key, relation.clone());
        Ok(relation)
    }

    async fn get_relations(&self, artifact_id: &ArtifactId) -> AppResult<Vec<ArtifactRelation>> {
        let relations = self.relations.read().await;
        Ok(relations
            .values()
            .filter(|r| r.from_artifact_id == *artifact_id || r.to_artifact_id == *artifact_id)
            .cloned()
            .collect())
    }

    async fn get_relations_by_type(
        &self,
        artifact_id: &ArtifactId,
        relation_type: ArtifactRelationType,
    ) -> AppResult<Vec<ArtifactRelation>> {
        let relations = self.relations.read().await;
        Ok(relations
            .values()
            .filter(|r| {
                (r.from_artifact_id == *artifact_id || r.to_artifact_id == *artifact_id)
                    && r.relation_type == relation_type
            })
            .cloned()
            .collect())
    }

    async fn delete_relation(&self, from_id: &ArtifactId, to_id: &ArtifactId) -> AppResult<()> {
        let mut relations = self.relations.write().await;
        let key = format!("{}-{}", from_id, to_id);
        relations.remove(&key);
        Ok(())
    }

    async fn create_with_previous_version(
        &self,
        artifact: Artifact,
        _previous_version_id: ArtifactId,
    ) -> AppResult<Artifact> {
        // Memory implementation doesn't track previous_version_id, just create normally
        self.create(artifact).await
    }

    async fn get_version_history(
        &self,
        id: &ArtifactId,
    ) -> AppResult<Vec<crate::domain::repositories::ArtifactVersionSummary>> {
        // Memory implementation just returns single artifact summary if it exists
        let artifacts = self.artifacts.read().await;
        if let Some(artifact) = artifacts.get(id) {
            Ok(vec![crate::domain::repositories::ArtifactVersionSummary {
                id: artifact.id.clone(),
                version: artifact.metadata.version,
                name: artifact.name.clone(),
                created_at: artifact.metadata.created_at,
            }])
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_artifact() -> Artifact {
        Artifact::new_inline("Test PRD", ArtifactType::Prd, "Test content", "user")
    }

    #[tokio::test]
    async fn test_create_and_get_artifact() {
        let repo = MemoryArtifactRepository::new();
        let artifact = create_test_artifact();

        repo.create(artifact.clone()).await.unwrap();
        let found = repo.get_by_id(&artifact.id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, artifact.id);
    }

    #[tokio::test]
    async fn test_get_by_bucket() {
        let repo = MemoryArtifactRepository::new();
        let bucket_id = ArtifactBucketId::from_string("test-bucket");
        let artifact = create_test_artifact().with_bucket(bucket_id.clone());

        repo.create(artifact.clone()).await.unwrap();
        let found = repo.get_by_bucket(&bucket_id).await.unwrap();

        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn test_get_by_type() {
        let repo = MemoryArtifactRepository::new();
        let artifact = create_test_artifact();

        repo.create(artifact).await.unwrap();
        let found = repo.get_by_type(ArtifactType::Prd).await.unwrap();

        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_artifact() {
        let repo = MemoryArtifactRepository::new();
        let artifact = create_test_artifact();

        repo.create(artifact.clone()).await.unwrap();
        repo.delete(&artifact.id).await.unwrap();
        let found = repo.get_by_id(&artifact.id).await.unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_add_and_get_relations() {
        let repo = MemoryArtifactRepository::new();
        let artifact1 = create_test_artifact();
        let artifact2 = Artifact::new_inline("Child", ArtifactType::Findings, "Findings", "agent");

        repo.create(artifact1.clone()).await.unwrap();
        repo.create(artifact2.clone()).await.unwrap();

        let relation =
            ArtifactRelation::derived_from(artifact2.id.clone(), artifact1.id.clone());
        repo.add_relation(relation).await.unwrap();

        let relations = repo.get_relations(&artifact2.id).await.unwrap();
        assert_eq!(relations.len(), 1);
    }
}
