//! Tests for migration v57: plan verification columns on ideation_sessions

use rusqlite::Connection;

use super::helpers;
use super::v57_plan_verification;

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    conn.execute_batch(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        );",
    )
    .expect("Failed to create test schema");

    conn
}

// ---------------------------------------------------------------------------
// Column existence tests
// ---------------------------------------------------------------------------

#[test]
fn test_verification_status_column_added() {
    let conn = setup_test_db();

    assert!(
        !helpers::column_exists(&conn, "ideation_sessions", "verification_status"),
        "column should not exist before migration"
    );

    v57_plan_verification::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "ideation_sessions", "verification_status"),
        "verification_status column should exist after migration"
    );
}

#[test]
fn test_verification_in_progress_column_added() {
    let conn = setup_test_db();

    v57_plan_verification::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "ideation_sessions", "verification_in_progress"),
        "verification_in_progress column should exist after migration"
    );
}

#[test]
fn test_verification_metadata_column_added() {
    let conn = setup_test_db();

    v57_plan_verification::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "ideation_sessions", "verification_metadata"),
        "verification_metadata column should exist after migration"
    );
}

// ---------------------------------------------------------------------------
// Default values
// ---------------------------------------------------------------------------

#[test]
fn test_verification_status_defaults_to_unverified() {
    let conn = setup_test_db();
    v57_plan_verification::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id) VALUES ('s1', 'p1')",
        [],
    )
    .unwrap();

    let status: String = conn
        .query_row(
            "SELECT verification_status FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(status, "unverified", "default verification_status should be 'unverified'");
}

#[test]
fn test_verification_in_progress_defaults_to_zero() {
    let conn = setup_test_db();
    v57_plan_verification::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id) VALUES ('s1', 'p1')",
        [],
    )
    .unwrap();

    let in_progress: i64 = conn
        .query_row(
            "SELECT verification_in_progress FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(in_progress, 0, "default verification_in_progress should be 0");
}

#[test]
fn test_verification_metadata_defaults_to_null() {
    let conn = setup_test_db();
    v57_plan_verification::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id) VALUES ('s1', 'p1')",
        [],
    )
    .unwrap();

    let metadata: Option<String> = conn
        .query_row(
            "SELECT verification_metadata FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(metadata.is_none(), "verification_metadata should default to NULL");
}

// ---------------------------------------------------------------------------
// Explicit value insert
// ---------------------------------------------------------------------------

#[test]
fn test_can_set_verification_status() {
    let conn = setup_test_db();
    v57_plan_verification::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, verification_status) VALUES ('s1', 'p1', 'verified')",
        [],
    )
    .unwrap();

    let status: String = conn
        .query_row(
            "SELECT verification_status FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(status, "verified");
}

#[test]
fn test_can_set_verification_metadata() {
    let conn = setup_test_db();
    v57_plan_verification::migrate(&conn).unwrap();

    let metadata = r#"{"v":1,"current_round":2,"rounds":[]}"#;
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, verification_metadata) VALUES ('s1', 'p1', ?1)",
        [metadata],
    )
    .unwrap();

    let stored: String = conn
        .query_row(
            "SELECT verification_metadata FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(stored, metadata);
}

// ---------------------------------------------------------------------------
// Idempotency
// ---------------------------------------------------------------------------

#[test]
fn test_migration_idempotent() {
    let conn = setup_test_db();

    // Run twice — add_column_if_not_exists guards against duplicate
    v57_plan_verification::migrate(&conn).unwrap();
    v57_plan_verification::migrate(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "ideation_sessions", "verification_status"));
    assert!(helpers::column_exists(&conn, "ideation_sessions", "verification_in_progress"));
    assert!(helpers::column_exists(&conn, "ideation_sessions", "verification_metadata"));
}
