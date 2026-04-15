//! Tests for migration v20260415164250: merge validation mode off

use rusqlite::Connection;

use super::v20260415164250_merge_validation_mode_off;
use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

fn setup_legacy_projects_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory database");
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;

         CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            worktree_branch TEXT,
            base_branch TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            worktree_parent_directory TEXT,
            use_feature_branches INTEGER NOT NULL DEFAULT 1,
            detected_analysis TEXT DEFAULT NULL,
            custom_analysis TEXT DEFAULT NULL,
            analyzed_at TEXT DEFAULT NULL,
            merge_validation_mode TEXT NOT NULL DEFAULT 'block',
            merge_strategy TEXT NOT NULL DEFAULT 'rebase',
            github_pr_enabled BOOLEAN NOT NULL DEFAULT 1,
            archived_at TEXT NULL
         );

         CREATE UNIQUE INDEX idx_projects_working_dir
            ON projects(working_directory)
            WHERE working_directory IS NOT NULL;
         CREATE INDEX idx_projects_archived_at
            ON projects(archived_at);",
    )
    .expect("Failed to create legacy projects schema");
    conn
}

#[test]
fn test_migration_backfills_existing_projects_and_repairs_default() {
    let conn = setup_legacy_projects_db();
    conn.execute_batch(
        "INSERT INTO projects (id, name, working_directory, merge_validation_mode)
             VALUES ('p-block', 'Block', '/legacy/block', 'block');
         INSERT INTO projects (id, name, working_directory, merge_validation_mode)
             VALUES ('p-warn', 'Warn', '/legacy/warn', 'warn');
         INSERT INTO projects (id, name, working_directory, use_feature_branches, merge_validation_mode)
             VALUES ('p-auto', 'Auto', '/legacy/auto', 0, 'auto_fix');",
    )
    .unwrap();

    v20260415164250_merge_validation_mode_off::migrate(&conn).unwrap();

    let non_off_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE merge_validation_mode != 'off'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        non_off_count, 0,
        "all legacy project rows should be normalized to off"
    );

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p-new', 'Fresh', '/legacy/new')",
        [],
    )
    .unwrap();
    let stored_mode: String = conn
        .query_row(
            "SELECT merge_validation_mode FROM projects WHERE id = 'p-new'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_mode, "off");

    let schema_default: String = conn
        .query_row(
            "SELECT dflt_value FROM pragma_table_info('projects') WHERE name = 'merge_validation_mode'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(schema_default, "'off'");

    let use_feature_branches: i64 = conn
        .query_row(
            "SELECT use_feature_branches FROM projects WHERE id = 'p-auto'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        use_feature_branches, 0,
        "non-merge project settings should survive repair"
    );
}

#[test]
fn test_migration_preserves_projects_indexes_and_uniqueness() {
    let conn = setup_legacy_projects_db();

    v20260415164250_merge_validation_mode_off::migrate(&conn).unwrap();

    let index_names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='projects'")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .flatten()
        .collect();

    assert!(
        index_names
            .iter()
            .any(|name| name == "idx_projects_working_dir"),
        "expected idx_projects_working_dir after repair, got {:?}",
        index_names
    );
    assert!(
        index_names
            .iter()
            .any(|name| name == "idx_projects_archived_at"),
        "expected idx_projects_archived_at after repair, got {:?}",
        index_names
    );

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p1', 'One', '/same/path')",
        [],
    )
    .unwrap();
    let duplicate_insert = conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('p2', 'Two', '/same/path')",
        [],
    );

    assert!(
        duplicate_insert.is_err(),
        "unique working_directory index should survive the repair"
    );
}

#[test]
fn test_run_migrations_uses_off_for_raw_project_inserts() {
    let conn = open_memory_connection().unwrap();
    run_migrations(&conn).unwrap();

    conn.execute(
        "INSERT INTO projects (id, name, working_directory) VALUES ('fresh', 'Fresh', '/fresh/path')",
        [],
    )
    .unwrap();

    let stored_mode: String = conn
        .query_row(
            "SELECT merge_validation_mode FROM projects WHERE id = 'fresh'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_mode, "off");

    let use_feature_branches: i64 = conn
        .query_row(
            "SELECT use_feature_branches FROM projects WHERE id = 'fresh'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        use_feature_branches, 1,
        "repaired schema should keep project defaults intact"
    );
}
