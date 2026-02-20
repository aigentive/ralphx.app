use super::*;
use crate::domain::entities::ArtifactType;

fn create_test_bucket() -> ArtifactBucket {
    ArtifactBucket::new("Test Bucket")
        .accepts(ArtifactType::Prd)
        .with_writer("user")
}

fn create_system_bucket() -> ArtifactBucket {
    ArtifactBucket::system("research-outputs", "Research Outputs")
        .accepts(ArtifactType::ResearchDocument)
}

#[tokio::test]
async fn test_create_and_get_bucket() {
    let repo = MemoryArtifactBucketRepository::new();
    let bucket = create_test_bucket();

    repo.create(bucket.clone()).await.unwrap();
    let found = repo.get_by_id(&bucket.id).await.unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, bucket.id);
}

#[tokio::test]
async fn test_get_all_buckets() {
    let repo = MemoryArtifactBucketRepository::new();
    let bucket1 = create_test_bucket();
    let bucket2 = create_system_bucket();

    repo.create(bucket1).await.unwrap();
    repo.create(bucket2).await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_system_buckets() {
    let repo = MemoryArtifactBucketRepository::new();
    let custom = create_test_bucket();
    let system = create_system_bucket();

    repo.create(custom).await.unwrap();
    repo.create(system).await.unwrap();

    let system_buckets = repo.get_system_buckets().await.unwrap();
    assert_eq!(system_buckets.len(), 1);
    assert!(system_buckets[0].is_system);
}

#[tokio::test]
async fn test_delete_custom_bucket() {
    let repo = MemoryArtifactBucketRepository::new();
    let bucket = create_test_bucket();

    repo.create(bucket.clone()).await.unwrap();
    repo.delete(&bucket.id).await.unwrap();
    let found = repo.get_by_id(&bucket.id).await.unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn test_delete_system_bucket_fails() {
    let repo = MemoryArtifactBucketRepository::new();
    let bucket = create_system_bucket();

    repo.create(bucket.clone()).await.unwrap();
    let result = repo.delete(&bucket.id).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_exists() {
    let repo = MemoryArtifactBucketRepository::new();
    let bucket = create_test_bucket();

    assert!(!repo.exists(&bucket.id).await.unwrap());
    repo.create(bucket.clone()).await.unwrap();
    assert!(repo.exists(&bucket.id).await.unwrap());
}
