use super::*;
use crate::infrastructure::sqlite::open_memory_connection;
use crate::infrastructure::sqlite::run_migrations;

fn setup_test_db() -> (
    SqlitePlanSelectionStatsRepository,
    ProjectId,
    IdeationSessionId,
) {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project
    let project_id = ProjectId::new();
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, merge_validation_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            project_id.as_str(),
            "Test Project",
            "/test/path",
            "local",
            "block",
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339(),
        ],
    )
    .unwrap();

    // Create ideation session
    let session_id = IdeationSessionId::new();
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            session_id.as_str(),
            project_id.as_str(),
            "Test Session",
            "accepted",
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339(),
        ],
    )
    .unwrap();

    (
        SqlitePlanSelectionStatsRepository::new(conn),
        project_id,
        session_id,
    )
}

#[tokio::test]
async fn test_record_selection_creates_new_entry() {
    let (repo, project_id, session_id) = setup_test_db();
    let timestamp = Utc::now();

    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::KanbanInline,
        timestamp,
    )
    .await
    .unwrap();

    let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert_eq!(stats.selected_count, 1);
    assert_eq!(
        stats.last_selected_source,
        Some("kanban_inline".to_string())
    );
}

#[tokio::test]
async fn test_record_selection_increments_count() {
    let (repo, project_id, session_id) = setup_test_db();
    let timestamp1 = Utc::now();

    // First selection
    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::KanbanInline,
        timestamp1,
    )
    .await
    .unwrap();

    // Second selection
    let timestamp2 = Utc::now();
    repo.record_selection(
        &project_id,
        &session_id,
        SelectionSource::QuickSwitcher,
        timestamp2,
    )
    .await
    .unwrap();

    let stats = repo
        .get_stats(&project_id, &session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stats.selected_count, 2);
    assert_eq!(
        stats.last_selected_source,
        Some("quick_switcher".to_string())
    );
}

#[tokio::test]
async fn test_get_stats_batch() {
    let (repo, project_id, session1) = setup_test_db();

    // Create second session
    let session2 = IdeationSessionId::new();
    repo.conn
        .lock()
        .await
        .execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session2.as_str(),
                project_id.as_str(),
                "Test Session 2",
                "accepted",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

    let session3 = IdeationSessionId::new(); // Not in DB
    let timestamp = Utc::now();

    // Record stats for session1 and session2
    repo.record_selection(
        &project_id,
        &session1,
        SelectionSource::KanbanInline,
        timestamp,
    )
    .await
    .unwrap();
    repo.record_selection(
        &project_id,
        &session2,
        SelectionSource::GraphInline,
        timestamp,
    )
    .await
    .unwrap();

    // Query batch
    let results = repo
        .get_stats_batch(
            &project_id,
            &[session1.clone(), session2.clone(), session3.clone()],
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_some());
    assert_eq!(results[0].as_ref().unwrap().ideation_session_id, session1);
    assert!(results[1].is_some());
    assert_eq!(results[1].as_ref().unwrap().ideation_session_id, session2);
    assert!(results[2].is_none()); // session3 not in DB
}

#[tokio::test]
async fn test_get_stats_nonexistent() {
    let (repo, project_id, session_id) = setup_test_db();

    let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
    assert!(stats.is_none());
}
