use rusqlite::Connection;

use super::v20260410143000_upstream_provider_metadata;

#[test]
fn test_upstream_provider_metadata_migration_adds_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE agent_runs (
            id TEXT PRIMARY KEY
        );
        CREATE TABLE chat_messages (
            id TEXT PRIMARY KEY
        );",
    )
    .unwrap();

    v20260410143000_upstream_provider_metadata::migrate(&conn).unwrap();

    for table in ["agent_runs", "chat_messages"] {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        for expected in ["upstream_provider", "provider_profile"] {
            assert!(columns.iter().any(|value| value == expected));
        }
    }
}
