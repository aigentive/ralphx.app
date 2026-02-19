// Tests for v43 session_title_source migration

#[cfg(test)]
mod tests {
    use crate::infrastructure::sqlite::migrations::helpers::column_exists;
    use crate::infrastructure::sqlite::migrations::v43_session_title_source;
    use crate::infrastructure::sqlite::open_connection;
    use crate::infrastructure::sqlite::run_migrations;
    use std::path::PathBuf;

    fn setup_db() -> rusqlite::Connection {
        let conn = open_connection(&PathBuf::from(":memory:")).unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('p-1', 'Test', '/tmp/test')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_title_source_column_exists() {
        let conn = setup_db();
        assert!(column_exists(&conn, "ideation_sessions", "title_source"));
    }

    #[test]
    fn test_existing_rows_have_null_title_source() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status) VALUES ('s-1', 'p-1', 'active')",
            [],
        )
        .unwrap();

        let title_source: Option<String> = conn
            .query_row(
                "SELECT title_source FROM ideation_sessions WHERE id = 's-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(title_source.is_none());
    }

    #[test]
    fn test_insert_with_auto_title_source() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, title_source) VALUES ('s-2', 'p-1', 'active', 'auto')",
            [],
        )
        .unwrap();

        let title_source: String = conn
            .query_row(
                "SELECT title_source FROM ideation_sessions WHERE id = 's-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title_source, "auto");
    }

    #[test]
    fn test_insert_with_user_title_source() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, status, title_source) VALUES ('s-3', 'p-1', 'active', 'user')",
            [],
        )
        .unwrap();

        let title_source: String = conn
            .query_row(
                "SELECT title_source FROM ideation_sessions WHERE id = 's-3'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title_source, "user");
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = setup_db();
        v43_session_title_source::migrate(&conn).unwrap();
    }
}
