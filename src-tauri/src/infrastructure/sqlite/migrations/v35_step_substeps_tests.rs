// V35 migration tests - parent_step_id and scope_context columns on task_steps

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

/// Helper to create a task (required FK parent for task_steps)
fn create_task(conn: &rusqlite::Connection, id: &str, project_id: &str) {
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at)
         VALUES (?1, ?2, 'feature', 'Test Task', 'backlog',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        rusqlite::params![id, project_id],
    )
    .unwrap();
}

#[test]
fn test_v35_parent_step_id_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "task_steps",
        "parent_step_id"
    ));
}

#[test]
fn test_v35_scope_context_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "task_steps", "scope_context"));
}

#[test]
fn test_v35_columns_are_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    // Insert step without parent_step_id or scope_context
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, created_at, updated_at)
         VALUES ('step-1', 't-1', 'Test Step', 'pending', 0, 'test',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    let parent_step_id: Option<String> = conn
        .query_row(
            "SELECT parent_step_id FROM task_steps WHERE id = 'step-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let scope_context: Option<String> = conn
        .query_row(
            "SELECT scope_context FROM task_steps WHERE id = 'step-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(parent_step_id.is_none());
    assert!(scope_context.is_none());
}

#[test]
fn test_v35_existing_data_preserved() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    // Insert step before migration (simulated - but migration already ran)
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, created_at, updated_at)
         VALUES ('step-old', 't-1', 'Old Step', 'completed', 0, 'test',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Re-run migration to verify idempotency
    super::v35_step_substeps::migrate(&conn).unwrap();

    // Verify old step still exists with all fields
    let (title, status): (String, String) = conn
        .query_row(
            "SELECT title, status FROM task_steps WHERE id = 'step-old'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(title, "Old Step");
    assert_eq!(status, "completed");
}

#[test]
fn test_v35_can_set_parent_step_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    // Insert parent step
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, created_at, updated_at)
         VALUES ('parent-step', 't-1', 'Parent Step', 'in_progress', 0, 'test',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Insert sub-step with parent_step_id
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, parent_step_id, created_at, updated_at)
         VALUES ('sub-step', 't-1', 'Sub Step', 'pending', 1, 'test', 'parent-step',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    let parent_step_id: Option<String> = conn
        .query_row(
            "SELECT parent_step_id FROM task_steps WHERE id = 'sub-step'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(parent_step_id, Some("parent-step".to_string()));
}

#[test]
fn test_v35_can_set_scope_context() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    let scope_json = r#"{"files":["src/foo.rs","src/bar.rs"],"instructions":"Implement caching"}"#;

    // Insert step with scope_context
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, scope_context, created_at, updated_at)
         VALUES ('step-scoped', 't-1', 'Scoped Step', 'pending', 0, 'test', ?1,
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [scope_json],
    )
    .unwrap();

    let scope_context: Option<String> = conn
        .query_row(
            "SELECT scope_context FROM task_steps WHERE id = 'step-scoped'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(scope_context, Some(scope_json.to_string()));
}

#[test]
fn test_v35_can_query_sub_steps_by_parent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    // Insert parent step
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, created_at, updated_at)
         VALUES ('parent', 't-1', 'Parent', 'in_progress', 0, 'test',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Insert two sub-steps
    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, parent_step_id, created_at, updated_at)
         VALUES ('sub-1', 't-1', 'Sub 1', 'completed', 1, 'test', 'parent',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_by, parent_step_id, created_at, updated_at)
         VALUES ('sub-2', 't-1', 'Sub 2', 'in_progress', 2, 'test', 'parent',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    // Query sub-steps by parent_step_id
    let mut stmt = conn
        .prepare("SELECT id FROM task_steps WHERE parent_step_id = 'parent' ORDER BY sort_order")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(ids, vec!["sub-1", "sub-2"]);
}

#[test]
fn test_v35_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "task_steps",
        "parent_step_id"
    ));
    assert!(helpers::column_exists(&conn, "task_steps", "scope_context"));
}
