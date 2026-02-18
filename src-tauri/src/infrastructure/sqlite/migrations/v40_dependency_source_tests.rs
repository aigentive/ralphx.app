// Tests for v40 dependency_source migration

#[cfg(test)]
mod tests {
    use crate::infrastructure::sqlite::migrations::helpers::column_exists;
    use crate::infrastructure::sqlite::migrations::v40_dependency_source;
    use crate::infrastructure::sqlite::open_connection;
    use crate::infrastructure::sqlite::run_migrations;
    use std::path::PathBuf;

    fn setup_db() -> rusqlite::Connection {
        let conn = open_connection(&PathBuf::from(":memory:")).unwrap();
        run_migrations(&conn).unwrap();
        // Insert prerequisite rows to satisfy FK constraints
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('p-1', 'Test', '/tmp/test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status) VALUES ('s-1', 'p-1', 'active')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('prop-1', 's-1', 'Proposal 1', 'feature', 'medium')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('prop-2', 's-1', 'Proposal 2', 'feature', 'medium')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_source_column_exists() {
        let conn = setup_db();
        assert!(column_exists(&conn, "proposal_dependencies", "source"));
    }

    #[test]
    fn test_existing_rows_default_to_auto() {
        let conn = setup_db();
        // Insert without specifying source — should default to 'auto'
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', 'prop-1', 'prop-2')",
            [],
        )
        .unwrap();

        let source: String = conn
            .query_row(
                "SELECT source FROM proposal_dependencies WHERE id = 'dep-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "auto");
    }

    #[test]
    fn test_insert_with_auto_source() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id, source)
             VALUES ('dep-2', 'prop-1', 'prop-2', 'auto')",
            [],
        )
        .unwrap();

        let source: String = conn
            .query_row(
                "SELECT source FROM proposal_dependencies WHERE id = 'dep-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "auto");
    }

    #[test]
    fn test_insert_with_manual_source() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id, source)
             VALUES ('dep-3', 'prop-1', 'prop-2', 'manual')",
            [],
        )
        .unwrap();

        let source: String = conn
            .query_row(
                "SELECT source FROM proposal_dependencies WHERE id = 'dep-3'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "manual");
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = setup_db();
        // Running v40 again should not error
        v40_dependency_source::migrate(&conn).unwrap();
    }
}
