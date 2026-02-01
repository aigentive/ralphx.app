use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

// ==========================================================================
// Core migration system tests
// ==========================================================================

#[test]
fn test_schema_version_constant() {
    assert_eq!(SCHEMA_VERSION, 6);
}

#[test]
fn test_run_migrations_creates_migrations_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_run_migrations_sets_schema_version() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let version = get_schema_version(&conn).unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}

#[test]
fn test_run_migrations_is_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run migrations twice
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    // Should still work and have correct version
    let version = get_schema_version(&conn).unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}

// ==========================================================================
// Core tables existence tests
// ==========================================================================

#[test]
fn test_creates_projects_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let result = conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_tasks_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_tasks_has_all_columns() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Test that all expected columns exist
    let result = conn.execute(
        "INSERT INTO tasks (
            id, project_id, category, title, description, priority,
            internal_status, needs_qa, qa_prep_status, qa_test_status,
            needs_review_point, external_status, wave, checkpoint_type,
            phase_id, plan_id, must_haves_json, archived_at
        ) VALUES (
            't1', 'p1', 'feature', 'Task', 'desc', 1,
            'backlog', 1, 'pending', 'pending',
            0, NULL, NULL, NULL,
            NULL, NULL, NULL, NULL
        )",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Relationship tables tests
// ==========================================================================

#[test]
fn test_creates_task_dependencies_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task 1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t2', 'p1', 'feature', 'Task 2')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "INSERT INTO task_dependencies (id, task_id, depends_on_task_id) VALUES ('d1', 't1', 't2')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_task_dependencies_self_reference_check() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    // Self-reference should be rejected by CHECK constraint
    let result = conn.execute(
        "INSERT INTO task_dependencies (id, task_id, depends_on_task_id) VALUES ('d1', 't1', 't1')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn test_task_dependencies_unique_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task 1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t2', 'p1', 'feature', 'Task 2')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO task_dependencies (id, task_id, depends_on_task_id) VALUES ('d1', 't1', 't2')",
        [],
    )
    .unwrap();

    // Duplicate should be rejected by UNIQUE constraint
    let result = conn.execute(
        "INSERT INTO task_dependencies (id, task_id, depends_on_task_id) VALUES ('d2', 't1', 't2')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn test_creates_task_blockers_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t1', 'p1', 'feature', 'Task 1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title) VALUES ('t2', 'p1', 'feature', 'Task 2')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('t1', 't2')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// State tracking tables tests
// ==========================================================================

#[test]
fn test_creates_task_state_history_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    let result = conn.execute(
        "INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by)
         VALUES ('h1', 't1', 'backlog', 'ready', 'user')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_task_state_data_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    let result = conn.execute(
        "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('t1', 'executing', '{}')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Review system tables tests
// ==========================================================================

#[test]
fn test_creates_reviews_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    let result = conn.execute(
        "INSERT INTO reviews (id, project_id, task_id, reviewer_type) VALUES ('r1', 'p1', 't1', 'ai')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_review_actions_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO reviews (id, project_id, task_id, reviewer_type) VALUES ('r1', 'p1', 't1', 'ai')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "INSERT INTO review_actions (id, review_id, action_type) VALUES ('a1', 'r1', 'approved')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_review_notes_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    let result = conn.execute(
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('n1', 't1', 'ai', 'approved')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Ideation system tables tests
// ==========================================================================

#[test]
fn test_creates_ideation_sessions_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('s1', 'p1', 'Session')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_task_proposals_table() {
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

    let result = conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
         VALUES ('tp1', 's1', 'Proposal', 'feature', 'medium')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Chat system tables tests
// ==========================================================================

#[test]
fn test_creates_chat_conversations_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let result = conn.execute(
        "INSERT INTO chat_conversations (id, context_type, context_id, created_at, updated_at)
         VALUES ('c1', 'ideation', 's1', strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_creates_chat_messages_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let result = conn.execute(
        "INSERT INTO chat_messages (id, role, content) VALUES ('m1', 'user', 'Hello')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Artifact system tables tests
// ==========================================================================

#[test]
fn test_creates_artifacts_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let result = conn.execute(
        "INSERT INTO artifacts (id, type, name, content_type, created_by)
         VALUES ('a1', 'code', 'file.rs', 'text/plain', 'user')",
        [],
    );
    assert!(result.is_ok());
}

// ==========================================================================
// Settings tables tests
// ==========================================================================

#[test]
fn test_review_settings_has_defaults() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM review_settings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_ideation_settings_has_defaults() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM ideation_settings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// ==========================================================================
// Cascade delete tests
// ==========================================================================

#[test]
fn test_tasks_cascade_delete_on_project() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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

    // Delete project
    conn.execute("DELETE FROM projects WHERE id = 'p1'", [])
        .unwrap();

    // Task should be deleted (CASCADE)
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE id = 't1'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_reviews_cascade_delete_on_task() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO reviews (id, project_id, task_id, reviewer_type) VALUES ('r1', 'p1', 't1', 'ai')",
        [],
    )
    .unwrap();

    // Delete task
    conn.execute("DELETE FROM tasks WHERE id = 't1'", [])
        .unwrap();

    // Review should be deleted (CASCADE)
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM reviews WHERE id = 'r1'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

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

// ==========================================================================
// Helper function tests
// ==========================================================================

#[test]
fn test_helper_column_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "tasks", "title"));
    assert!(helpers::column_exists(&conn, "tasks", "internal_status"));
    assert!(!helpers::column_exists(&conn, "tasks", "nonexistent"));
}

#[test]
fn test_helper_table_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "tasks"));
    assert!(helpers::table_exists(&conn, "projects"));
    assert!(!helpers::table_exists(&conn, "nonexistent"));
}

#[test]
fn test_helper_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_tasks_project_id"));
    assert!(!helpers::index_exists(&conn, "nonexistent_index"));
}

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

