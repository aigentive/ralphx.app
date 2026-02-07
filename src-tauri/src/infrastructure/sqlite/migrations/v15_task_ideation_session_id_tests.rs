// V15 migration tests - ideation_session_id column on tasks

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

/// Helper to create a project (required FK parent for tasks)
fn create_project(conn: &rusqlite::Connection, id: &str) {
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES (?1, 'Test Project', '/tmp/test')",
        [id],
    )
    .unwrap();
}

/// Helper to create an ideation session (required FK parent for task_proposals)
fn create_session(conn: &rusqlite::Connection, id: &str, project_id: &str) {
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES (?1, ?2, 'active',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        rusqlite::params![id, project_id],
    )
    .unwrap();
}

#[test]
fn test_v15_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "ideation_session_id"));
}

#[test]
fn test_v15_column_is_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");

    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at)
         VALUES ('t-1', 'p-1', 'feature', 'Test', 'backlog',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT ideation_session_id FROM tasks WHERE id = 't-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(result.is_none());
}

#[test]
fn test_v15_backfill_from_proposals() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "sess-abc", "p-1");

    // Insert a task
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at)
         VALUES ('t-backfill', 'p-1', 'feature', 'Backfill Test', 'backlog',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Insert a proposal linking this task to a session
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority, created_task_id, created_at, updated_at)
         VALUES ('prop-1', 'sess-abc', 'Prop', 'feature', 'medium', 't-backfill',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Re-run v15 migration to trigger backfill (idempotent)
    super::v15_task_ideation_session_id::migrate(&conn).unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT ideation_session_id FROM tasks WHERE id = 't-backfill'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, Some("sess-abc".to_string()));
}

#[test]
fn test_v15_backfill_skips_tasks_without_proposals() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");

    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at)
         VALUES ('t-noprop', 'p-1', 'feature', 'No Proposal', 'backlog',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    super::v15_task_ideation_session_id::migrate(&conn).unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT ideation_session_id FROM tasks WHERE id = 't-noprop'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(result.is_none());
}

#[test]
fn test_v15_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "ideation_session_id"));
}
