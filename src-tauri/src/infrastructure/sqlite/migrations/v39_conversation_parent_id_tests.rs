// Tests for v39 conversation_parent_id migration

#[cfg(test)]
mod tests {
    use crate::infrastructure::sqlite::migrations::helpers::column_exists;
    use crate::infrastructure::sqlite::migrations::v39_conversation_parent_id;
    use crate::infrastructure::sqlite::open_connection;
    use crate::infrastructure::sqlite::run_migrations;
    use std::path::PathBuf;

    fn setup_db() -> rusqlite::Connection {
        let conn = open_connection(&PathBuf::from(":memory:")).unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_parent_conversation_id_column_exists() {
        let conn = setup_db();
        assert!(column_exists(
            &conn,
            "chat_conversations",
            "parent_conversation_id"
        ));
    }

    #[test]
    fn test_existing_rows_have_null_parent() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO chat_conversations (id, context_type, context_id, message_count, created_at, updated_at)
             VALUES ('conv-1', 'task_execution', 'task-1', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [],
        )
        .unwrap();

        let parent: Option<String> = conn
            .query_row(
                "SELECT parent_conversation_id FROM chat_conversations WHERE id = 'conv-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(parent.is_none());
    }

    #[test]
    fn test_insert_with_parent_conversation_id() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO chat_conversations (id, context_type, context_id, message_count, created_at, updated_at)
             VALUES ('conv-1', 'task_execution', 'task-1', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chat_conversations (id, context_type, context_id, message_count, created_at, updated_at, parent_conversation_id)
             VALUES ('conv-2', 'task_execution', 'task-1', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 'conv-1')",
            [],
        )
        .unwrap();

        let parent: String = conn
            .query_row(
                "SELECT parent_conversation_id FROM chat_conversations WHERE id = 'conv-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(parent, "conv-1");
    }

    #[test]
    fn test_migration_is_idempotent() {
        let conn = setup_db();
        // Running v39 again should not error
        v39_conversation_parent_id::migrate(&conn).unwrap();
    }
}
