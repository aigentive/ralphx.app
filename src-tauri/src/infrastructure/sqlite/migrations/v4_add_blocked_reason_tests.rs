use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V4 migration tests - blocked_reason
// ==========================================================================

#[test]
fn test_v4_adds_blocked_reason_column_to_tasks() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify the blocked_reason column exists on tasks
    assert!(helpers::column_exists(&conn, "tasks", "blocked_reason"));
}

#[test]
fn test_v4_blocked_reason_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task with blocked_reason
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, blocked_reason)
         VALUES ('t1', 'p1', 'feature', 'Task', 'Waiting for API key')",
        [],
    );
    assert!(result.is_ok());

    // Verify blocked_reason was stored
    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, Some("Waiting for API key".to_string()));
}

#[test]
fn test_v4_blocked_reason_allows_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task without blocked_reason (NULL)
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title)
         VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    );
    assert!(result.is_ok());

    // Verify blocked_reason is NULL
    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, None);
}

#[test]
fn test_v4_blocked_reason_can_be_updated() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title)
         VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();

    // Update blocked_reason
    conn.execute(
        "UPDATE tasks SET blocked_reason = 'Blocked by dependency' WHERE id = 't1'",
        [],
    )
    .unwrap();

    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, Some("Blocked by dependency".to_string()));

    // Clear blocked_reason
    conn.execute(
        "UPDATE tasks SET blocked_reason = NULL WHERE id = 't1'",
        [],
    )
    .unwrap();

    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, None);
}
