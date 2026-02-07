// V14 migration tests - app_state singleton table

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v14_creates_app_state_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "app_state"));
}

#[test]
fn test_v14_app_state_has_correct_columns() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "app_state", "id"));
    assert!(helpers::column_exists(
        &conn,
        "app_state",
        "active_project_id"
    ));
    assert!(helpers::column_exists(&conn, "app_state", "updated_at"));
}

#[test]
fn test_v14_singleton_row_inserted() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let (id, active_project_id): (i64, Option<String>) = conn
        .query_row("SELECT id, active_project_id FROM app_state WHERE id = 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();

    assert_eq!(id, 1);
    assert!(active_project_id.is_none());
}

#[test]
fn test_v14_check_constraint_prevents_id_not_1() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let result = conn.execute(
        "INSERT INTO app_state (id, active_project_id, updated_at)
         VALUES (2, NULL, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn test_v14_active_project_id_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Update with a project id
    conn.execute(
        "UPDATE app_state SET active_project_id = 'proj-123', updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = 1",
        [],
    )
    .unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT active_project_id FROM app_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, Some("proj-123".to_string()));

    // Set back to NULL
    conn.execute(
        "UPDATE app_state SET active_project_id = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = 1",
        [],
    )
    .unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT active_project_id FROM app_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(result.is_none());
}

#[test]
fn test_v14_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "app_state"));

    // Singleton row should still be there (INSERT OR IGNORE)
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_state", [], |row| row.get(0))
        .unwrap();

    assert_eq!(count, 1);
}
