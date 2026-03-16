//! Tests for migration v69: archived_at columns on task_proposals, artifacts, and projects

use rusqlite::Connection;

use super::helpers;
use super::v69_soft_delete_archived_at;

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    conn.execute_batch(
        "CREATE TABLE task_proposals (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE artifacts (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );",
    )
    .expect("Failed to create test schema");

    conn
}

// ---------------------------------------------------------------------------
// Column existence tests
// ---------------------------------------------------------------------------

#[test]
fn test_archived_at_columns_added() {
    let conn = setup_test_db();

    for table in &["task_proposals", "artifacts", "projects"] {
        assert!(
            !helpers::column_exists(&conn, table, "archived_at"),
            "archived_at should not exist on {table} before migration"
        );
    }

    v69_soft_delete_archived_at::migrate(&conn).unwrap();

    for table in &["task_proposals", "artifacts", "projects"] {
        assert!(
            helpers::column_exists(&conn, table, "archived_at"),
            "archived_at should exist on {table} after migration"
        );
    }
}

// ---------------------------------------------------------------------------
// Default value (NULL)
// ---------------------------------------------------------------------------

#[test]
fn test_archived_at_defaults_to_null() {
    let conn = setup_test_db();
    v69_soft_delete_archived_at::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO task_proposals (id, title) VALUES ('p1', 'Test Proposal')",
        [],
    )
    .unwrap();

    let val: Option<String> = conn
        .query_row(
            "SELECT archived_at FROM task_proposals WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(val.is_none(), "archived_at should default to NULL");
}

// ---------------------------------------------------------------------------
// Idempotency
// ---------------------------------------------------------------------------

#[test]
fn test_migration_idempotent() {
    let conn = setup_test_db();

    v69_soft_delete_archived_at::migrate(&conn).unwrap();
    v69_soft_delete_archived_at::migrate(&conn).unwrap();

    for table in &["task_proposals", "artifacts", "projects"] {
        assert!(helpers::column_exists(&conn, table, "archived_at"));
    }
}
