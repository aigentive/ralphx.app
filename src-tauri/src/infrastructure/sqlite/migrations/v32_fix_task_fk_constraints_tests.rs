// V32 migration tests - FK constraint fixes for task_proposals and artifacts

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

/// Helper to create a project
fn create_project(conn: &rusqlite::Connection, id: &str) {
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES (?1, 'Test Project', '/tmp/test')",
        [id],
    )
    .unwrap();
}

/// Helper to create a task
fn create_task(conn: &rusqlite::Connection, id: &str, project_id: &str) {
    conn.execute(
        "INSERT INTO tasks (id, project_id, category, title, internal_status)
         VALUES (?1, ?2, 'feature', 'Test Task', 'backlog')",
        rusqlite::params![id, project_id],
    )
    .unwrap();
}

/// Helper to create an ideation session
fn create_session(conn: &rusqlite::Connection, id: &str, project_id: &str) {
    conn.execute(
        "INSERT INTO ideation_sessions (id, project_id, status, created_at, updated_at)
         VALUES (?1, ?2, 'active',
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
                 strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        rusqlite::params![id, project_id],
    )
    .unwrap();
}

/// Helper to create a task proposal
fn create_proposal(conn: &rusqlite::Connection, id: &str, session_id: &str, created_task_id: Option<&str>) {
    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
         VALUES (?1, ?2, 'Test Proposal', 'feature', 'medium')",
        rusqlite::params![id, session_id],
    )
    .unwrap();

    if let Some(task_id) = created_task_id {
        conn.execute(
            "UPDATE task_proposals SET created_task_id = ?1 WHERE id = ?2",
            rusqlite::params![task_id, id],
        )
        .unwrap();
    }
}

/// Helper to create an artifact
fn create_artifact(conn: &rusqlite::Connection, id: &str, task_id: Option<&str>) {
    conn.execute(
        "INSERT INTO artifacts (id, type, name, content_type, created_by)
         VALUES (?1, 'plan', 'Test Artifact', 'markdown', 'test-user')",
        [id],
    )
    .unwrap();

    if let Some(t_id) = task_id {
        conn.execute(
            "UPDATE artifacts SET task_id = ?1 WHERE id = ?2",
            rusqlite::params![t_id, id],
        )
        .unwrap();
    }
}

#[test]
fn test_v32_task_proposals_created_task_id_has_on_delete_set_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data
    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");
    create_session(&conn, "s-1", "p-1");
    create_proposal(&conn, "prop-1", "s-1", Some("t-1"));

    // Verify the proposal has the created_task_id set
    let task_id: Option<String> = conn
        .query_row(
            "SELECT created_task_id FROM task_proposals WHERE id = 'prop-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(task_id, Some("t-1".to_string()));

    // Delete the task
    conn.execute("DELETE FROM tasks WHERE id = 't-1'", [])
        .unwrap();

    // Verify the proposal still exists but created_task_id is now NULL
    let task_id_after: Option<String> = conn
        .query_row(
            "SELECT created_task_id FROM task_proposals WHERE id = 'prop-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(task_id_after.is_none());

    // Verify the proposal itself is still there
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_proposals WHERE id = 'prop-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_v32_artifacts_task_id_has_on_delete_set_null() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data
    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");
    create_artifact(&conn, "art-1", Some("t-1"));

    // Verify the artifact has the task_id set
    let task_id: Option<String> = conn
        .query_row(
            "SELECT task_id FROM artifacts WHERE id = 'art-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(task_id, Some("t-1".to_string()));

    // Delete the task
    conn.execute("DELETE FROM tasks WHERE id = 't-1'", [])
        .unwrap();

    // Verify the artifact still exists but task_id is now NULL
    let task_id_after: Option<String> = conn
        .query_row(
            "SELECT task_id FROM artifacts WHERE id = 'art-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(task_id_after.is_none());

    // Verify the artifact itself is still there
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM artifacts WHERE id = 'art-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_v32_proposal_dependencies_survive_recreation() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data
    create_project(&conn, "p-1");
    create_session(&conn, "s-1", "p-1");
    create_proposal(&conn, "prop-1", "s-1", None);
    create_proposal(&conn, "prop-2", "s-1", None);

    // Create a proposal dependency
    conn.execute(
        "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
         VALUES ('dep-1', 'prop-1', 'prop-2')",
        [],
    )
    .unwrap();

    // Verify the dependency exists
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM proposal_dependencies WHERE id = 'dep-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Query for the proposals and verify they're intact
    let (prop1, prop2): (String, String) = conn
        .query_row(
            "SELECT proposal_id, depends_on_proposal_id FROM proposal_dependencies WHERE id = 'dep-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(prop1, "prop-1");
    assert_eq!(prop2, "prop-2");
}

#[test]
fn test_v32_task_proposals_nullable_created_task_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data
    create_project(&conn, "p-1");
    create_session(&conn, "s-1", "p-1");
    create_proposal(&conn, "prop-1", "s-1", None);

    // Verify created_task_id is NULL when not set
    let task_id: Option<String> = conn
        .query_row(
            "SELECT created_task_id FROM task_proposals WHERE id = 'prop-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(task_id.is_none());
}

#[test]
fn test_v32_artifacts_nullable_task_id() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data
    create_artifact(&conn, "art-1", None);

    // Verify task_id is NULL when not set
    let task_id: Option<String> = conn
        .query_row(
            "SELECT task_id FROM artifacts WHERE id = 'art-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(task_id.is_none());
}

#[test]
fn test_v32_task_proposals_indexes_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(
        &conn,
        "idx_task_proposals_session_id"
    ));
    assert!(helpers::index_exists(
        &conn,
        "idx_task_proposals_sort_order"
    ));
    assert!(helpers::index_exists(
        &conn,
        "idx_task_proposals_created_task_id"
    ));
}

#[test]
fn test_v32_artifacts_indexes_exist() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::index_exists(&conn, "idx_artifacts_bucket"));
    assert!(helpers::index_exists(&conn, "idx_artifacts_type"));
    assert!(helpers::index_exists(&conn, "idx_artifacts_task"));
}

