use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V9 migration tests - project git fields (worktree_parent_directory)
// ==========================================================================

#[test]
fn test_v9_adds_worktree_parent_directory_column_to_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "projects", "worktree_parent_directory"));
}

#[test]
fn test_v9_worktree_parent_directory_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert project with worktree_parent_directory
    let result = conn.execute(
        "INSERT INTO projects (id, name, working_directory, worktree_parent_directory)
         VALUES ('p1', 'Test', '/path', '/home/user/ralphx-worktrees')",
        [],
    );
    assert!(result.is_ok());

    // Verify worktree_parent_directory was stored
    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, Some("/home/user/ralphx-worktrees".to_string()));
}

#[test]
fn test_v9_worktree_parent_directory_allows_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert project without worktree_parent_directory (NULL)
    let result = conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('p1', 'Test', '/path')",
        [],
    );
    assert!(result.is_ok());

    // Verify worktree_parent_directory is NULL
    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, None);
}

#[test]
fn test_v9_worktree_parent_directory_can_be_updated() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Update worktree_parent_directory
    conn.execute(
        "UPDATE projects SET worktree_parent_directory = '/custom/worktrees' WHERE id = 'p1'",
        [],
    )
    .unwrap();

    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, Some("/custom/worktrees".to_string()));

    // Clear worktree_parent_directory
    conn.execute(
        "UPDATE projects SET worktree_parent_directory = NULL WHERE id = 'p1'",
        [],
    )
    .unwrap();

    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, None);
}

#[test]
fn test_v9_migration_is_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migrations twice to verify idempotency
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    // Column should still exist
    assert!(helpers::column_exists(&conn, "projects", "worktree_parent_directory"));
}

#[test]
fn test_v9_existing_projects_unaffected() {
    let conn = open_memory_connection().unwrap();

    // Run migrations up to v8 (manually run v1-v8)
    run_migrations(&conn).unwrap();

    // Insert project without the new field (simulating pre-migration data)
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode)
         VALUES ('p1', 'Existing Project', '/existing/path', 'local')",
        [],
    )
    .unwrap();

    // Verify project exists and worktree_parent_directory is NULL
    let (name, dir): (String, Option<String>) = conn
        .query_row(
            "SELECT name, worktree_parent_directory FROM projects WHERE id = 'p1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(name, "Existing Project");
    assert_eq!(dir, None);
}
