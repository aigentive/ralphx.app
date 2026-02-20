use super::*;
use crate::domain::entities::{ArtifactBucketId, ArtifactFlowStep, ArtifactFlowTrigger};

fn create_test_flow() -> ArtifactFlow {
    ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created()).with_step(
        ArtifactFlowStep::copy(ArtifactBucketId::from_string("test-bucket")),
    )
}

#[tokio::test]
async fn test_create_and_get_flow() {
    let repo = MemoryArtifactFlowRepository::new();
    let flow = create_test_flow();

    repo.create(flow.clone()).await.unwrap();
    let found = repo.get_by_id(&flow.id).await.unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, flow.id);
}

#[tokio::test]
async fn test_get_all_flows() {
    let repo = MemoryArtifactFlowRepository::new();
    let flow1 = create_test_flow();
    let flow2 = ArtifactFlow::new("Another Flow", ArtifactFlowTrigger::on_task_completed());

    repo.create(flow1).await.unwrap();
    repo.create(flow2).await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_active_flows() {
    let repo = MemoryArtifactFlowRepository::new();
    let active = create_test_flow();
    let inactive = ArtifactFlow::new("Inactive", ArtifactFlowTrigger::on_artifact_created())
        .set_active(false);

    repo.create(active).await.unwrap();
    repo.create(inactive).await.unwrap();

    let active_flows = repo.get_active().await.unwrap();
    assert_eq!(active_flows.len(), 1);
}

#[tokio::test]
async fn test_set_active() {
    let repo = MemoryArtifactFlowRepository::new();
    let flow = create_test_flow();

    repo.create(flow.clone()).await.unwrap();
    repo.set_active(&flow.id, false).await.unwrap();

    let found = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert!(!found.is_active);
}

#[tokio::test]
async fn test_delete_flow() {
    let repo = MemoryArtifactFlowRepository::new();
    let flow = create_test_flow();

    repo.create(flow.clone()).await.unwrap();
    repo.delete(&flow.id).await.unwrap();
    let found = repo.get_by_id(&flow.id).await.unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn test_exists() {
    let repo = MemoryArtifactFlowRepository::new();
    let flow = create_test_flow();

    assert!(!repo.exists(&flow.id).await.unwrap());
    repo.create(flow.clone()).await.unwrap();
    assert!(repo.exists(&flow.id).await.unwrap());
}