#[test]
fn test_v32_data_preserved_in_task_proposals() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data with various fields
    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");
    create_session(&conn, "s-1", "p-1");

    conn.execute(
        "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority,
                                     priority_score, estimated_complexity, status, selected, created_task_id)
         VALUES ('prop-1', 's-1', 'Complex Feature', 'feature', 'high', 85, 'high', 'selected', 1, 't-1')",
        [],
    )
    .unwrap();

    // Query and verify all data is preserved
    let (title, category, suggested_priority, priority_score, estimated_complexity, status, selected, created_task_id):
        (String, String, String, i32, String, String, i32, Option<String>) = conn
        .query_row(
            "SELECT title, category, suggested_priority, priority_score, estimated_complexity, status, selected, created_task_id
             FROM task_proposals WHERE id = 'prop-1'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(title, "Complex Feature");
    assert_eq!(category, "feature");
    assert_eq!(suggested_priority, "high");
    assert_eq!(priority_score, 85);
    assert_eq!(estimated_complexity, "high");
    assert_eq!(status, "selected");
    assert_eq!(selected, 1);
    assert_eq!(created_task_id, Some("t-1".to_string()));
}

#[test]
fn test_v32_data_preserved_in_artifacts() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test data with various fields
    create_project(&conn, "p-1");
    create_task(&conn, "t-1", "p-1");

    conn.execute(
        "INSERT INTO artifacts (id, type, name, content_type, content_text, created_by, version, task_id)
         VALUES ('art-1', 'plan', 'Migration Plan', 'markdown', 'Plan content', 'test-user', 2, 't-1')",
        [],
    )
    .unwrap();

    // Query and verify all data is preserved
    let (type_val, name, content_type, content_text, created_by, version, task_id):
        (String, String, String, Option<String>, String, i32, Option<String>) = conn
        .query_row(
            "SELECT type, name, content_type, content_text, created_by, version, task_id
             FROM artifacts WHERE id = 'art-1'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(type_val, "plan");
    assert_eq!(name, "Migration Plan");
    assert_eq!(content_type, "markdown");
    assert_eq!(content_text, Some("Plan content".to_string()));
    assert_eq!(created_by, "test-user");
    assert_eq!(version, 2);
    assert_eq!(task_id, Some("t-1".to_string()));
}

#[test]
fn test_v32_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();
    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    // Verify the fixes are still in place
    assert!(helpers::index_exists(
        &conn,
        "idx_task_proposals_session_id"
    ));
    assert!(helpers::index_exists(&conn, "idx_artifacts_task"));
}
