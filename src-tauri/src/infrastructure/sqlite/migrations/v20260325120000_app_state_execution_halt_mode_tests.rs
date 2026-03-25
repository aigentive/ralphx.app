use rusqlite::Connection;

use super::helpers;
use super::v20260325120000_app_state_execution_halt_mode;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");

    conn.execute_batch(
        "CREATE TABLE app_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_project_id TEXT DEFAULT NULL,
            updated_at TEXT NOT NULL
        );
        INSERT INTO app_state (id, active_project_id, updated_at)
        VALUES (1, NULL, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'));",
    )
    .expect("Failed to create test schema");

    conn
}

#[test]
fn test_adds_execution_halt_mode_column() {
    let conn = setup_test_db();

    assert!(!helpers::column_exists(
        &conn,
        "app_state",
        "execution_halt_mode"
    ));

    v20260325120000_app_state_execution_halt_mode::migrate(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "app_state",
        "execution_halt_mode"
    ));
}

#[test]
fn test_backfills_running_default_for_existing_row() {
    let conn = setup_test_db();

    v20260325120000_app_state_execution_halt_mode::migrate(&conn).unwrap();

    let halt_mode: String = conn
        .query_row(
            "SELECT execution_halt_mode FROM app_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(halt_mode, "running");
}

#[test]
fn test_idempotent() {
    let conn = setup_test_db();

    v20260325120000_app_state_execution_halt_mode::migrate(&conn).unwrap();
    v20260325120000_app_state_execution_halt_mode::migrate(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "app_state",
        "execution_halt_mode"
    ));
}
