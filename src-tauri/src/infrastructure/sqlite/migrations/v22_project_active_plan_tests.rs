use super::v22_project_active_plan;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v22_creates_project_active_plan_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_active_plan'",
            [],
            |row| row.get(0),
        )
        .map(|count: i32| count > 0)
        .unwrap();

    assert!(table_exists, "project_active_plan table should exist");
}

#[test]
fn test_v22_creates_session_index() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify index exists
    let index_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_project_active_plan_session'",
            [],
            |row| row.get(0),
        )
        .map(|count: i32| count > 0)
        .unwrap();

    assert!(
        index_exists,
        "idx_project_active_plan_session index should exist"
    );
}

#[test]
fn test_v22_table_schema() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify columns exist
    let columns: Vec<(String, String)> = conn
        .prepare("PRAGMA table_info(project_active_plan)")
        .unwrap()
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let column_names: Vec<String> = columns.iter().map(|(name, _)| name.clone()).collect();

    assert!(column_names.contains(&"project_id".to_string()));
    assert!(column_names.contains(&"ideation_session_id".to_string()));
    assert!(column_names.contains(&"updated_at".to_string()));
}

#[test]
fn test_v22_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migration twice
    v22_project_active_plan::migrate(&conn).unwrap();
    v22_project_active_plan::migrate(&conn).unwrap();

    // Verify table still exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='project_active_plan'",
            [],
            |row| row.get(0),
        )
        .map(|count: i32| count > 0)
        .unwrap();

    assert!(table_exists);
}
