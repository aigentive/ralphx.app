use rusqlite::Connection;

use super::v20260410124500_chat_message_usage;

#[test]
fn test_chat_message_usage_migration_adds_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE chat_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT
        );",
    )
    .unwrap();

    v20260410124500_chat_message_usage::migrate(&conn).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(chat_messages)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for expected in [
        "input_tokens",
        "output_tokens",
        "cache_creation_tokens",
        "cache_read_tokens",
        "estimated_usd",
    ] {
        assert!(columns.iter().any(|value| value == expected));
    }
}
