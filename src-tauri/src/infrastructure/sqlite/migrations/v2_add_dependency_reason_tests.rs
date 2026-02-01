use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V2 migration tests - dependency reason
// ==========================================================================

#[test]
fn test_v2_adds_reason_column_to_proposal_dependencies() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify the reason column exists on proposal_dependencies
    assert!(helpers::column_exists(
        &conn,
        "proposal_dependencies",
        "reason"
    ));

    // Test we can insert with reason
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('tp1', 's1', 'Proposal 1', 'feature', 'medium')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('tp2', 's1', 'Proposal 2', 'feature', 'medium')",
        [],
    )
    .unwrap();

    // Insert dependency with reason (note: column is depends_on_proposal_id, not depends_on_id)
    let result = conn.execute(
        "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id, reason) VALUES ('pd1', 'tp1', 'tp2', 'API needs database schema to exist')",
        [],
    );
    assert!(result.is_ok());

    // Verify reason was stored
    let reason: Option<String> = conn
        .query_row(
            "SELECT reason FROM proposal_dependencies WHERE proposal_id = 'tp1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        reason,
        Some("API needs database schema to exist".to_string())
    );
}

#[test]
fn test_v2_reason_column_allows_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('tp1', 's1', 'Proposal 1', 'feature', 'medium')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority) VALUES ('tp2', 's1', 'Proposal 2', 'feature', 'medium')",
        [],
    )
    .unwrap();

    // Insert dependency without reason (NULL) - note: column is depends_on_proposal_id
    let result = conn.execute(
        "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id) VALUES ('pd1', 'tp1', 'tp2')",
        [],
    );
    assert!(result.is_ok());

    // Verify reason is NULL
    let reason: Option<String> = conn
        .query_row(
            "SELECT reason FROM proposal_dependencies WHERE proposal_id = 'tp1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, None);
}
