use super::*;

// ===== GitMode Tests =====

#[test]
fn git_mode_default_is_worktree() {
    assert_eq!(GitMode::default(), GitMode::Worktree);
}

#[test]
fn git_mode_display_worktree() {
    assert_eq!(format!("{}", GitMode::Worktree), "worktree");
}

#[test]
fn git_mode_serializes_to_snake_case() {
    let worktree_json = serde_json::to_string(&GitMode::Worktree).expect("Should serialize");

    assert_eq!(worktree_json, "\"worktree\"");
}

#[test]
fn git_mode_deserializes_from_snake_case() {
    let worktree: GitMode = serde_json::from_str("\"worktree\"").expect("Should deserialize");

    assert_eq!(worktree, GitMode::Worktree);
}

#[test]
fn git_mode_deserializes_local_as_worktree() {
    // Backward compat: "local" in DB/JSON maps to Worktree
    let local: GitMode = serde_json::from_str("\"local\"").expect("Should deserialize");

    assert_eq!(local, GitMode::Worktree);
}

#[test]
fn git_mode_clone_works() {
    let mode = GitMode::Worktree;
    let cloned = mode;
    assert_eq!(mode, cloned);
}

#[test]
fn git_mode_equality_works() {
    assert_eq!(GitMode::Worktree, GitMode::Worktree);
}

// ===== Project Creation Tests =====

#[test]
fn project_new_creates_with_defaults() {
    let project = Project::new("My Project".to_string(), "/path/to/project".to_string());

    assert_eq!(project.name, "My Project");
    assert_eq!(project.working_directory, "/path/to/project");
    assert_eq!(project.git_mode, GitMode::Worktree);
    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
    assert!(project.base_branch.is_none());
    assert!(project.worktree_parent_directory.is_none());
}

#[test]
fn project_new_generates_unique_id() {
    let project1 = Project::new("Project 1".to_string(), "/path/1".to_string());
    let project2 = Project::new("Project 2".to_string(), "/path/2".to_string());

    assert_ne!(project1.id, project2.id);
}

#[test]
fn project_new_sets_timestamps() {
    let before = Utc::now();
    let project = Project::new("Test".to_string(), "/test".to_string());
    let after = Utc::now();

    assert!(project.created_at >= before);
    assert!(project.created_at <= after);
    assert!(project.updated_at >= before);
    assert!(project.updated_at <= after);
    assert_eq!(project.created_at, project.updated_at);
}

#[test]
fn project_worktree_mode_via_field_set() {
    let mut project = Project::new("Worktree Project".to_string(), "/main/repo".to_string());
    project.git_mode = GitMode::Worktree;
    project.base_branch = Some("main".to_string());

    assert_eq!(project.name, "Worktree Project");
    assert_eq!(project.working_directory, "/main/repo");
    assert_eq!(project.git_mode, GitMode::Worktree);
    assert_eq!(project.base_branch, Some("main".to_string()));
    assert!(project.worktree_parent_directory.is_none());
}

// ===== Project Method Tests =====

#[test]
fn project_is_worktree_returns_true_for_worktree_mode() {
    let mut project = Project::new("WT".to_string(), "/repo".to_string());
    project.git_mode = GitMode::Worktree;

    assert!(project.is_worktree());
}

#[test]
fn project_new_defaults_to_worktree_mode() {
    let project = Project::new("Test".to_string(), "/repo".to_string());

    assert!(project.is_worktree());
}

#[test]
fn project_touch_updates_timestamp() {
    let mut project = Project::new("Test".to_string(), "/test".to_string());
    let original_updated = project.updated_at;
    let original_created = project.created_at;

    // Small delay to ensure time difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    project.touch();

    assert_eq!(project.created_at, original_created);
    assert!(project.updated_at > original_updated);
}

// ===== Project Serialization Tests =====

#[test]
fn project_serializes_to_json() {
    let project = Project::new("JSON Test".to_string(), "/json/path".to_string());
    let json = serde_json::to_string(&project).expect("Should serialize");

    assert!(json.contains("\"name\":\"JSON Test\""));
    assert!(json.contains("\"working_directory\":\"/json/path\""));
    assert!(json.contains("\"git_mode\":\"worktree\""));
}

#[test]
fn project_deserializes_from_json() {
    let json = r#"{
        "id": "test-id-123",
        "name": "Deserialized",
        "working_directory": "/deser/path",
        "git_mode": "worktree",
        "worktree_path": "/wt/path",
        "worktree_branch": "feature",
        "base_branch": "main",
        "created_at": "2025-01-24T12:00:00Z",
        "updated_at": "2025-01-24T12:00:00Z"
    }"#;

    let project: Project = serde_json::from_str(json).expect("Should deserialize");

    assert_eq!(project.id.as_str(), "test-id-123");
    assert_eq!(project.name, "Deserialized");
    assert_eq!(project.working_directory, "/deser/path");
    assert_eq!(project.git_mode, GitMode::Worktree);
    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
    assert_eq!(project.base_branch, Some("main".to_string()));
}

