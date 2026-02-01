use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// Helper function tests
// ==========================================================================

#[test]
fn test_helper_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "title"));
    assert!(helpers::column_exists(&conn, "tasks", "internal_status"));
    assert!(!helpers::column_exists(&conn, "tasks", "nonexistent"));
}

#[test]
fn test_helper_table_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "tasks"));
    assert!(helpers::table_exists(&conn, "projects"));
    assert!(!helpers::table_exists(&conn, "nonexistent"));
}

#[test]
fn test_helper_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_tasks_project_id"));
    assert!(!helpers::index_exists(&conn, "nonexistent_index"));
}

// ==========================================================================
// Cascade delete tests (cross-cutting behavior)
// ==========================================================================

#[test]
fn test_tasks_cascade_delete_on_project() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();

    // Delete project
    conn.execute("DELETE FROM projects WHERE id = 'p1'", [])
        .unwrap();

    // Task should be deleted (CASCADE)
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE id = 't1'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_reviews_cascade_delete_on_task() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO reviews (id, project_id, task_id, reviewer_type) VALUES ('r1', 'p1', 't1', 'ai')",
        [],
    )
    .unwrap();

    // Delete task
    conn.execute("DELETE FROM tasks WHERE id = 't1'", [])
        .unwrap();

    // Review should be deleted (CASCADE)
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM reviews WHERE id = 'r1'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}
