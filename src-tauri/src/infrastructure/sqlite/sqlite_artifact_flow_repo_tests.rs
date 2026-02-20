use super::*;
use crate::domain::entities::{
    ArtifactBucketId, ArtifactFlowFilter, ArtifactFlowStep, ArtifactFlowTrigger, ArtifactType,
};
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_test_db() -> Connection {
    let conn = open_memory_connection().expect("Failed to open memory connection");
    run_migrations(&conn).expect("Failed to run migrations");
    conn
}

fn create_test_flow() -> ArtifactFlow {
    ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created()).with_step(
        ArtifactFlowStep::copy(ArtifactBucketId::from_string("test-bucket")),
    )
}

fn create_flow_with_filter() -> ArtifactFlow {
    ArtifactFlow::new(
        "Filtered Flow",
        ArtifactFlowTrigger::on_artifact_created().with_filter(
            ArtifactFlowFilter::new()
                .with_artifact_types(vec![ArtifactType::Recommendations])
                .with_source_bucket(ArtifactBucketId::from_string("research-outputs")),
        ),
    )
    .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
        "prd-library",
    )))
    .with_step(ArtifactFlowStep::spawn_process(
        "task_decomposition",
        "orchestrator",
    ))
}

#[tokio::test]
async fn test_create_artifact_flow() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);
    let flow = create_test_flow();

    let result = repo.create(flow.clone()).await;
    assert!(result.is_ok());

    let created = result.unwrap();
    assert_eq!(created.id, flow.id);
    assert_eq!(created.name, "Test Flow");
}

#[tokio::test]
async fn test_get_by_id_found() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);
    let flow = create_test_flow();

    repo.create(flow.clone()).await.unwrap();

    let result = repo.get_by_id(&flow.id).await;
    assert!(result.is_ok());

    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Test Flow");
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);
    let id = ArtifactFlowId::new();

    let result = repo.get_by_id(&id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_all_empty() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let result = repo.get_all().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_all_with_flows() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow1 = create_test_flow();
    let flow2 = create_flow_with_filter();

    repo.create(flow1).await.unwrap();
    repo.create(flow2).await.unwrap();

    let result = repo.get_all().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_all_returns_sorted_by_name() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let mut flow1 = create_test_flow();
    flow1.name = "Zebra Flow".to_string();

    let mut flow2 = create_test_flow();
    flow2.id = ArtifactFlowId::new();
    flow2.name = "Alpha Flow".to_string();

    repo.create(flow1).await.unwrap();
    repo.create(flow2).await.unwrap();

    let result = repo.get_all().await.unwrap();
    assert_eq!(result[0].name, "Alpha Flow");
    assert_eq!(result[1].name, "Zebra Flow");
}

#[tokio::test]
async fn test_get_active_filters_inactive() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let active_flow = create_test_flow();
    let inactive_flow = create_flow_with_filter().set_active(false);

    repo.create(active_flow.clone()).await.unwrap();
    repo.create(inactive_flow).await.unwrap();

    let result = repo.get_active().await;
    assert!(result.is_ok());

    let active = result.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, active_flow.id);
}

#[tokio::test]
async fn test_get_active_returns_all_active() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow1 = create_test_flow();
    let flow2 = create_flow_with_filter();

    repo.create(flow1).await.unwrap();
    repo.create(flow2).await.unwrap();

    let result = repo.get_active().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn test_update_flow() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let mut flow = create_test_flow();
    repo.create(flow.clone()).await.unwrap();

    flow.name = "Updated Name".to_string();
    flow.is_active = false;

    let result = repo.update(&flow).await;
    assert!(result.is_ok());

    let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert!(!updated.is_active);
}

#[tokio::test]
async fn test_delete_flow() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_test_flow();
    repo.create(flow.clone()).await.unwrap();

    let result = repo.delete(&flow.id).await;
    assert!(result.is_ok());

    let found = repo.get_by_id(&flow.id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn test_set_active_true() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_test_flow().set_active(false);
    repo.create(flow.clone()).await.unwrap();

    repo.set_active(&flow.id, true).await.unwrap();

    let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert!(updated.is_active);
}

