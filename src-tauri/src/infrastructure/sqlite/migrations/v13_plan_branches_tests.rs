// V13 migration tests - plan_branches table and use_feature_branches column

use super::helpers;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v13_creates_plan_branches_table() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "plan_branches"));
}

#[test]
fn test_v13_plan_branches_has_correct_columns() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(&conn, "plan_branches", "id"));
    assert!(helpers::column_exists(&conn, "plan_branches", "plan_artifact_id"));
    assert!(helpers::column_exists(&conn, "plan_branches", "session_id"));
    assert!(helpers::column_exists(&conn, "plan_branches", "project_id"));
    assert!(helpers::column_exists(&conn, "plan_branches", "branch_name"));
    assert!(helpers::column_exists(&conn, "plan_branches", "source_branch"));
    assert!(helpers::column_exists(&conn, "plan_branches", "status"));
    assert!(helpers::column_exists(&conn, "plan_branches", "merge_task_id"));
    assert!(helpers::column_exists(&conn, "plan_branches", "created_at"));
    assert!(helpers::column_exists(&conn, "plan_branches", "merged_at"));
}

#[test]
fn test_v13_adds_use_feature_branches_to_projects() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    assert!(helpers::column_exists(
        &conn,
        "projects",
        "use_feature_branches"
    ));
}

#[test]
fn test_v13_use_feature_branches_defaults_to_true() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a project without specifying use_feature_branches
    conn.execute(
        "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
         VALUES ('proj-1', 'Test', '/path', 'local',
         strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'),
         strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .unwrap();

    let result: i64 = conn
        .query_row(
            "SELECT use_feature_branches FROM projects WHERE id = 'proj-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, 1); // Default true
}

#[test]
fn test_v13_can_insert_plan_branch() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-1', 'art-1', 'sess-1', 'proj-1', 'ralphx/app/plan-abc', 'main', 'active')",
        [],
    )
    .unwrap();

    let (id, status): (String, String) = conn
        .query_row(
            "SELECT id, status FROM plan_branches WHERE id = 'pb-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(id, "pb-1");
    assert_eq!(status, "active");
}

#[test]
fn test_v13_plan_artifact_id_unique() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-1', 'art-1', 'sess-1', 'proj-1', 'ralphx/app/plan-abc', 'main', 'active')",
        [],
    )
    .unwrap();

    // Duplicate plan_artifact_id should fail
    let result = conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-2', 'art-1', 'sess-1', 'proj-1', 'ralphx/app/plan-def', 'main', 'active')",
        [],
    );

    assert!(result.is_err());
}

#[test]
fn test_v13_merge_task_id_nullable() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert without merge_task_id (should default to NULL)
    conn.execute(
        "INSERT INTO plan_branches (id, plan_artifact_id, session_id, project_id, branch_name, source_branch, status)
         VALUES ('pb-1', 'art-1', 'sess-1', 'proj-1', 'ralphx/app/plan-abc', 'main', 'active')",
        [],
    )
    .unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT merge_task_id FROM plan_branches WHERE id = 'pb-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(result.is_none());

    // Update with merge_task_id
    conn.execute(
        "UPDATE plan_branches SET merge_task_id = 'task-1' WHERE id = 'pb-1'",
        [],
    )
    .unwrap();

    let result: Option<String> = conn
        .query_row(
            "SELECT merge_task_id FROM plan_branches WHERE id = 'pb-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(result, Some("task-1".to_string()));
}

#[test]
fn test_v13_idempotent() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Running migrations again should not error
    run_migrations(&conn).unwrap();

    assert!(helpers::table_exists(&conn, "plan_branches"));
    assert!(helpers::column_exists(
        &conn,
        "projects",
        "use_feature_branches"
    ));
}
