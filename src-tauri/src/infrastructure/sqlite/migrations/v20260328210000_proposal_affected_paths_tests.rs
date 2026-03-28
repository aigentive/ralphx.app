use rusqlite::Connection;

use super::helpers;
use super::v20260328210000_proposal_affected_paths;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE task_proposals (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );",
    )
    .expect("Failed to create test schema");
    conn
}

#[test]
fn test_affected_paths_column_added() {
    let conn = setup_test_db();
    assert!(
        !helpers::column_exists(&conn, "task_proposals", "affected_paths"),
        "affected_paths should not exist before migration"
    );

    v20260328210000_proposal_affected_paths::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "task_proposals", "affected_paths"),
        "affected_paths should exist after migration"
    );
}

#[test]
fn test_existing_proposals_default_to_null_affected_paths() {
    let conn = setup_test_db();
    conn.execute(
        "INSERT INTO task_proposals (id, title) VALUES ('p1', 'Pre-migration Proposal')",
        [],
    )
    .unwrap();

    v20260328210000_proposal_affected_paths::migrate(&conn).unwrap();

    let affected_paths: Option<String> = conn
        .query_row(
            "SELECT affected_paths FROM task_proposals WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(affected_paths.is_none());
}

#[test]
fn test_new_proposals_can_store_affected_paths() {
    let conn = setup_test_db();
    v20260328210000_proposal_affected_paths::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO task_proposals (id, title, affected_paths) VALUES ('p2', 'Scoped Proposal', '[\"src/foo.rs\"]')",
        [],
    )
    .unwrap();

    let affected_paths: Option<String> = conn
        .query_row(
            "SELECT affected_paths FROM task_proposals WHERE id = 'p2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(affected_paths.as_deref(), Some("[\"src/foo.rs\"]"));
}
