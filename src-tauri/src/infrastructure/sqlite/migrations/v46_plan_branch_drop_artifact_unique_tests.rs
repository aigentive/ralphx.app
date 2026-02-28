// Tests for v46 plan_branch_drop_artifact_unique migration

#[cfg(test)]
mod tests {
    use crate::infrastructure::sqlite::open_connection;
    use crate::infrastructure::sqlite::run_migrations;
    use std::path::PathBuf;

    fn setup_db() -> rusqlite::Connection {
        let conn = open_connection(&PathBuf::from(":memory:")).unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_multiple_branches_same_artifact_id() {
        let conn = setup_db();

        // Insert two plan branches with the same plan_artifact_id but different session_ids
        conn.execute(
            "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
             VALUES ('pb-1', 'artifact-shared', 'session-parent', 'proj-1', 'ralphx/test/plan-art', 'main', 'active')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
             VALUES ('pb-2', 'artifact-shared', 'session-child', 'proj-1', 'ralphx/test/plan-art-child', 'main', 'active')",
            [],
        )
        .unwrap();

        // Verify both exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM plan_branches WHERE plan_artifact_id = 'artifact-shared'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_session_id_still_unique() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
             VALUES ('pb-1', 'artifact-1', 'session-1', 'proj-1', 'ralphx/test/plan-1', 'main', 'active')",
            [],
        )
        .unwrap();

        // Inserting another branch with the same session_id should fail
        let result = conn.execute(
            "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
             VALUES ('pb-2', 'artifact-2', 'session-1', 'proj-1', 'ralphx/test/plan-2', 'main', 'active')",
            [],
        );
        assert!(result.is_err(), "Should fail: session_id must be unique");
    }

    #[test]
    fn test_plan_artifact_id_index_exists() {
        let conn = setup_db();

        // Verify the non-unique index exists
        let index_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_plan_branches_plan_artifact_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            index_exists,
            "Non-unique index on plan_artifact_id should exist"
        );
    }

    #[test]
    fn test_session_id_unique_index_preserved() {
        let conn = setup_db();

        // Verify the UNIQUE index on session_id still exists after table recreation
        let index_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_plan_branches_session_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            index_exists,
            "UNIQUE index on session_id should be preserved"
        );
    }

    #[test]
    fn test_migration_preserves_existing_data() {
        let conn = setup_db();

        conn.execute(
            "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
             VALUES ('pb-1', 'artifact-1', 'session-1', 'proj-1', 'ralphx/test/plan-1', 'main', 'active')",
            [],
        )
        .unwrap();

        // Verify data survives the migration
        let branch_name: String = conn
            .query_row(
                "SELECT branch_name FROM plan_branches WHERE id = 'pb-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(branch_name, "ralphx/test/plan-1");
    }

    #[test]
    fn test_plan_artifact_id_no_unique_constraint() {
        let conn = setup_db();

        // Verify plan_artifact_id does NOT have a UNIQUE constraint (inline or index)
        let unique_index_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='index'
                   AND tbl_name='plan_branches'
                   AND name LIKE '%plan_artifact_id%'
                   AND sql LIKE '%UNIQUE%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            unique_index_count, 0,
            "plan_artifact_id should NOT have a UNIQUE index"
        );
    }
}
