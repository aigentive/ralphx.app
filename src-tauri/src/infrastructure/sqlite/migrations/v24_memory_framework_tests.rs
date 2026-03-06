// Tests for v24_memory_framework migration

use super::helpers;
use super::v24_memory_framework;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

#[test]
fn test_v24_creates_all_memory_tables() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify all 5 tables exist
    assert!(
        helpers::table_exists(&conn, "project_memory_settings"),
        "project_memory_settings table should exist"
    );
    assert!(
        helpers::table_exists(&conn, "memory_entries"),
        "memory_entries table should exist"
    );
    assert!(
        helpers::table_exists(&conn, "memory_events"),
        "memory_events table should exist"
    );
    assert!(
        helpers::table_exists(&conn, "memory_rule_bindings"),
        "memory_rule_bindings table should exist"
    );
    assert!(
        helpers::table_exists(&conn, "memory_archive_jobs"),
        "memory_archive_jobs table should exist"
    );
}

#[test]
fn test_v24_creates_memory_entries_indexes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Verify indexes exist
    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_entries_project_bucket_status'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Project bucket status index should exist");

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_entries_conversation'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Conversation index should exist");

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_entries_content_hash'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Content hash index should exist");
}

#[test]
fn test_v24_creates_memory_events_indexes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_events_project'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Project index should exist");

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_events_type'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Event type index should exist");
}

#[test]
fn test_v24_creates_memory_archive_jobs_indexes() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_archive_jobs_project_status'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Project status index should exist");

    let index_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='index' AND name='idx_memory_archive_jobs_status'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 1, "Status index should exist");
}

#[test]
fn test_v24_project_memory_settings_defaults() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Insert a test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project-1', 'Test Project', '/test')",
        [],
    )
    .unwrap();

    // Re-run migration to trigger the INSERT OR IGNORE for existing projects
    v24_memory_framework::migrate(&conn).unwrap();

    // Verify default settings were inserted
    let result: (i32, String, String, i32, String, i32, i32) = conn
        .query_row(
            "SELECT enabled, maintenance_categories_json, capture_categories_json,
                    archive_enabled, archive_path, archive_auto_commit, retain_rule_snapshots
             FROM project_memory_settings
             WHERE project_id = 'test-project-1'",
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

    assert_eq!(result.0, 1, "Memory should be enabled by default");
    assert_eq!(
        result.1, "[\"execution\",\"review\",\"merge\"]",
        "Maintenance categories should match defaults"
    );
    assert_eq!(
        result.2, "[\"planning\",\"execution\",\"review\"]",
        "Capture categories should match defaults"
    );
    assert_eq!(result.3, 1, "Archive should be enabled by default");
    assert_eq!(
        result.4, ".claude/memory-archive",
        "Archive path should match default"
    );
    assert_eq!(result.5, 0, "Archive auto-commit should be disabled");
    assert_eq!(result.6, 1, "Retain rule snapshots should be enabled");
}

#[test]
fn test_v24_memory_entries_bucket_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Valid bucket should succeed
    let result = conn.execute(
        "INSERT INTO memory_entries
         (id, project_id, bucket, title, summary, details_markdown, content_hash)
         VALUES ('mem-1', 'test-project', 'architecture_patterns', 'Test', 'Summary', 'Details', 'hash1')",
        [],
    );
    assert!(result.is_ok(), "Valid bucket should be accepted");

    // Invalid bucket should fail
    let result = conn.execute(
        "INSERT INTO memory_entries
         (id, project_id, bucket, title, summary, details_markdown, content_hash)
         VALUES ('mem-2', 'test-project', 'invalid_bucket', 'Test', 'Summary', 'Details', 'hash2')",
        [],
    );
    assert!(result.is_err(), "Invalid bucket should be rejected");
}

#[test]
fn test_v24_memory_entries_status_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Valid status should succeed
    let result = conn.execute(
        "INSERT INTO memory_entries
         (id, project_id, bucket, title, summary, details_markdown, status, content_hash)
         VALUES ('mem-1', 'test-project', 'architecture_patterns', 'Test', 'Summary', 'Details', 'active', 'hash1')",
        [],
    );
    assert!(result.is_ok(), "Valid status should be accepted");

    // Invalid status should fail
    let result = conn.execute(
        "INSERT INTO memory_entries
         (id, project_id, bucket, title, summary, details_markdown, status, content_hash)
         VALUES ('mem-2', 'test-project', 'architecture_patterns', 'Test', 'Summary', 'Details', 'invalid', 'hash2')",
        [],
    );
    assert!(result.is_err(), "Invalid status should be rejected");
}

#[test]
fn test_v24_memory_events_actor_type_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Valid actor_type should succeed
    let result = conn.execute(
        "INSERT INTO memory_events
         (id, project_id, event_type, actor_type)
         VALUES ('evt-1', 'test-project', 'test_event', 'system')",
        [],
    );
    assert!(result.is_ok(), "Valid actor_type should be accepted");

    // Invalid actor_type should fail
    let result = conn.execute(
        "INSERT INTO memory_events
         (id, project_id, event_type, actor_type)
         VALUES ('evt-2', 'test-project', 'test_event', 'invalid_actor')",
        [],
    );
    assert!(result.is_err(), "Invalid actor_type should be rejected");
}

