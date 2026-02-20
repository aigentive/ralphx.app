use super::*;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[tokio::test]
async fn test_get_default_app_state() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteAppStateRepository::new(conn);

    let settings = repo.get().await.unwrap();
    assert!(settings.active_project_id.is_none());
}

#[tokio::test]
async fn test_set_and_get_active_project() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteAppStateRepository::new(conn);

    let project_id = ProjectId::from_string("proj-123".to_string());
    repo.set_active_project(Some(&project_id)).await.unwrap();

    let settings = repo.get().await.unwrap();
    assert_eq!(
        settings.active_project_id,
        Some(ProjectId::from_string("proj-123".to_string()))
    );
}

#[tokio::test]
async fn test_clear_active_project() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let repo = SqliteAppStateRepository::new(conn);

    // Set a project
    let project_id = ProjectId::from_string("proj-123".to_string());
    repo.set_active_project(Some(&project_id)).await.unwrap();

    // Clear it
    repo.set_active_project(None).await.unwrap();

    let settings = repo.get().await.unwrap();
    assert!(settings.active_project_id.is_none());
}

#[tokio::test]
async fn test_shared_connection() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    let shared_conn = Arc::new(Mutex::new(conn));

    let repo = SqliteAppStateRepository::from_shared(Arc::clone(&shared_conn));

    let settings = repo.get().await.unwrap();
    assert!(settings.active_project_id.is_none());
}
