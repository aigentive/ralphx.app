use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

/// Insert a minimal chat_conversations row to satisfy FK constraints
fn insert_conversation(conn: &rusqlite::Connection, id: &str) {
    conn.execute(
        "INSERT INTO chat_conversations (id, context_type, context_id)
         VALUES (?1, 'project', 'test-project')",
        [id],
    )
    .unwrap();
}

#[test]
fn test_v34_chat_attachments_table_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify table exists
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_attachments'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_v34_chat_attachments_columns_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify columns exist via PRAGMA table_info
    let mut stmt = conn.prepare("PRAGMA table_info(chat_attachments)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"conversation_id".to_string()));
    assert!(columns.contains(&"message_id".to_string()));
    assert!(columns.contains(&"file_name".to_string()));
    assert!(columns.contains(&"file_path".to_string()));
    assert!(columns.contains(&"mime_type".to_string()));
    assert!(columns.contains(&"file_size".to_string()));
    assert!(columns.contains(&"created_at".to_string()));
}

#[test]
fn test_v34_message_id_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "test-conv");

    // Insert attachment without message_id (before message is sent)
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, file_name, file_path, file_size)
         VALUES ('attach-1', 'test-conv', 'test.txt', '/path/to/test.txt', 1024)",
        [],
    )
    .unwrap();

    let message_id: Option<String> = conn
        .query_row(
            "SELECT message_id FROM chat_attachments WHERE id = 'attach-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(message_id.is_none());
}

#[test]
fn test_v34_foreign_key_cascade_delete() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "test-conv");

    // Insert attachment
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, file_name, file_path, file_size)
         VALUES ('attach-1', 'test-conv', 'test.txt', '/path/to/test.txt', 1024)",
        [],
    )
    .unwrap();

    // Verify attachment exists
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM chat_attachments WHERE id = 'attach-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Delete conversation (should cascade to attachments)
    conn.execute("DELETE FROM chat_conversations WHERE id = 'test-conv'", [])
        .unwrap();

    // Verify attachment was deleted
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM chat_attachments WHERE id = 'attach-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_v34_indexes_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Check conversation index
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_chat_attachments_conversation'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Check message index
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_chat_attachments_message'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_v34_attachments_queryable_by_conversation() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "conv-1");
    insert_conversation(&conn, "conv-2");

    // Insert attachments for conv-1
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, file_name, file_path, file_size)
         VALUES ('attach-1', 'conv-1', 'file1.txt', '/path/to/file1.txt', 1024)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, file_name, file_path, file_size)
         VALUES ('attach-2', 'conv-1', 'file2.txt', '/path/to/file2.txt', 2048)",
        [],
    )
    .unwrap();

    // Insert attachment for conv-2
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, file_name, file_path, file_size)
         VALUES ('attach-3', 'conv-2', 'file3.txt', '/path/to/file3.txt', 512)",
        [],
    )
    .unwrap();

    // Query attachments for conv-1
    let mut stmt = conn
        .prepare("SELECT id FROM chat_attachments WHERE conversation_id = 'conv-1' ORDER BY id")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(ids, vec!["attach-1", "attach-2"]);
}

#[test]
fn test_v34_attachments_queryable_by_message() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "conv-1");

    // Insert attachments with different message IDs
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, message_id, file_name, file_path, file_size)
         VALUES ('attach-1', 'conv-1', 'msg-1', 'file1.txt', '/path/to/file1.txt', 1024)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, message_id, file_name, file_path, file_size)
         VALUES ('attach-2', 'conv-1', 'msg-1', 'file2.txt', '/path/to/file2.txt', 2048)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO chat_attachments (id, conversation_id, message_id, file_name, file_path, file_size)
         VALUES ('attach-3', 'conv-1', 'msg-2', 'file3.txt', '/path/to/file3.txt', 512)",
        [],
    )
    .unwrap();

    // Query attachments for msg-1
    let mut stmt = conn
        .prepare("SELECT id FROM chat_attachments WHERE message_id = 'msg-1' ORDER BY id")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(ids, vec!["attach-1", "attach-2"]);
}