#[test]
fn project_deserializes_with_null_optionals() {
    let json = r#"{
        "id": "test-id",
        "name": "Minimal",
        "working_directory": "/path",
        "git_mode": "worktree",
        "base_branch": null,
        "created_at": "2025-01-24T12:00:00Z",
        "updated_at": "2025-01-24T12:00:00Z"
    }"#;

    let project: Project = serde_json::from_str(json).expect("Should deserialize");

    assert!(project.base_branch.is_none());
    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
}

#[test]
fn project_roundtrip_serialization() {
    let mut original = Project::new("Roundtrip".to_string(), "/original".to_string());
    original.git_mode = GitMode::Worktree;
    original.base_branch = Some("main".to_string());

    let json = serde_json::to_string(&original).expect("Should serialize");
    let restored: Project = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(original.id, restored.id);
    assert_eq!(original.name, restored.name);
    assert_eq!(original.working_directory, restored.working_directory);
    assert_eq!(original.git_mode, restored.git_mode);
    assert_eq!(original.base_branch, restored.base_branch);
}

// ===== Project Clone Tests =====

#[test]
fn project_clone_works() {
    let original = Project::new("Clone Test".to_string(), "/clone".to_string());
    let cloned = original.clone();

    assert_eq!(original.id, cloned.id);
    assert_eq!(original.name, cloned.name);
    assert_eq!(original.working_directory, cloned.working_directory);
    assert_eq!(original.git_mode, cloned.git_mode);
}

#[test]
fn project_clone_is_independent() {
    let original = Project::new("Independent".to_string(), "/independent".to_string());
    let mut cloned = original.clone();

    cloned.touch();

    // Original should be unchanged
    assert_ne!(original.updated_at, cloned.updated_at);
}

// ===== Fallback Default Method Tests =====

#[test]
fn base_branch_or_default_returns_value_when_set() {
    let mut project = Project::new("Test".to_string(), "/test".to_string());
    project.base_branch = Some("develop".to_string());
    assert_eq!(project.base_branch_or_default(), "develop");
}

#[test]
fn base_branch_or_default_returns_main_when_none() {
    let project = Project::new("Test".to_string(), "/test".to_string());
    assert_eq!(project.base_branch_or_default(), "main");
}

#[test]
fn base_branch_or_default_returns_main_when_empty() {
    let mut project = Project::new("Test".to_string(), "/test".to_string());
    project.base_branch = Some("".to_string());
    assert_eq!(project.base_branch_or_default(), "main");
}

#[test]
fn worktree_parent_or_default_returns_value_when_set() {
    let mut project = Project::new("Test".to_string(), "/test".to_string());
    project.worktree_parent_directory = Some("/custom/worktrees".to_string());
    assert_eq!(project.worktree_parent_or_default(), "/custom/worktrees");
}

#[test]
fn worktree_parent_or_default_returns_default_when_none() {
    let project = Project::new("Test".to_string(), "/test".to_string());
    assert_eq!(project.worktree_parent_or_default(), "~/ralphx-worktrees");
}

#[test]
fn worktree_parent_or_default_returns_default_when_empty() {
    let mut project = Project::new("Test".to_string(), "/test".to_string());
    project.worktree_parent_directory = Some("".to_string());
    assert_eq!(project.worktree_parent_or_default(), "~/ralphx-worktrees");
}

// ===== MergeValidationMode Tests =====

#[test]
fn merge_validation_mode_default_is_off() {
    assert_eq!(MergeValidationMode::default(), MergeValidationMode::Off);
}

#[test]
fn merge_validation_mode_serializes() {
    assert_eq!(
        serde_json::to_string(&MergeValidationMode::Block).unwrap(),
        "\"block\""
    );
    assert_eq!(
        serde_json::to_string(&MergeValidationMode::AutoFix).unwrap(),
        "\"auto_fix\""
    );
    assert_eq!(
        serde_json::to_string(&MergeValidationMode::Warn).unwrap(),
        "\"warn\""
    );
    assert_eq!(
        serde_json::to_string(&MergeValidationMode::Off).unwrap(),
        "\"off\""
    );
}

#[test]
fn merge_validation_mode_deserializes() {
    let block: MergeValidationMode = serde_json::from_str("\"block\"").unwrap();
    let auto_fix: MergeValidationMode = serde_json::from_str("\"auto_fix\"").unwrap();
    let warn: MergeValidationMode = serde_json::from_str("\"warn\"").unwrap();
    let off: MergeValidationMode = serde_json::from_str("\"off\"").unwrap();
    assert_eq!(block, MergeValidationMode::Block);
    assert_eq!(auto_fix, MergeValidationMode::AutoFix);
    assert_eq!(warn, MergeValidationMode::Warn);
    assert_eq!(off, MergeValidationMode::Off);
}

