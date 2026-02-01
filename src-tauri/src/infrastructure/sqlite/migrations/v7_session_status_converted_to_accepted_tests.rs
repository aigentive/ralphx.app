//! Tests for migration v7: session status 'converted' to 'accepted'

use rusqlite::Connection;

use super::v7_session_status_converted_to_accepted;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    // Create minimal schema needed for this migration
    conn.execute(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            archived_at TEXT,
            converted_at TEXT
        )",
        [],
    )
    .expect("Failed to create ideation_sessions table");

    conn
}

#[test]
fn test_migration_updates_converted_to_accepted() {
    let conn = setup_test_db();

    // Insert sessions with various statuses including 'converted'
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s1', 'p1', 'active', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s2', 'p1', 'converted', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s3', 'p1', 'archived', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s4', 'p1', 'converted', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    // Run migration
    v7_session_status_converted_to_accepted::migrate(&conn).unwrap();

    // Verify 'converted' sessions are now 'accepted'
    let s1_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's1'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s1_status, "active", "active session should remain active");

    let s2_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's2'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s2_status, "accepted", "converted session should be accepted");

    let s3_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's3'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s3_status, "archived", "archived session should remain archived");

    let s4_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's4'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s4_status, "accepted", "converted session should be accepted");
}

#[test]
fn test_migration_is_idempotent() {
    let conn = setup_test_db();

    // Insert a session that's already 'accepted'
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s1', 'p1', 'accepted', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    // Run migration twice
    v7_session_status_converted_to_accepted::migrate(&conn).unwrap();
    v7_session_status_converted_to_accepted::migrate(&conn).unwrap();

    // Should still be 'accepted'
    let status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's1'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(status, "accepted");
}

#[test]
fn test_migration_no_converted_sessions() {
    let conn = setup_test_db();

    // Insert sessions that are NOT 'converted'
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s1', 'p1', 'active', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES ('s2', 'p1', 'archived', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();

    // Run migration - should succeed even with no 'converted' sessions
    v7_session_status_converted_to_accepted::migrate(&conn).unwrap();

    // Verify nothing changed
    let s1_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's1'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s1_status, "active");

    let s2_status: String = conn
        .query_row("SELECT status FROM ideation_sessions WHERE id = 's2'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(s2_status, "archived");
}
