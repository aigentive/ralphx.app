use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// V3 migration tests - activity_events
// ==========================================================================

#[test]
fn test_v3_creates_activity_events_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "activity_events"));
}

#[test]
fn test_v3_activity_events_can_insert_with_task_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project and task first
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();

    // Insert activity event for task
    let result = conn.execute(
        "INSERT INTO activity_events (id, task_id, event_type, role, content)
         VALUES ('ae1', 't1', 'thinking', 'agent', 'Test thinking content')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_v3_activity_events_can_insert_with_session_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project and session first
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

    // Insert activity event for session
    let result = conn.execute(
        "INSERT INTO activity_events (id, ideation_session_id, event_type, role, content)
         VALUES ('ae1', 's1', 'text', 'agent', 'Test text content')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_v3_activity_events_requires_exactly_one_context() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project, task, and session
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
        [],
    )
    .unwrap();

    // Should fail: BOTH task_id AND session_id set
    let result = conn.execute(
        "INSERT INTO activity_events (id, task_id, ideation_session_id, event_type, role, content)
         VALUES ('ae1', 't1', 's1', 'thinking', 'agent', 'Invalid')",
        [],
    );
    assert!(result.is_err());

    // Should fail: NEITHER task_id NOR session_id set
    let result = conn.execute(
        "INSERT INTO activity_events (id, event_type, role, content)
         VALUES ('ae2', 'thinking', 'agent', 'Invalid')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn test_v3_activity_events_has_all_indexes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Check all expected indexes exist
    assert!(helpers::index_exists(&conn, "idx_activity_events_task_id"));
    assert!(helpers::index_exists(&conn, "idx_activity_events_session_id"));
    assert!(helpers::index_exists(&conn, "idx_activity_events_type"));
    assert!(helpers::index_exists(&conn, "idx_activity_events_created_at"));
    assert!(helpers::index_exists(&conn, "idx_activity_events_task_cursor"));
    assert!(helpers::index_exists(&conn, "idx_activity_events_session_cursor"));
}

#[test]
fn test_v3_activity_events_cascade_delete_on_task() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project, task, and activity event
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO activity_events (id, task_id, event_type, role, content)
         VALUES ('ae1', 't1', 'thinking', 'agent', 'Test content')",
        [],
    )
    .unwrap();

    // Delete task
    conn.execute("DELETE FROM tasks WHERE id = 't1'", [])
        .unwrap();

    // Activity event should be deleted (CASCADE)
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM activity_events WHERE id = 'ae1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_v3_activity_events_cascade_delete_on_session() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project, session, and activity event
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
        "INSERT INTO activity_events (id, ideation_session_id, event_type, role, content)
         VALUES ('ae1', 's1', 'text', 'agent', 'Test content')",
        [],
    )
    .unwrap();

    // Delete session
    conn.execute("DELETE FROM ideation_sessions WHERE id = 's1'", [])
        .unwrap();

    // Activity event should be deleted (CASCADE)
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM activity_events WHERE id = 'ae1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_v3_activity_events_all_columns_accessible() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project and task
    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();

    // Insert with all optional columns
    let result = conn.execute(
        "INSERT INTO activity_events (id, task_id, internal_status, event_type, role, content, metadata, created_at)
         VALUES ('ae1', 't1', 'executing', 'tool_call', 'agent', 'Tool content', '{\"tool_use_id\": \"abc\"}', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    );
    assert!(result.is_ok());

    // Verify we can read all columns
    let (event_type, role, internal_status, metadata): (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT event_type, role, internal_status, metadata FROM activity_events WHERE id = 'ae1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(event_type, "tool_call");
    assert_eq!(role, "agent");
    assert_eq!(internal_status, Some("executing".to_string()));
    assert_eq!(metadata, Some("{\"tool_use_id\": \"abc\"}".to_string()));
}
