use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V8 migration tests - task git fields (task_branch, worktree_path, merge_commit_sha)
// ==========================================================================

#[test]
fn test_v8_adds_task_branch_column_to_tasks() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "task_branch"));
}

#[test]
fn test_v8_adds_worktree_path_column_to_tasks() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "worktree_path"));
}

#[test]
fn test_v8_adds_merge_commit_sha_column_to_tasks() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "merge_commit_sha"));
}

#[test]
fn test_v8_task_branch_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task with task_branch
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, task_branch)
         VALUES ('t1', 'p1', 'feature', 'Task', 'ralphx/test/task-t1')",
        [],
    );
    assert!(result.is_ok());

    // Verify task_branch was stored
    let branch: Option<String> = conn
        .query_row(
            "SELECT task_branch FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(branch, Some("ralphx/test/task-t1".to_string()));
}

#[test]
fn test_v8_worktree_path_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task with worktree_path
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, worktree_path)
         VALUES ('t1', 'p1', 'feature', 'Task', '/home/user/ralphx-worktrees/test/task-t1')",
        [],
    );
    assert!(result.is_ok());

    // Verify worktree_path was stored
    let path: Option<String> = conn
        .query_row(
            "SELECT worktree_path FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(path, Some("/home/user/ralphx-worktrees/test/task-t1".to_string()));
}

#[test]
fn test_v8_merge_commit_sha_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task with merge_commit_sha
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, merge_commit_sha)
         VALUES ('t1', 'p1', 'feature', 'Task', 'abc123def456')",
        [],
    );
    assert!(result.is_ok());

    // Verify merge_commit_sha was stored
    let sha: Option<String> = conn
        .query_row(
            "SELECT merge_commit_sha FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sha, Some("abc123def456".to_string()));
}

#[test]
fn test_v8_git_fields_allow_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task without git fields (all NULL)
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title)
         VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    );
    assert!(result.is_ok());

    // Verify all git fields are NULL
    let (branch, path, sha): (Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT task_branch, worktree_path, merge_commit_sha FROM tasks WHERE id = 't1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(branch, None);
    assert_eq!(path, None);
    assert_eq!(sha, None);
}

#[test]
fn test_v8_git_fields_can_be_updated() {
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

    // Update all git fields
    conn.execute(
        "UPDATE tasks SET task_branch = 'ralphx/test/task-t1', worktree_path = '/path/to/worktree', merge_commit_sha = 'sha123' WHERE id = 't1'",
        [],
    )
    .unwrap();

    let (branch, path, sha): (Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT task_branch, worktree_path, merge_commit_sha FROM tasks WHERE id = 't1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(branch, Some("ralphx/test/task-t1".to_string()));
    assert_eq!(path, Some("/path/to/worktree".to_string()));
    assert_eq!(sha, Some("sha123".to_string()));

    // Clear git fields
    conn.execute(
        "UPDATE tasks SET task_branch = NULL, worktree_path = NULL, merge_commit_sha = NULL WHERE id = 't1'",
        [],
    )
    .unwrap();

    let (branch, path, sha): (Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT task_branch, worktree_path, merge_commit_sha FROM tasks WHERE id = 't1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(branch, None);
    assert_eq!(path, None);
    assert_eq!(sha, None);
}

#[test]
fn test_v8_migration_is_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migrations twice to verify idempotency
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    // All columns should still exist
    assert!(helpers::column_exists(&conn, "tasks", "task_branch"));
    assert!(helpers::column_exists(&conn, "tasks", "worktree_path"));
    assert!(helpers::column_exists(&conn, "tasks", "merge_commit_sha"));
}
