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
fn test_v33_agent_run_chain_id_columns_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify columns exist via PRAGMA table_info
    let mut stmt = conn.prepare("PRAGMA table_info(agent_runs)").unwrap();
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(columns.contains(&"run_chain_id".to_string()));
    assert!(columns.contains(&"parent_run_id".to_string()));
}

#[test]
fn test_v33_run_chain_id_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "test-conv");

    // Insert a row without run_chain_id/parent_run_id — should succeed with NULL defaults
    conn.execute(
        "INSERT INTO agent_runs (id, conversation_id, status, started_at)
         VALUES ('test-run', 'test-conv', 'running', '2026-01-01T00:00:00+00:00')",
        [],
    )
    .unwrap();

    let (chain_id, parent_id): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT run_chain_id, parent_run_id FROM agent_runs WHERE id = 'test-run'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert!(chain_id.is_none());
    assert!(parent_id.is_none());
}

#[test]
fn test_v33_run_chain_id_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_agent_runs_chain'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_v33_run_chain_id_queryable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    insert_conversation(&conn, "conv-1");

    // Insert two runs with same chain_id
    conn.execute(
        "INSERT INTO agent_runs (id, conversation_id, status, started_at, run_chain_id, parent_run_id)
         VALUES ('run-1', 'conv-1', 'completed', '2026-01-01T00:00:00+00:00', 'chain-abc', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO agent_runs (id, conversation_id, status, started_at, run_chain_id, parent_run_id)
         VALUES ('run-2', 'conv-1', 'running', '2026-01-01T00:01:00+00:00', 'chain-abc', 'run-1')",
        [],
    )
    .unwrap();

    // Query by chain_id
    let mut stmt = conn
        .prepare("SELECT id FROM agent_runs WHERE run_chain_id = 'chain-abc' ORDER BY started_at")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(ids, vec!["run-1", "run-2"]);

    // Verify parent_run_id
    let parent: Option<String> = conn
        .query_row(
            "SELECT parent_run_id FROM agent_runs WHERE id = 'run-2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(parent, Some("run-1".to_string()));
}
