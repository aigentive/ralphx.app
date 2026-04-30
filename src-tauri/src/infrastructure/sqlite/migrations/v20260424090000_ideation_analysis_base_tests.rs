use rusqlite::Connection;

use super::helpers::column_exists;
use super::v20260424090000_ideation_analysis_base::migrate;

#[test]
fn adds_ideation_analysis_base_columns() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    migrate(&conn).unwrap();

    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_base_ref_kind"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_base_ref"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_base_display_name"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_workspace_kind"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_workspace_path"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_base_commit"
    ));
    assert!(column_exists(
        &conn,
        "ideation_sessions",
        "analysis_base_locked_at"
    ));
}

#[test]
fn migration_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    migrate(&conn).unwrap();
    migrate(&conn).unwrap();
}