// ==========================================================================
// V4 migration tests - blocked_reason
// ==========================================================================

#[test]
fn test_v4_adds_blocked_reason_column_to_tasks() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify the blocked_reason column exists on tasks
    assert!(helpers::column_exists(&conn, "tasks", "blocked_reason"));
}

#[test]
fn test_v4_blocked_reason_can_be_set() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task with blocked_reason
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, blocked_reason)
         VALUES ('t1', 'p1', 'feature', 'Task', 'Waiting for API key')",
        [],
    );
    assert!(result.is_ok());

    // Verify blocked_reason was stored
    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, Some("Waiting for API key".to_string()));
}

#[test]
fn test_v4_blocked_reason_allows_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();

    // Insert task without blocked_reason (NULL)
    let result = conn.execute(
        "INSERT INTO tasks (id, project_id, category, title)
         VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    );
    assert!(result.is_ok());

    // Verify blocked_reason is NULL
    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, None);
}

#[test]
fn test_v4_blocked_reason_can_be_updated() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'Test', '/path')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title)
         VALUES ('t1', 'p1', 'feature', 'Task')",
        [],
    )
    .unwrap();

    // Update blocked_reason
    conn.execute(
        "UPDATE tasks SET blocked_reason = 'Blocked by dependency' WHERE id = 't1'",
        [],
    )
    .unwrap();

    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, Some("Blocked by dependency".to_string()));

    // Clear blocked_reason
    conn.execute(
        "UPDATE tasks SET blocked_reason = NULL WHERE id = 't1'",
        [],
    )
    .unwrap();

    let reason: Option<String> = conn
        .query_row(
            "SELECT blocked_reason FROM tasks WHERE id = 't1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(reason, None);
}

// ==========================================================================
// V6 migration tests - review_issues
// ==========================================================================

#[test]
fn test_v6_creates_review_issues_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "review_issues"));
}

#[test]
fn test_v6_review_issues_can_insert() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create project, task, and review_note first
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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();

    // Insert review issue
    let result = conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 'Missing null check', 'major', 'open')",
        [],
    );
    assert!(result.is_ok());
}

#[test]
fn test_v6_review_issues_severity_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();

    // Invalid severity should fail
    let result = conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 'Issue', 'invalid_severity', 'open')",
        [],
    );
    assert!(result.is_err());

    // Valid severities should work
    for severity in ["critical", "major", "minor", "suggestion"] {
        let id = format!("ri_{}", severity);
        let result = conn.execute(
            &format!(
                "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
                 VALUES ('{}', 'rn1', 't1', 'Issue', '{}', 'open')",
                id, severity
            ),
            [],
        );
        assert!(result.is_ok(), "Failed for severity: {}", severity);
    }
}

#[test]
fn test_v6_review_issues_status_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();

    // Invalid status should fail
    let result = conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 'Issue', 'major', 'invalid_status')",
        [],
    );
    assert!(result.is_err());

    // Valid statuses should work
    for status in ["open", "in_progress", "addressed", "verified", "wontfix"] {
        let id = format!("ri_{}", status);
        let result = conn.execute(
            &format!(
                "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
                 VALUES ('{}', 'rn1', 't1', 'Issue', 'major', '{}')",
                id, status
            ),
            [],
        );
        assert!(result.is_ok(), "Failed for status: {}", status);
    }
}

