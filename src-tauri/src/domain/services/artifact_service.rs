// ArtifactService - domain service for artifact management
//
// Provides business logic for:
// - Creating artifacts with bucket validation
// - Retrieving artifacts for tasks and processes
// - Copying artifacts between buckets
// - Versioning artifacts
// - Managing artifact content (inline vs file)

use std::sync::Arc;

use crate::domain::entities::{
    Artifact, ArtifactBucket, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata,
    ArtifactRelation, ArtifactRelationType, ArtifactType, ProcessId, TaskId,
};
use crate::domain::repositories::{ArtifactBucketRepository, ArtifactRepository};
use crate::error::{AppError, AppResult};

/// Service for artifact-related business logic
pub struct ArtifactService<A: ArtifactRepository, B: ArtifactBucketRepository> {
    artifact_repo: Arc<A>,
    bucket_repo: Arc<B>,
}

impl<A: ArtifactRepository, B: ArtifactBucketRepository> ArtifactService<A, B> {
    /// Create a new ArtifactService with the given repositories
    pub fn new(artifact_repo: Arc<A>, bucket_repo: Arc<B>) -> Self {
        Self {
            artifact_repo,
            bucket_repo,
        }
    }

    /// Create a new artifact, optionally in a specific bucket.
    /// Validates bucket constraints (accepted types, writers) if bucket is specified.
    pub async fn create_artifact(
        &self,
        artifact: Artifact,
        creator: &str,
    ) -> AppResult<Artifact> {
        // Validate bucket constraints if bucket is specified
        if let Some(bucket_id) = &artifact.bucket_id {
            let bucket = self
                .bucket_repo
                .get_by_id(bucket_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Bucket not found: {}", bucket_id)))?;

            // Check if the artifact type is accepted by the bucket
            if !bucket.accepts_type(artifact.artifact_type) {
                return Err(AppError::Validation(format!(
                    "Bucket '{}' does not accept artifact type '{}'",
                    bucket.name, artifact.artifact_type
                )));
            }

            // Check if the creator can write to the bucket
            if !bucket.can_write(creator) {
                return Err(AppError::Validation(format!(
                    "Creator '{}' cannot write to bucket '{}'",
                    creator, bucket.name
                )));
            }
        }

        // Create the artifact
        self.artifact_repo.create(artifact).await
    }

    /// Get an artifact by ID
    pub async fn get_artifact(&self, id: &ArtifactId) -> AppResult<Option<Artifact>> {
        self.artifact_repo.get_by_id(id).await
    }

    /// Get all artifacts for a task
    pub async fn get_artifacts_for_task(&self, task_id: &TaskId) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_by_task(task_id).await
    }

