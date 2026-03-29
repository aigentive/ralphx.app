//! Tests for migration v20260329113000: ideation blocker fingerprint

use rusqlite::Connection;

use super::v20260329113000_ideation_blocker_fingerprint;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )",
    )
    .expect("Failed to create ideation_sessions table");
    conn
}

#[test]
fn test_migration_adds_blocker_fingerprint_column() {
    let conn = setup_test_db();
    v20260329113000_ideation_blocker_fingerprint::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, blocker_fingerprint)
         VALUES ('s1', 'p1', 'ood:task-1:abc123def456')",
        [],
    )
    .expect("Insert with blocker_fingerprint should succeed");

    let fingerprint: Option<String> = conn
        .query_row(
            "SELECT blocker_fingerprint FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .expect("Query should succeed");

    assert_eq!(fingerprint.as_deref(), Some("ood:task-1:abc123def456"));
}

#[test]
fn test_migration_is_idempotent() {
    let conn = setup_test_db();
    v20260329113000_ideation_blocker_fingerprint::migrate(&conn).unwrap();
    v20260329113000_ideation_blocker_fingerprint::migrate(&conn).unwrap();
}
