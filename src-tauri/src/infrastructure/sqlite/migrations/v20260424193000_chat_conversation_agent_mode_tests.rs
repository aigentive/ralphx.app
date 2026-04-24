use rusqlite::Connection;

use super::v20260424193000_chat_conversation_agent_mode::migrate;

#[test]
fn adds_nullable_agent_mode_column() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE chat_conversations (
            id TEXT PRIMARY KEY,
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    migrate(&conn).unwrap();

    let mut stmt = conn
        .prepare("PRAGMA table_info(chat_conversations)")
        .unwrap();
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(columns.contains(&"agent_mode".to_string()));

    conn.execute(
        "INSERT INTO chat_conversations (id, context_type, context_id, agent_mode)
         VALUES ('conv-chat', 'project', 'project-1', 'chat')",
        [],
    )
    .unwrap();
}
