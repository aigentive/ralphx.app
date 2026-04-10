use rusqlite::Connection;

use super::v20260410113000_agent_run_usage;

#[test]
fn test_agent_run_usage_migration_adds_columns() {
    let conn = Connection::open_in_memory().unwrap();

    conn.execute_batch(
        "CREATE TABLE agent_runs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL
        );",
    )
    .unwrap();

    v20260410113000_agent_run_usage::migrate(&conn).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(agent_runs)").unwrap();
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
