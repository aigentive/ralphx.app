use super::*;
use crate::infrastructure::sqlite::connection::open_memory_connection;

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
