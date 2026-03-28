//! Tests for migration v20260327233752: pending initial prompt

use rusqlite::Connection;

use super::v20260327233752_pending_initial_prompt;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    // Create a minimal ideation_sessions table for the migration test
    conn.execute_batch(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )",
    )
    .expect("Failed to create table");
    conn
}

#[test]
fn test_migration_adds_column() {
    let conn = setup_test_db();
    v20260327233752_pending_initial_prompt::migrate(&conn).unwrap();

    // Verify the column exists by inserting and reading back
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, pending_initial_prompt) VALUES ('s1', 'p1', 'hello world')",
        [],
    )
    .expect("Insert with pending_initial_prompt should succeed");

    let prompt: Option<String> = conn
        .query_row(
            "SELECT pending_initial_prompt FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .expect("Query should succeed");

    assert_eq!(prompt, Some("hello world".to_string()));
}

#[test]
fn test_migration_allows_null() {
    let conn = setup_test_db();
    v20260327233752_pending_initial_prompt::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, pending_initial_prompt) VALUES ('s2', 'p1', NULL)",
        [],
    )
    .expect("Insert with NULL pending_initial_prompt should succeed");

    let prompt: Option<String> = conn
        .query_row(
            "SELECT pending_initial_prompt FROM ideation_sessions WHERE id = 's2'",
            [],
            |row| row.get(0),
        )
        .expect("Query should succeed");

    assert_eq!(prompt, None);
}

#[test]
fn test_migration_idempotent() {
    let conn = setup_test_db();
    // Running twice should not fail (add_column_if_not_exists)
    v20260327233752_pending_initial_prompt::migrate(&conn).unwrap();
    v20260327233752_pending_initial_prompt::migrate(&conn).unwrap();
}
