// V18 migration tests - task metadata column

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v18_metadata_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let has_column: bool = conn
        .prepare("PRAGMA table_info(tasks)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .any(|col| col == "metadata");

    assert!(has_column, "tasks table should have metadata column");
}

#[test]
fn test_v18_metadata_set_and_get() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert project first (FK constraint)
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert a task with metadata
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at, metadata)
         VALUES ('task-meta', 'proj-1', 'feature', 'Test', 'backlog', '2026-02-08T00:00:00+00:00', '2026-02-08T00:00:00+00:00', '{\"error\":\"git merge failed\"}')",
        [],
    )
    .unwrap();

    let metadata: Option<String> = conn
        .query_row(
            "SELECT metadata FROM tasks WHERE id = 'task-meta'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        metadata,
        Some("{\"error\":\"git merge failed\"}".to_string())
    );
}

#[test]
fn test_v18_metadata_defaults_to_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert project first (FK constraint)
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert a task without specifying metadata
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status, created_at, updated_at)
         VALUES ('task-no-meta', 'proj-1', 'feature', 'Test', 'backlog', '2026-02-08T00:00:00+00:00', '2026-02-08T00:00:00+00:00')",
        [],
    )
    .unwrap();

    let metadata: Option<String> = conn
        .query_row(
            "SELECT metadata FROM tasks WHERE id = 'task-no-meta'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(metadata.is_none(), "metadata should default to NULL");
}

#[test]
fn test_v18_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    let has_column: bool = conn
        .prepare("PRAGMA table_info(tasks)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .flatten()
        .any(|col| col == "metadata");

    assert!(has_column);
}
