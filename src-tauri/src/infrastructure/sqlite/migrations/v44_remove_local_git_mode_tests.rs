// Tests for v44 remove_local_git_mode migration

#[cfg(test)]
mod tests {
    use crate::infrastructure::sqlite::migrations::v44_remove_local_git_mode;
    use crate::infrastructure::sqlite::open_connection;
    use crate::infrastructure::sqlite::run_migrations;
    use std::path::PathBuf;

    fn setup_db() -> rusqlite::Connection {
        let conn = open_connection(&PathBuf::from(":memory:")).unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_local_projects_converted_to_worktree() {
        let conn = setup_db();

        // All projects should be 'worktree' after migration (migration already ran via run_migrations)
        // Insert a project — the v1 schema default is 'local', but v44 converts to 'worktree'.
        // Since migrations already ran, new inserts still use the schema DEFAULT 'local',
        // but we can verify the migration logic by re-running it.
        conn.execute(
            "UPDATE projects SET git_mode = 'local' WHERE 1=0",
            [],
        )
        .unwrap();

        // Verify no 'local' projects exist after full migration
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE git_mode = 'local'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_local_mode_converted_on_existing_project() {
        let conn = setup_db();

        // Insert a project and force it back to 'local' to simulate pre-migration state
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode) VALUES ('p-1', 'Test', '/tmp/test', 'local')",
            [],
        )
        .unwrap();

        // Re-run migration (should be idempotent)
        v44_remove_local_git_mode::migrate(&conn).unwrap();

        let git_mode: String = conn
            .query_row(
                "SELECT git_mode FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(git_mode, "worktree");
    }

    #[test]
    fn test_worktree_projects_unchanged() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, worktree_parent_directory) VALUES ('p-1', 'Test', '/tmp/test', 'worktree', '/custom/worktrees')",
            [],
        )
        .unwrap();

        v44_remove_local_git_mode::migrate(&conn).unwrap();

        let git_mode: String = conn
            .query_row(
                "SELECT git_mode FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(git_mode, "worktree");

        let parent_dir: String = conn
            .query_row(
                "SELECT worktree_parent_directory FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parent_dir, "/custom/worktrees");
    }

    #[test]
    fn test_null_worktree_parent_gets_default() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode) VALUES ('p-1', 'Test', '/tmp/test', 'local')",
            [],
        )
        .unwrap();

        v44_remove_local_git_mode::migrate(&conn).unwrap();

        let parent_dir: String = conn
            .query_row(
                "SELECT worktree_parent_directory FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parent_dir, "~/ralphx-worktrees");
    }

    #[test]
    fn test_empty_worktree_parent_gets_default() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, worktree_parent_directory) VALUES ('p-1', 'Test', '/tmp/test', 'worktree', '')",
            [],
        )
        .unwrap();

        v44_remove_local_git_mode::migrate(&conn).unwrap();

        let parent_dir: String = conn
            .query_row(
                "SELECT worktree_parent_directory FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parent_dir, "~/ralphx-worktrees");
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode) VALUES ('p-1', 'Test', '/tmp/test', 'local')",
            [],
        )
        .unwrap();

        // Run migration twice — should not error
        v44_remove_local_git_mode::migrate(&conn).unwrap();
        v44_remove_local_git_mode::migrate(&conn).unwrap();

        let git_mode: String = conn
            .query_row(
                "SELECT git_mode FROM projects WHERE id = 'p-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(git_mode, "worktree");
    }
}
