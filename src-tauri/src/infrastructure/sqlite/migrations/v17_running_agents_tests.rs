// V17 migration tests - running_agents table

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v17_table_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master
             WHERE type = 'table' AND name = 'running_agents'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(table_exists, "running_agents table should exist");
}

#[test]
fn test_v17_primary_key() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert first agent
    conn.execute(
        "INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id, started_at)
         VALUES ('TaskExecution', 'task-1', 12345, 'conv-1', 'run-1', '2026-02-08T00:00:00+00:00')",
        [],
    )
    .unwrap();

    // Insert with different key should succeed
    conn.execute(
        "INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id, started_at)
         VALUES ('Review', 'task-2', 12346, 'conv-2', 'run-2', '2026-02-08T00:00:00+00:00')",
        [],
    )
    .unwrap();

    // Insert with DUPLICATE primary key should fail
    let result = conn.execute(
        "INSERT INTO running_agents (context_type, context_id, pid, conversation_id, agent_run_id, started_at)
         VALUES ('TaskExecution', 'task-1', 99999, 'conv-3', 'run-3', '2026-02-08T00:00:00+00:00')",
        [],
    );

    assert!(
        result.is_err(),
        "Duplicate (context_type, context_id) should violate PRIMARY KEY constraint"
    );
}

#[test]
fn test_v17_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error (IF NOT EXISTS)
    run_migrations(&conn).unwrap();

    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master
             WHERE type = 'table' AND name = 'running_agents'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(table_exists);
}