#[test]
fn merge_validation_mode_from_str() {
    assert_eq!(
        "block".parse::<MergeValidationMode>().unwrap(),
        MergeValidationMode::Block
    );
    assert_eq!(
        "auto_fix".parse::<MergeValidationMode>().unwrap(),
        MergeValidationMode::AutoFix
    );
    assert_eq!(
        "warn".parse::<MergeValidationMode>().unwrap(),
        MergeValidationMode::Warn
    );
    assert_eq!(
        "off".parse::<MergeValidationMode>().unwrap(),
        MergeValidationMode::Off
    );
    assert!("invalid".parse::<MergeValidationMode>().is_err());
}

#[test]
fn merge_validation_mode_display() {
    assert_eq!(format!("{}", MergeValidationMode::Block), "block");
    assert_eq!(format!("{}", MergeValidationMode::AutoFix), "auto_fix");
    assert_eq!(format!("{}", MergeValidationMode::Warn), "warn");
    assert_eq!(format!("{}", MergeValidationMode::Off), "off");
}

// ===== GitMode FromStr Tests =====

#[test]
fn git_mode_from_str_local_maps_to_worktree() {
    // Backward compat: "local" parses to Worktree
    let mode: GitMode = "local".parse().unwrap();
    assert_eq!(mode, GitMode::Worktree);
}

#[test]
fn git_mode_from_str_worktree() {
    let mode: GitMode = "worktree".parse().unwrap();
    assert_eq!(mode, GitMode::Worktree);
}

#[test]
fn git_mode_from_str_invalid() {
    let result: Result<GitMode, _> = "invalid".parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.value, "invalid");
}

#[test]
fn git_mode_parse_error_displays_correctly() {
    let err = ParseGitModeError {
        value: "unknown".to_string(),
    };
    assert_eq!(err.to_string(), "unknown git mode: 'unknown'");
}

// ===== Project parse_datetime Tests =====

#[test]
fn project_parse_datetime_rfc3339() {
    let dt = Project::parse_datetime("2026-01-24T12:30:00Z".to_string());
    assert_eq!(dt.year(), 2026);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 24);
    assert_eq!(dt.hour(), 12);
}

#[test]
fn project_parse_datetime_sqlite_format() {
    let dt = Project::parse_datetime("2026-01-24 15:45:30".to_string());
    assert_eq!(dt.hour(), 15);
    assert_eq!(dt.minute(), 45);
}

// ===== Project from_row Integration Tests =====

use chrono::{Datelike, Timelike};
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        r#"CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            worktree_branch TEXT,
            base_branch TEXT,
            worktree_parent_directory TEXT,
            use_feature_branches INTEGER NOT NULL DEFAULT 1,
            merge_validation_mode TEXT NOT NULL DEFAULT 'off',
            merge_strategy TEXT NOT NULL DEFAULT 'rebase',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            archived_at TEXT NULL
        )"#,
        [],
    )
    .unwrap();
    conn
}

#[test]
fn project_from_row_local_mode_maps_to_worktree() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode,
           worktree_path, worktree_branch, base_branch, created_at, updated_at)
           VALUES ('proj-123', 'My Project', '/path/to/project', 'local',
           NULL, NULL, NULL, '2026-01-24T10:00:00Z', '2026-01-24T11:00:00Z')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row("SELECT * FROM projects WHERE id = 'proj-123'", [], |row| {
            Project::from_row(row)
        })
        .unwrap();

    assert_eq!(project.id.as_str(), "proj-123");
    assert_eq!(project.name, "My Project");
    assert_eq!(project.working_directory, "/path/to/project");
    // "local" in DB now maps to Worktree (backward compat)
    assert_eq!(project.git_mode, GitMode::Worktree);
    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
    assert!(project.base_branch.is_none());
    assert!(project.worktree_parent_directory.is_none());
}

#[test]
fn project_from_row_worktree_mode() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode,
           worktree_path, worktree_branch, base_branch, created_at, updated_at)
           VALUES ('proj-wt', 'Worktree Project', '/main/repo', 'worktree',
           '/worktrees/feature', 'feature-branch', 'main',
           '2026-01-24T08:00:00Z', '2026-01-24T09:00:00Z')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row("SELECT * FROM projects WHERE id = 'proj-wt'", [], |row| {
            Project::from_row(row)
        })
        .unwrap();

    assert_eq!(project.git_mode, GitMode::Worktree);
    assert_eq!(project.base_branch, Some("main".to_string()));
}

