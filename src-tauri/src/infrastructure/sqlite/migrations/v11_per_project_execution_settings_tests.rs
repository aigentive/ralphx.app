// V11 migration tests - per-project execution settings and global cap

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v11_adds_project_id_column() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify project_id column exists in execution_settings
    assert!(helpers::column_exists(&conn, "execution_settings", "project_id"));
}

#[test]
fn test_v11_creates_global_execution_settings_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify table exists
    assert!(helpers::table_exists(&conn, "global_execution_settings"));

    // Verify default row exists with correct values
    let result: (i64, i64) = conn
        .query_row(
            "SELECT id, global_max_concurrent FROM global_execution_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, 20); // Default global max concurrent
}

#[test]
fn test_v11_existing_settings_preserved() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify the existing global execution_settings row is preserved
    let result: (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT id, max_concurrent_tasks, auto_commit, pause_on_failure
             FROM execution_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(result.0, 1);
    assert_eq!(result.1, 2); // Default max concurrent
    assert_eq!(result.2, 1); // auto_commit true
    assert_eq!(result.3, 1); // pause_on_failure true
}

#[test]
fn test_v11_can_insert_project_specific_settings() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project-specific settings row
    conn.execute(
        "INSERT INTO execution_settings (max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
         VALUES (4, 0, 1, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), 'project-123')",
        [],
    )
    .unwrap();

    // Verify project-specific row
    let result: (i64, String) = conn
        .query_row(
            "SELECT max_concurrent_tasks, project_id FROM execution_settings WHERE project_id = 'project-123'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(result.0, 4);
    assert_eq!(result.1, "project-123");
}

#[test]
fn test_v11_project_id_unique_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert first project settings
    conn.execute(
        "INSERT INTO execution_settings (max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
         VALUES (4, 0, 1, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), 'project-123')",
        [],
    )
    .unwrap();

    // Attempt duplicate should fail
    let result = conn.execute(
        "INSERT INTO execution_settings (max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
         VALUES (8, 1, 0, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), 'project-123')",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn test_v11_can_update_global_max_concurrent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Update global max concurrent
    conn.execute(
        "UPDATE global_execution_settings SET global_max_concurrent = 50 WHERE id = 1",
        [],
    )
    .unwrap();

    // Verify update
    let result: i64 = conn
        .query_row(
            "SELECT global_max_concurrent FROM global_execution_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, 50);
}

#[test]
fn test_v11_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    // (CREATE TABLE IF NOT EXISTS, INSERT OR IGNORE patterns)
    run_migrations(&conn).unwrap();

    // Verify state is still correct
    assert!(helpers::column_exists(&conn, "execution_settings", "project_id"));
    assert!(helpers::table_exists(&conn, "global_execution_settings"));
}