    /// Get all artifacts for a process
    pub async fn get_artifacts_for_process(
        &self,
        process_id: &ProcessId,
    ) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_by_process(process_id).await
    }

    /// Get all artifacts in a bucket
    pub async fn get_artifacts_in_bucket(
        &self,
        bucket_id: &ArtifactBucketId,
    ) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_by_bucket(bucket_id).await
    }

    /// Get all artifacts of a specific type
    pub async fn get_artifacts_by_type(
        &self,
        artifact_type: ArtifactType,
    ) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_by_type(artifact_type).await
    }

    /// Copy an artifact to a different bucket, creating a new artifact.
    /// Returns the new artifact with the target bucket ID.
    pub async fn copy_to_bucket(
        &self,
        artifact_id: &ArtifactId,
        target_bucket_id: &ArtifactBucketId,
        copier: &str,
    ) -> AppResult<Artifact> {
        // Get the source artifact
        let source = self
            .artifact_repo
            .get_by_id(artifact_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Artifact not found: {}", artifact_id)))?;

        // Get the target bucket
        let bucket = self
            .bucket_repo
            .get_by_id(target_bucket_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Bucket not found: {}", target_bucket_id))
            })?;

        // Validate the artifact type is accepted
        if !bucket.accepts_type(source.artifact_type) {
            return Err(AppError::Validation(format!(
                "Bucket '{}' does not accept artifact type '{}'",
                bucket.name, source.artifact_type
            )));
        }

        // Validate the copier can write to the bucket
        if !bucket.can_write(copier) {
            return Err(AppError::Validation(format!(
                "Copier '{}' cannot write to bucket '{}'",
                copier, bucket.name
            )));
        }

        // Create a new artifact as a copy
        let new_artifact = Artifact {
            id: ArtifactId::new(),
            artifact_type: source.artifact_type,
            name: source.name.clone(),
            content: source.content.clone(),
            metadata: ArtifactMetadata::new(copier)
                .with_version(1),
            derived_from: vec![source.id.clone()],
            bucket_id: Some(target_bucket_id.clone()),
        };

        // Add task/process associations if present in source
        let mut new_artifact = new_artifact;
        if let Some(task_id) = &source.metadata.task_id {
            new_artifact.metadata.task_id = Some(task_id.clone());
        }
        if let Some(process_id) = &source.metadata.process_id {
            new_artifact.metadata.process_id = Some(process_id.clone());
        }

        // Create the artifact
        let created = self.artifact_repo.create(new_artifact).await?;

        // Add a derived_from relation
        let relation = ArtifactRelation::derived_from(created.id.clone(), source.id.clone());
        self.artifact_repo.add_relation(relation).await?;

        Ok(created)
    }

    /// Create a new version of an artifact.
    /// Returns the new version with incremented version number.
    pub async fn version_artifact(
        &self,
        artifact_id: &ArtifactId,
        new_content: ArtifactContent,
        updater: &str,
    ) -> AppResult<Artifact> {
        // Get the current artifact
        let current = self
            .artifact_repo
            .get_by_id(artifact_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Artifact not found: {}", artifact_id)))?;

        // Create a new artifact as the next version
        let new_version = current.metadata.version + 1;
        let new_artifact = Artifact {
            id: ArtifactId::new(),
            artifact_type: current.artifact_type,
            name: current.name.clone(),
            content: new_content,
            metadata: ArtifactMetadata::new(updater)
                .with_version(new_version),
            derived_from: vec![current.id.clone()],
            bucket_id: current.bucket_id.clone(),
        };

        // Preserve task/process associations
        let mut new_artifact = new_artifact;
        if let Some(task_id) = &current.metadata.task_id {
            new_artifact.metadata.task_id = Some(task_id.clone());
        }
        if let Some(process_id) = &current.metadata.process_id {
            new_artifact.metadata.process_id = Some(process_id.clone());
        }

        // Create the new version
        let created = self.artifact_repo.create(new_artifact).await?;

        // Add a derived_from relation
        let relation = ArtifactRelation::derived_from(created.id.clone(), current.id.clone());
        self.artifact_repo.add_relation(relation).await?;

        Ok(created)
    }

    /// Get all buckets
    pub async fn get_buckets(&self) -> AppResult<Vec<ArtifactBucket>> {
        self.bucket_repo.get_all().await
    }

    /// Get a bucket by ID
    pub async fn get_bucket(&self, id: &ArtifactBucketId) -> AppResult<Option<ArtifactBucket>> {
        self.bucket_repo.get_by_id(id).await
    }

    /// Get artifacts that a given artifact was derived from
    pub async fn get_derived_from(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_derived_from(artifact_id).await
    }

    /// Get all related artifacts
    pub async fn get_related(&self, artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
        self.artifact_repo.get_related(artifact_id).await
    }

    /// Add a relation between two artifacts
    pub async fn add_relation(
        &self,
        from_id: ArtifactId,
        to_id: ArtifactId,
        relation_type: ArtifactRelationType,
    ) -> AppResult<ArtifactRelation> {
        // Validate both artifacts exist
        self.artifact_repo
            .get_by_id(&from_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Artifact not found: {}", from_id)))?;
        self.artifact_repo
            .get_by_id(&to_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Artifact not found: {}", to_id)))?;

        let relation = ArtifactRelation::new(from_id, to_id, relation_type);
        self.artifact_repo.add_relation(relation).await
    }
}

#[cfg(test)]
#[path = "artifact_service_tests.rs"]
mod tests;
