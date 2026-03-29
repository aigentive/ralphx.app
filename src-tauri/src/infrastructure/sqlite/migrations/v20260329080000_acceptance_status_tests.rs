use rusqlite::Connection;

use super::helpers;
use super::v20260329080000_acceptance_status;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL
        );
        CREATE TABLE ideation_settings (
            id INTEGER PRIMARY KEY,
            plan_mode TEXT NOT NULL DEFAULT 'optional'
        );",
    )
    .expect("Failed to create test schema");
    conn
}

#[test]
fn test_acceptance_status_column_added_to_sessions() {
    let conn = setup_test_db();
    assert!(
        !helpers::column_exists(&conn, "ideation_sessions", "acceptance_status"),
        "acceptance_status should not exist before migration"
    );

    v20260329080000_acceptance_status::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "ideation_sessions", "acceptance_status"),
        "acceptance_status should exist after migration"
    );
}

#[test]
fn test_require_accept_for_finalize_column_added_to_settings() {
    let conn = setup_test_db();
    assert!(
        !helpers::column_exists(&conn, "ideation_settings", "require_accept_for_finalize"),
        "require_accept_for_finalize should not exist before migration"
    );

    v20260329080000_acceptance_status::migrate(&conn).unwrap();

    assert!(
        helpers::column_exists(&conn, "ideation_settings", "require_accept_for_finalize"),
        "require_accept_for_finalize should exist after migration"
    );
}

#[test]
fn test_existing_sessions_default_to_null_acceptance_status() {
    let conn = setup_test_db();
    conn.execute(
        "INSERT INTO ideation_sessions (id, title) VALUES ('s1', 'Pre-migration Session')",
        [],
    )
    .unwrap();

    v20260329080000_acceptance_status::migrate(&conn).unwrap();

    let acceptance_status: Option<String> = conn
        .query_row(
            "SELECT acceptance_status FROM ideation_sessions WHERE id = 's1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(acceptance_status.is_none());
}

#[test]
fn test_existing_settings_default_require_accept_to_zero() {
    let conn = setup_test_db();
    conn.execute(
        "INSERT INTO ideation_settings (id, plan_mode) VALUES (1, 'optional')",
        [],
    )
    .unwrap();

    v20260329080000_acceptance_status::migrate(&conn).unwrap();

    let require_accept: i64 = conn
        .query_row(
            "SELECT require_accept_for_finalize FROM ideation_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(require_accept, 0);
}

#[test]
fn test_sessions_can_store_acceptance_status() {
    let conn = setup_test_db();
    v20260329080000_acceptance_status::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO ideation_sessions (id, title, acceptance_status) VALUES ('s2', 'Pending Session', 'pending')",
        [],
    )
    .unwrap();

    let acceptance_status: Option<String> = conn
        .query_row(
            "SELECT acceptance_status FROM ideation_sessions WHERE id = 's2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(acceptance_status.as_deref(), Some("pending"));
}

#[test]
fn test_migration_is_idempotent() {
    let conn = setup_test_db();
    v20260329080000_acceptance_status::migrate(&conn).unwrap();
    // Running migration again should not fail
    v20260329080000_acceptance_status::migrate(&conn).unwrap();
}
