// V21 migration tests - pending_questions and pending_permissions tables

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v21_tables_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    for table in &["pending_questions", "pending_permissions"] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "{table} table should exist");
    }
}

#[test]
fn test_v21_indexes_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let expected = [
        "idx_pending_questions_status",
        "idx_pending_questions_session_id",
        "idx_pending_permissions_status",
    ];

    for idx in &expected {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [idx],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "index {idx} should exist");
    }
}

#[test]
fn test_v21_questions_round_trip() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO pending_questions (request_id, session_id, question, header, options, multi_select, status)
         VALUES ('req-1', 'sess-1', 'Which DB?', 'Choose one', '[\"PostgreSQL\",\"SQLite\"]', 0, 'pending')",
        [],
    )
    .unwrap();

    let (question, status, multi): (String, String, i32) = conn
        .query_row(
            "SELECT question, status, multi_select FROM pending_questions WHERE request_id = 'req-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(question, "Which DB?");
    assert_eq!(status, "pending");
    assert_eq!(multi, 0);
}

#[test]
fn test_v21_questions_answer_update() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO pending_questions (request_id, session_id, question)
         VALUES ('req-2', 'sess-1', 'Pick a color')",
        [],
    )
    .unwrap();

    conn.execute(
        "UPDATE pending_questions SET status = 'answered', answer_text = 'Blue',
         resolved_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
         WHERE request_id = 'req-2'",
        [],
    )
    .unwrap();

    let (status, answer): (String, String) = conn
        .query_row(
            "SELECT status, answer_text FROM pending_questions WHERE request_id = 'req-2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(status, "answered");
    assert_eq!(answer, "Blue");
}

#[test]
fn test_v21_permissions_round_trip() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO pending_permissions (request_id, tool_name, tool_input, context, status)
         VALUES ('perm-1', 'Write', '{\"file\":\"/tmp/x\"}', 'Writing file', 'pending')",
        [],
    )
    .unwrap();

    let (tool, status): (String, String) = conn
        .query_row(
            "SELECT tool_name, status FROM pending_permissions WHERE request_id = 'perm-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(tool, "Write");
    assert_eq!(status, "pending");
}

#[test]
fn test_v21_permissions_decision_update() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO pending_permissions (request_id, tool_name)
         VALUES ('perm-2', 'Bash')",
        [],
    )
    .unwrap();

    conn.execute(
        "UPDATE pending_permissions SET status = 'decided', decision = 'allow', decision_message = 'User approved',
         resolved_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
         WHERE request_id = 'perm-2'",
        [],
    )
    .unwrap();

    let (status, decision): (String, String) = conn
        .query_row(
            "SELECT status, decision FROM pending_permissions WHERE request_id = 'perm-2'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(status, "decided");
    assert_eq!(decision, "allow");
}

#[test]
fn test_v21_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    for table in &["pending_questions", "pending_permissions"] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();

        assert!(exists, "{table} should still exist after double migration");
    }
}

#[test]
fn test_v21_created_at_defaults() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO pending_questions (request_id, session_id, question) VALUES ('req-ts', 'sess-1', 'Test?')",
        [],
    )
    .unwrap();

    let created_at: String = conn
        .query_row(
            "SELECT created_at FROM pending_questions WHERE request_id = 'req-ts'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        created_at.contains('T') && created_at.contains("+00:00"),
        "created_at should be RFC3339 UTC: {created_at}"
    );

    conn.execute(
        "INSERT INTO pending_permissions (request_id, tool_name) VALUES ('perm-ts', 'Read')",
        [],
    )
    .unwrap();

    let created_at: String = conn
        .query_row(
            "SELECT created_at FROM pending_permissions WHERE request_id = 'perm-ts'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        created_at.contains('T') && created_at.contains("+00:00"),
        "created_at should be RFC3339 UTC: {created_at}"
    );
}
