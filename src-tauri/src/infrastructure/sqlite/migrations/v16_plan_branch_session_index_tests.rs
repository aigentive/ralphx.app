// V16 migration tests - UNIQUE index on plan_branches.session_id

use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

/// Helper to create a project (required FK parent)
fn create_project(conn: &rusqlite::Connection, id: &str) {
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES (?1, 'Test Project', '/tmp/test')",
        [id],
    )
    .unwrap();
}

/// Helper to create an ideation session (required FK parent)
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

#[test]
fn test_v16_index_exists() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let index_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_plan_branches_session_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_plan_branches_session_id should exist");
}

#[test]
fn test_v16_index_is_unique() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    create_project(&conn, "p-1");
    create_session(&conn, "sess-1", "p-1");
    create_session(&conn, "sess-2", "p-1");

    // Insert first plan branch
    conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-1', 'art-1', 'sess-1', 'p-1', 'feature/test-1', 'main', 'active')",
        [],
    )
    .unwrap();

    // Insert second with DIFFERENT session_id should succeed
    conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-2', 'art-2', 'sess-2', 'p-1', 'feature/test-2', 'main', 'active')",
        [],
    )
    .unwrap();

    // Insert with DUPLICATE session_id should fail
    let result = conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-3', 'art-3', 'sess-1', 'p-1', 'feature/test-3', 'main', 'active')",
        [],
    );

    assert!(
        result.is_err(),
        "Duplicate session_id should violate UNIQUE constraint"
    );
}

#[test]
fn test_v16_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error (IF NOT EXISTS)
    run_migrations(&conn).unwrap();

    let index_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_plan_branches_session_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists);
}