#[test]
fn test_v24_memory_archive_jobs_job_type_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Valid job_type should succeed
    let result = conn.execute(
        "INSERT INTO memory_archive_jobs
         (id, project_id, job_type)
         VALUES ('job-1', 'test-project', 'memory_snapshot')",
        [],
    );
    assert!(result.is_ok(), "Valid job_type should be accepted");

    // Invalid job_type should fail
    let result = conn.execute(
        "INSERT INTO memory_archive_jobs
         (id, project_id, job_type)
         VALUES ('job-2', 'test-project', 'invalid_type')",
        [],
    );
    assert!(result.is_err(), "Invalid job_type should be rejected");
}

#[test]
fn test_v24_memory_archive_jobs_status_constraint() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Valid status should succeed
    let result = conn.execute(
        "INSERT INTO memory_archive_jobs
         (id, project_id, job_type, status)
         VALUES ('job-1', 'test-project', 'memory_snapshot', 'pending')",
        [],
    );
    assert!(result.is_ok(), "Valid status should be accepted");

    // Invalid status should fail
    let result = conn.execute(
        "INSERT INTO memory_archive_jobs
         (id, project_id, job_type, status)
         VALUES ('job-2', 'test-project', 'memory_snapshot', 'invalid_status')",
        [],
    );
    assert!(result.is_err(), "Invalid status should be rejected");
}

#[test]
fn test_v24_memory_rule_bindings_composite_key() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    // Create test project
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('test-project', 'Test', '/test')",
        [],
    )
    .unwrap();

    // Insert first binding
    conn.execute(
        "INSERT INTO memory_rule_bindings
         (project_id, scope_key, rule_file_path)
         VALUES ('test-project', 'scope-1', 'path/to/rule.md')",
        [],
    )
    .unwrap();

    // Same project, different scope should succeed
    let result = conn.execute(
        "INSERT INTO memory_rule_bindings
         (project_id, scope_key, rule_file_path)
         VALUES ('test-project', 'scope-2', 'path/to/other.md')",
        [],
    );
    assert!(
        result.is_ok(),
        "Different scope_key should allow duplicate project_id"
    );

    // Same project and scope should fail (composite key violation)
    let result = conn.execute(
        "INSERT INTO memory_rule_bindings
         (project_id, scope_key, rule_file_path)
         VALUES ('test-project', 'scope-1', 'path/to/duplicate.md')",
        [],
    );
    assert!(
        result.is_err(),
        "Duplicate (project_id, scope_key) should be rejected"
    );
}

#[test]
fn test_v24_fresh_db_creation() {
    let conn = open_memory_connection().unwrap();

    // Run all migrations from scratch
    run_migrations(&conn).unwrap();

    // Verify schema version is 24
    let version: i32 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(
        version, 55,
        "Schema version should be 55 after fresh install"
    );

    // Verify all memory tables exist
    assert!(helpers::table_exists(&conn, "project_memory_settings"));
    assert!(helpers::table_exists(&conn, "memory_entries"));
    assert!(helpers::table_exists(&conn, "memory_events"));
    assert!(helpers::table_exists(&conn, "memory_rule_bindings"));
    assert!(helpers::table_exists(&conn, "memory_archive_jobs"));
}

#[test]
fn test_v24_upgrade_from_v23() {
    let conn = open_memory_connection().unwrap();

    // Run migrations up to v23
    run_migrations(&conn).unwrap();

    // Verify we're at v23
    let version: i32 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(version >= 23, "Should have at least v23");

    // Create a test project before v24 migration
    conn.execute(
        "INSERT INTO projects (id, name, working_directory)
         VALUES ('existing-project', 'Existing', '/existing')",
        [],
    )
    .unwrap();

    // Run v24 migration
    v24_memory_framework::migrate(&conn).unwrap();

    // Verify default settings inserted for existing project
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM project_memory_settings WHERE project_id = 'existing-project'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        count, 1,
        "Default memory settings should be created for existing projects"
    );
}

#[test]
fn test_v24_idempotent() {
    let conn = open_memory_connection().unwrap();

    // Run all migrations to set up base schema
    run_migrations(&conn).unwrap();

    // Run v24 migration again
    v24_memory_framework::migrate(&conn).unwrap();

    // Should not error and all tables should still exist
    assert!(helpers::table_exists(&conn, "project_memory_settings"));
    assert!(helpers::table_exists(&conn, "memory_entries"));
    assert!(helpers::table_exists(&conn, "memory_events"));
    assert!(helpers::table_exists(&conn, "memory_rule_bindings"));
    assert!(helpers::table_exists(&conn, "memory_archive_jobs"));
}