#[test]
fn test_v6_review_issues_category_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();

    // Invalid category should fail
    let result = conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status, category)
         VALUES ('ri1', 'rn1', 't1', 'Issue', 'major', 'open', 'invalid_category')",
        [],
    );
    assert!(result.is_err());

    // NULL category should work (category is optional)
    let result = conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri_null', 'rn1', 't1', 'Issue', 'major', 'open')",
        [],
    );
    assert!(result.is_ok());

    // Valid categories should work
    for category in ["bug", "missing", "quality", "design"] {
        let id = format!("ri_{}", category);
        let result = conn.execute(
            &format!(
                "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status, category)
                 VALUES ('{}', 'rn1', 't1', 'Issue', 'major', 'open', '{}')",
                id, category
            ),
            [],
        );
        assert!(result.is_ok(), "Failed for category: {}", category);
    }
}

#[test]
fn test_v6_review_issues_has_all_indexes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_review_issues_task_id"));
    assert!(helpers::index_exists(&conn, "idx_review_issues_status"));
    assert!(helpers::index_exists(&conn, "idx_review_issues_review_note"));
}

#[test]
fn test_v6_review_issues_cascade_delete_on_task() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 'Issue', 'major', 'open')",
        [],
    )
    .unwrap();

    // Delete task
    conn.execute("DELETE FROM tasks WHERE id = 't1'", [])
        .unwrap();

    // Review issue should be deleted (CASCADE)
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM review_issues WHERE id = 'ri1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_v6_review_issues_cascade_delete_on_review_note() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 'Issue', 'major', 'open')",
        [],
    )
    .unwrap();

    // Delete review note
    conn.execute("DELETE FROM review_notes WHERE id = 'rn1'", [])
        .unwrap();

    // Review issue should be deleted (CASCADE)
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM review_issues WHERE id = 'ri1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_v6_review_issues_all_columns_accessible() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_at, updated_at) VALUES ('s1', 't1', 'Step 1', 'pending', 1, datetime('now'), datetime('now'))",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn2', 't1', 'ai', 'approved')",
        [],
    )
    .unwrap();

    // Insert with all columns
    let result = conn.execute(
        "INSERT INTO review_issues (
            id, review_note_id, task_id, step_id, no_step_reason,
            title, description, severity, category,
            file_path, line_number, code_snippet,
            status, resolution_notes, addressed_in_attempt, verified_by_review_id
        ) VALUES (
            'ri1', 'rn1', 't1', 's1', NULL,
            'Missing null check', 'The function does not handle null input', 'critical', 'bug',
            'src/lib.rs', 42, 'fn process(input: &str) {',
            'verified', 'Added null check', 2, 'rn2'
        )",
        [],
    );
    assert!(result.is_ok());

    // Verify we can read all columns
    let (title, severity, status, file_path, line_number): (
        String,
        String,
        String,
        Option<String>,
        Option<i32>,
    ) = conn
        .query_row(
            "SELECT title, severity, status, file_path, line_number FROM review_issues WHERE id = 'ri1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();

    assert_eq!(title, "Missing null check");
    assert_eq!(severity, "critical");
    assert_eq!(status, "verified");
    assert_eq!(file_path, Some("src/lib.rs".to_string()));
    assert_eq!(line_number, Some(42));
}

#[test]
fn test_v6_review_issues_step_set_null_on_delete() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

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
        "INSERT INTO task_steps (id, task_id, title, status, sort_order, created_at, updated_at) VALUES ('s1', 't1', 'Step 1', 'pending', 1, datetime('now'), datetime('now'))",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_notes (id, task_id, reviewer, outcome) VALUES ('rn1', 't1', 'ai', 'needs_changes')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO review_issues (id, review_note_id, task_id, step_id, title, severity, status)
         VALUES ('ri1', 'rn1', 't1', 's1', 'Issue', 'major', 'open')",
        [],
    )
    .unwrap();

    // Delete step
    conn.execute("DELETE FROM task_steps WHERE id = 's1'", [])
        .unwrap();

    // Review issue should still exist but step_id should be NULL
    let step_id: Option<String> = conn
        .query_row(
            "SELECT step_id FROM review_issues WHERE id = 'ri1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(step_id, None);
}