#[tokio::test]
async fn test_set_active_false() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_test_flow(); // default is active
    repo.create(flow.clone()).await.unwrap();

    repo.set_active(&flow.id, false).await.unwrap();

    let updated = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert!(!updated.is_active);
}

#[tokio::test]
async fn test_exists_true() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_test_flow();
    repo.create(flow.clone()).await.unwrap();

    let result = repo.exists(&flow.id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_exists_false() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let id = ArtifactFlowId::new();

    let result = repo.exists(&id).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_trigger_filter_preserved() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_flow_with_filter();
    repo.create(flow.clone()).await.unwrap();

    let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

    // Verify trigger and filter are preserved
    assert!(loaded.trigger.filter.is_some());
    let filter = loaded.trigger.filter.as_ref().unwrap();
    assert!(filter.artifact_types.is_some());
    assert_eq!(
        filter.artifact_types.as_ref().unwrap()[0],
        ArtifactType::Recommendations
    );
    assert!(filter.source_bucket.is_some());
    assert_eq!(
        filter.source_bucket.as_ref().unwrap().as_str(),
        "research-outputs"
    );
}

#[tokio::test]
async fn test_multiple_steps_preserved() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_flow_with_filter();
    repo.create(flow.clone()).await.unwrap();

    let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

    assert_eq!(loaded.steps.len(), 2);
    assert!(loaded.steps[0].is_copy());
    assert!(loaded.steps[1].is_spawn_process());

    // Verify step details
    if let crate::domain::entities::ArtifactFlowStep::Copy { to_bucket } = &loaded.steps[0] {
        assert_eq!(to_bucket.as_str(), "prd-library");
    } else {
        panic!("Expected copy step");
    }

    if let crate::domain::entities::ArtifactFlowStep::SpawnProcess {
        process_type,
        agent_profile,
    } = &loaded.steps[1]
    {
        assert_eq!(process_type, "task_decomposition");
        assert_eq!(agent_profile, "orchestrator");
    } else {
        panic!("Expected spawn_process step");
    }
}

#[tokio::test]
async fn test_created_at_preserved() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let flow = create_test_flow();
    let original_created_at = flow.created_at;
    repo.create(flow.clone()).await.unwrap();

    let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();

    // Timestamps should match (allowing for microsecond precision differences)
    let diff = (loaded.created_at - original_created_at)
        .num_milliseconds()
        .abs();
    assert!(diff < 1000, "Timestamps differ by {}ms", diff);
}

#[tokio::test]
async fn test_from_shared_connection() {
    let conn = setup_test_db();
    let shared = Arc::new(Mutex::new(conn));

    let repo1 = SqliteArtifactFlowRepository::from_shared(shared.clone());
    let repo2 = SqliteArtifactFlowRepository::from_shared(shared.clone());

    // Create via repo1
    let flow = create_test_flow();
    repo1.create(flow.clone()).await.unwrap();

    // Read via repo2
    let found = repo2.get_by_id(&flow.id).await.unwrap();
    assert!(found.is_some());
}

#[tokio::test]
async fn test_update_steps() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let mut flow = create_test_flow();
    repo.create(flow.clone()).await.unwrap();

    // Add a new step
    flow.steps
        .push(ArtifactFlowStep::spawn_process("verification", "reviewer"));
    repo.update(&flow).await.unwrap();

    let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert_eq!(loaded.steps.len(), 2);
}

#[tokio::test]
async fn test_update_trigger() {
    let conn = setup_test_db();
    let repo = SqliteArtifactFlowRepository::new(conn);

    let mut flow = create_test_flow();
    repo.create(flow.clone()).await.unwrap();

    // Change trigger
    flow.trigger = ArtifactFlowTrigger::on_task_completed();
    repo.update(&flow).await.unwrap();

    let loaded = repo.get_by_id(&flow.id).await.unwrap().unwrap();
    assert_eq!(
        loaded.trigger.event,
        crate::domain::entities::ArtifactFlowEvent::TaskCompleted
    );
}