#[test]
fn project_from_row_unknown_git_mode_defaults_to_worktree() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode,
           worktree_path, worktree_branch, base_branch, created_at, updated_at)
           VALUES ('proj-unk', 'Unknown Mode', '/path', 'unknown_mode',
           NULL, NULL, NULL, '2026-01-24T08:00:00Z', '2026-01-24T08:00:00Z')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row("SELECT * FROM projects WHERE id = 'proj-unk'", [], |row| {
            Project::from_row(row)
        })
        .unwrap();

    // Unknown git mode should default to Worktree
    assert_eq!(project.git_mode, GitMode::Worktree);
}

#[test]
fn project_from_row_sqlite_datetime_format() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode,
           worktree_path, worktree_branch, base_branch, created_at, updated_at)
           VALUES ('proj-sql', 'SQL Datetime', '/path', 'worktree',
           NULL, NULL, NULL, '2026-01-24 12:00:00', '2026-01-24 14:30:00')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row("SELECT * FROM projects WHERE id = 'proj-sql'", [], |row| {
            Project::from_row(row)
        })
        .unwrap();

    assert_eq!(project.created_at.hour(), 12);
    assert_eq!(project.updated_at.hour(), 14);
    assert_eq!(project.updated_at.minute(), 30);
}

#[test]
fn project_from_row_with_null_base_branch() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode,
           worktree_path, worktree_branch, base_branch, created_at, updated_at)
           VALUES ('proj-nb', 'No Base', '/path', 'worktree',
           '/wt', 'branch', NULL, '2026-01-24T08:00:00Z', '2026-01-24T08:00:00Z')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row("SELECT * FROM projects WHERE id = 'proj-nb'", [], |row| {
            Project::from_row(row)
        })
        .unwrap();

    assert!(project.base_branch.is_none());
    assert_eq!(project.git_mode, GitMode::Worktree);
}

#[test]
fn project_from_row_invalid_merge_validation_mode_defaults_to_off() {
    let conn = setup_test_db();
    conn.execute(
        r#"INSERT INTO projects (id, name, working_directory, git_mode, merge_validation_mode,
           created_at, updated_at)
           VALUES ('proj-bad-mode', 'Bad Mode', '/path', 'worktree', 'strict',
           '2026-01-24T08:00:00Z', '2026-01-24T08:00:00Z')"#,
        [],
    )
    .unwrap();

    let project: Project = conn
        .query_row(
            "SELECT * FROM projects WHERE id = 'proj-bad-mode'",
            [],
            Project::from_row,
        )
        .unwrap();

    assert_eq!(project.merge_validation_mode, MergeValidationMode::Off);
}

// ===== MergeStrategy Tests =====

#[test]
fn merge_strategy_default_is_rebase_squash() {
    assert_eq!(MergeStrategy::default(), MergeStrategy::RebaseSquash);
}

#[test]
fn merge_strategy_serializes() {
    assert_eq!(
        serde_json::to_string(&MergeStrategy::Rebase).unwrap(),
        "\"rebase\""
    );
    assert_eq!(
        serde_json::to_string(&MergeStrategy::Merge).unwrap(),
        "\"merge\""
    );
    assert_eq!(
        serde_json::to_string(&MergeStrategy::Squash).unwrap(),
        "\"squash\""
    );
    assert_eq!(
        serde_json::to_string(&MergeStrategy::RebaseSquash).unwrap(),
        "\"rebase_squash\""
    );
}

#[test]
fn merge_strategy_deserializes() {
    let rebase: MergeStrategy = serde_json::from_str("\"rebase\"").unwrap();
    let merge: MergeStrategy = serde_json::from_str("\"merge\"").unwrap();
    let squash: MergeStrategy = serde_json::from_str("\"squash\"").unwrap();
    let rebase_squash: MergeStrategy = serde_json::from_str("\"rebase_squash\"").unwrap();
    assert_eq!(rebase, MergeStrategy::Rebase);
    assert_eq!(merge, MergeStrategy::Merge);
    assert_eq!(squash, MergeStrategy::Squash);
    assert_eq!(rebase_squash, MergeStrategy::RebaseSquash);
}

#[test]
fn merge_strategy_from_str() {
    assert_eq!(
        "rebase".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::Rebase
    );
    assert_eq!(
        "merge".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::Merge
    );
    assert_eq!(
        "squash".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::Squash
    );
    assert_eq!(
        "rebase_squash".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::RebaseSquash
    );
    assert!("invalid".parse::<MergeStrategy>().is_err());
}

#[test]
fn merge_strategy_display() {
    assert_eq!(format!("{}", MergeStrategy::Rebase), "rebase");
    assert_eq!(format!("{}", MergeStrategy::Merge), "merge");
    assert_eq!(format!("{}", MergeStrategy::Squash), "squash");
    assert_eq!(format!("{}", MergeStrategy::RebaseSquash), "rebase_squash");
}
