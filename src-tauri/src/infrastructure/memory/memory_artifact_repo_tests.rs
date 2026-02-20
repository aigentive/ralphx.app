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

    let relation = ArtifactRelation::derived_from(artifact2.id.clone(), artifact1.id.clone());
    repo.add_relation(relation).await.unwrap();

    let relations = repo.get_relations(&artifact2.id).await.unwrap();
    assert_eq!(relations.len(), 1);
}
