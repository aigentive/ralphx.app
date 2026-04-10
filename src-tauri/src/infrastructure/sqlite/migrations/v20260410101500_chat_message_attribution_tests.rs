use rusqlite::Connection;

use super::v20260410101500_chat_message_attribution;

#[test]
fn test_chat_message_attribution_migration_adds_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE chat_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT
        );",
    )
    .unwrap();

    v20260410101500_chat_message_attribution::migrate(&conn).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(chat_messages)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for expected in [
        "attribution_source",
        "provider_harness",
        "provider_session_id",
        "logical_model",
        "effective_model_id",
        "logical_effort",
        "effective_effort",
    ] {
        assert!(columns.iter().any(|value| value == expected));
    }
}
