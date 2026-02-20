use super::*;
use crate::domain::entities::status::InternalStatus;
use crate::domain::entities::workflow::{WorkflowColumn, WorkflowSchema};

fn create_test_workflow() -> WorkflowSchema {
    WorkflowSchema::new(
        "Test Workflow",
        vec![
            WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
            WorkflowColumn::new("done", "Done", InternalStatus::Approved),
        ],
    )
}

fn create_test_methodology() -> MethodologyExtension {
    let workflow = create_test_workflow();
    MethodologyExtension::new("Test Method", workflow).with_description("A test methodology")
}

fn create_active_methodology() -> MethodologyExtension {
    let workflow = create_test_workflow();
    let mut methodology = MethodologyExtension::new("Active Method", workflow);
    methodology.activate();
    methodology
}

#[tokio::test]
async fn test_create_and_get_methodology() {
    let repo = MemoryMethodologyRepository::new();
    let methodology = create_test_methodology();

    repo.create(methodology.clone()).await.unwrap();
    let found = repo.get_by_id(&methodology.id).await.unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, methodology.id);
}

#[tokio::test]
async fn test_get_all_methodologies() {
    let repo = MemoryMethodologyRepository::new();
    let methodology1 = create_test_methodology();
    let methodology2 = create_active_methodology();

    repo.create(methodology1).await.unwrap();
    repo.create(methodology2).await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_active_none() {
    let repo = MemoryMethodologyRepository::new();
    let methodology = create_test_methodology();

    repo.create(methodology).await.unwrap();

    let active = repo.get_active().await.unwrap();
    assert!(active.is_none());
}

#[tokio::test]
async fn test_get_active_some() {
    let repo = MemoryMethodologyRepository::new();
    let inactive = create_test_methodology();
    let active = create_active_methodology();

    repo.create(inactive).await.unwrap();
    repo.create(active.clone()).await.unwrap();

    let found = repo.get_active().await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, active.id);
}

#[tokio::test]
async fn test_activate_deactivates_previous() {
    let repo = MemoryMethodologyRepository::new();
    let methodology1 = create_active_methodology();
    let methodology2 = create_test_methodology();

    repo.create(methodology1.clone()).await.unwrap();
    repo.create(methodology2.clone()).await.unwrap();

    // Activate methodology2
    repo.activate(&methodology2.id).await.unwrap();

    // methodology1 should no longer be active
    let found1 = repo.get_by_id(&methodology1.id).await.unwrap().unwrap();
    assert!(!found1.is_active);

    // methodology2 should be active
    let found2 = repo.get_by_id(&methodology2.id).await.unwrap().unwrap();
    assert!(found2.is_active);
}

#[tokio::test]
async fn test_deactivate() {
    let repo = MemoryMethodologyRepository::new();
    let methodology = create_active_methodology();

    repo.create(methodology.clone()).await.unwrap();
    repo.deactivate(&methodology.id).await.unwrap();

    let found = repo.get_by_id(&methodology.id).await.unwrap().unwrap();
    assert!(!found.is_active);
}

#[tokio::test]
async fn test_delete_methodology() {
    let repo = MemoryMethodologyRepository::new();
    let methodology = create_test_methodology();

    repo.create(methodology.clone()).await.unwrap();
    repo.delete(&methodology.id).await.unwrap();
    let found = repo.get_by_id(&methodology.id).await.unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn test_exists() {
    let repo = MemoryMethodologyRepository::new();
    let methodology = create_test_methodology();

    assert!(!repo.exists(&methodology.id).await.unwrap());
    repo.create(methodology.clone()).await.unwrap();
    assert!(repo.exists(&methodology.id).await.unwrap());
}
