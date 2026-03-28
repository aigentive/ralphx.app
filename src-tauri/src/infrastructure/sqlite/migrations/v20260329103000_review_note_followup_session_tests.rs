//! Tests for migration v20260329103000: review note follow-up session ids

use rusqlite::Connection;

use super::v20260329103000_review_note_followup_session;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE review_notes (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            reviewer TEXT NOT NULL,
            outcome TEXT NOT NULL,
            summary TEXT,
            notes TEXT,
            issues TEXT,
            created_at TEXT NOT NULL
        )",
    )
    .expect("Failed to create review_notes table");
    conn
}

#[test]
fn test_migration_adds_followup_session_column() {
    let conn = setup_test_db();
    v20260329103000_review_note_followup_session::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO review_notes (
            id, task_id, reviewer, outcome, followup_session_id, created_at
        ) VALUES ('rn1', 'task-1', 'ai', 'rejected', 'session-1', '2026-03-29T10:30:00Z')",
        [],
    )
    .expect("Insert with followup_session_id should succeed");

    let value: Option<String> = conn
        .query_row(
            "SELECT followup_session_id FROM review_notes WHERE id = 'rn1'",
            [],
            |row| row.get(0),
        )
        .expect("Query should succeed");

    assert_eq!(value.as_deref(), Some("session-1"));
}

#[test]
fn test_migration_is_idempotent() {
    let conn = setup_test_db();
    v20260329103000_review_note_followup_session::migrate(&conn).unwrap();
    v20260329103000_review_note_followup_session::migrate(&conn).unwrap();
}
