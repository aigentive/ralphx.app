use rusqlite::Connection;

use super::v20260410093000_chat_attribution_backfill_state;

#[test]
fn test_chat_attribution_backfill_state_migration_adds_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE chat_conversations (
            id TEXT PRIMARY KEY,
            updated_at TEXT NOT NULL
        );",
    )
    .unwrap();

    v20260410093000_chat_attribution_backfill_state::migrate(&conn).unwrap();

    let mut stmt = conn
        .prepare("PRAGMA table_info(chat_conversations)")
        .unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for expected in [
        "attribution_backfill_status",
        "attribution_backfill_source",
        "attribution_backfill_source_path",
        "attribution_backfill_last_attempted_at",
        "attribution_backfill_completed_at",
        "attribution_backfill_error_summary",
    ] {
        assert!(columns.iter().any(|value| value == expected));
    }
}
