use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V12 migration tests - fix worktree project settings
// ==========================================================================

#[test]
fn test_v12_fixes_null_base_branch_for_worktree_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert worktree project with NULL base_branch (pre-migration state simulated)
    conn.execute(
        "UPDATE projects SET base_branch = NULL WHERE 1=0",
        [],
    )
    .unwrap();

    // Insert a fresh worktree project with NULL base_branch
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, created_at, updated_at)
         VALUES ('p-wt-null', 'WT Null', '/path', 'worktree', NULL,
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    // Since migration already ran, re-run just v12 to fix
    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let base_branch: Option<String> = conn
        .query_row(
            "SELECT base_branch FROM projects WHERE id = 'p-wt-null'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(base_branch, Some("main".to_string()));
}

#[test]
fn test_v12_fixes_empty_base_branch_for_worktree_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, created_at, updated_at)
         VALUES ('p-wt-empty', 'WT Empty', '/path', 'worktree', '',
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let base_branch: Option<String> = conn
        .query_row(
            "SELECT base_branch FROM projects WHERE id = 'p-wt-empty'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(base_branch, Some("main".to_string()));
}

#[test]
fn test_v12_does_not_override_existing_base_branch() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, created_at, updated_at)
         VALUES ('p-wt-set', 'WT Set', '/path', 'worktree', 'develop',
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let base_branch: Option<String> = conn
        .query_row(
            "SELECT base_branch FROM projects WHERE id = 'p-wt-set'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(base_branch, Some("develop".to_string()));
}

#[test]
fn test_v12_does_not_affect_local_mode_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, created_at, updated_at)
         VALUES ('p-local', 'Local', '/path', 'local', NULL,
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let base_branch: Option<String> = conn
        .query_row(
            "SELECT base_branch FROM projects WHERE id = 'p-local'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(base_branch, None);
}

#[test]
fn test_v12_fixes_null_worktree_parent_directory() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, worktree_parent_directory, created_at, updated_at)
         VALUES ('p-wt-nodir', 'WT NoDir', '/path', 'worktree', NULL,
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p-wt-nodir'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, Some("~/ralphx-worktrees".to_string()));
}

#[test]
fn test_v12_fixes_empty_worktree_parent_directory() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, worktree_parent_directory, created_at, updated_at)
         VALUES ('p-wt-emptydir', 'WT EmptyDir', '/path', 'worktree', '',
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p-wt-emptydir'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, Some("~/ralphx-worktrees".to_string()));
}

#[test]
fn test_v12_does_not_override_existing_worktree_parent_directory() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, worktree_parent_directory, created_at, updated_at)
         VALUES ('p-wt-custom', 'WT Custom', '/path', 'worktree', '/custom/worktrees',
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let dir: Option<String> = conn
        .query_row(
            "SELECT worktree_parent_directory FROM projects WHERE id = 'p-wt-custom'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dir, Some("/custom/worktrees".to_string()));
}

#[test]
fn test_v12_migration_is_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, base_branch, worktree_parent_directory, created_at, updated_at)
         VALUES ('p-idem', 'Idempotent', '/path', 'worktree', NULL, NULL,
                 '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    // Run migration twice
    v12_fix_worktree_project_settings::migrate(&conn).unwrap();
    v12_fix_worktree_project_settings::migrate(&conn).unwrap();

    let base_branch: Option<String> = conn
        .query_row(
            "SELECT base_branch FROM projects WHERE id = 'p-idem'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(base_branch, Some("main".to_string()));
}
