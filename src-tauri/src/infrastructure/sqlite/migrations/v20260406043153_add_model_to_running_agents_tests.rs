//! Tests for migration v20260406043153: add model to running_agents

use rusqlite::Connection;

use super::v20260406043153_add_model_to_running_agents;

fn setup_test_db_with_running_agents() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "CREATE TABLE running_agents (
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL,
            pid INTEGER NOT NULL DEFAULT 0,
            conversation_id TEXT NOT NULL DEFAULT '',
            agent_run_id TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            worktree_path TEXT,
            last_active_at TEXT,
            PRIMARY KEY (context_type, context_id)
        );
        INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id)
            VALUES ('task', 'task-1', 12345, 'conv-1', 'run-1');",
    )
    .expect("Failed to create running_agents table");
    conn
}

#[test]
fn test_migration_adds_column() {
    let conn = setup_test_db_with_running_agents();
    v20260406043153_add_model_to_running_agents::migrate(&conn).unwrap();

    // Verify the column exists and is NULL by default
    let value: Option<String> = conn
        .query_row(
            "SELECT model FROM running_agents WHERE context_type = 'task' AND context_id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to query model column");

    assert_eq!(value, None, "model should default to NULL");
}

#[test]
fn test_migration_column_accepts_model_value() {
    let conn = setup_test_db_with_running_agents();
    v20260406043153_add_model_to_running_agents::migrate(&conn).unwrap();

    conn.execute(
        "UPDATE running_agents SET model = 'claude-sonnet-4-6' WHERE context_type = 'task' AND context_id = 'task-1'",
        [],
    )
    .expect("Failed to update model");

    let value: Option<String> = conn
        .query_row(
            "SELECT model FROM running_agents WHERE context_type = 'task' AND context_id = 'task-1'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to query model");

    assert_eq!(value, Some("claude-sonnet-4-6".to_string()));
}

#[test]
fn test_migration_existing_rows_get_null_default() {
    let conn = setup_test_db_with_running_agents();
    // Insert another row before migration
    conn.execute(
        "INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id)
         VALUES ('ideation', 'session-1', 99999, 'conv-2', 'run-2')",
        [],
    )
    .expect("Failed to insert second agent row");

    v20260406043153_add_model_to_running_agents::migrate(&conn).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM running_agents WHERE model IS NULL",
            [],
            |row| row.get(0),
        )
        .expect("Failed to count rows");

    assert_eq!(count, 2, "All existing rows should have NULL model");
}
