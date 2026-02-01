// Project entity - represents a development project in RalphX
// Contains project configuration, git settings, and metadata

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::ProjectId;

/// Git workflow mode for a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMode {
    /// Work directly in the local repository
    Local,
    /// Use git worktrees for isolated development
    Worktree,
}

impl Default for GitMode {
    fn default() -> Self {
        Self::Local
    }
}

impl std::fmt::Display for GitMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitMode::Local => write!(f, "local"),
            GitMode::Worktree => write!(f, "worktree"),
        }
    }
}

/// Error type for parsing GitMode from a string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseGitModeError {
    pub value: String,
}

impl std::fmt::Display for ParseGitModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown git mode: '{}'", self.value)
    }
}

impl std::error::Error for ParseGitModeError {}

impl FromStr for GitMode {
    type Err = ParseGitModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(GitMode::Local),
            "worktree" => Ok(GitMode::Worktree),
            _ => Err(ParseGitModeError {
                value: s.to_string(),
            }),
        }
    }
}

/// A development project managed by RalphX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier for this project
    pub id: ProjectId,
    /// Human-readable project name
    pub name: String,
    /// Absolute path to the project's working directory
    pub working_directory: String,
    /// Git workflow mode (local or worktree)
    pub git_mode: GitMode,
    /// Path to worktree (only if git_mode is Worktree)
    pub worktree_path: Option<String>,
    /// Branch name for worktree (only if git_mode is Worktree)
    pub worktree_branch: Option<String>,
    /// Base branch for comparisons (e.g., "main" or "master")
    pub base_branch: Option<String>,
    /// Parent directory for task worktrees (default: ~/ralphx-worktrees)
    pub worktree_parent_directory: Option<String>,
    /// When the project was created
    pub created_at: DateTime<Utc>,
    /// When the project was last updated
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Creates a new project with the given name and working directory
    /// Uses sensible defaults for git mode (Local) and timestamps (now)
    pub fn new(name: String, working_directory: String) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name,
            working_directory,
            git_mode: GitMode::default(),
            worktree_path: None,
            worktree_branch: None,
            base_branch: None,
            worktree_parent_directory: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new project configured for worktree mode
    pub fn new_with_worktree(
        name: String,
        working_directory: String,
        worktree_path: String,
        worktree_branch: String,
        base_branch: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name,
            working_directory,
            git_mode: GitMode::Worktree,
            worktree_path: Some(worktree_path),
            worktree_branch: Some(worktree_branch),
            base_branch,
            worktree_parent_directory: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns true if the project uses worktree mode
    pub fn is_worktree(&self) -> bool {
        self.git_mode == GitMode::Worktree
    }

    /// Updates the updated_at timestamp to now
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Deserialize a Project from a SQLite row.
    /// Expects columns: id, name, working_directory, git_mode,
    /// worktree_path, worktree_branch, base_branch, worktree_parent_directory, created_at, updated_at
    pub fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: ProjectId::from_string(row.get("id")?),
            name: row.get("name")?,
            working_directory: row.get("working_directory")?,
            git_mode: row
                .get::<_, String>("git_mode")?
                .parse()
                .unwrap_or(GitMode::Local),
            worktree_path: row.get("worktree_path")?,
            worktree_branch: row.get("worktree_branch")?,
            base_branch: row.get("base_branch")?,
            worktree_parent_directory: row.get("worktree_parent_directory")?,
            created_at: Self::parse_datetime(row.get("created_at")?),
            updated_at: Self::parse_datetime(row.get("updated_at")?),
        })
    }

    /// Parse a datetime string from SQLite into a DateTime<Utc>
    /// Handles both RFC3339 format and SQLite's CURRENT_TIMESTAMP format
    fn parse_datetime(s: String) -> DateTime<Utc> {
        // Try RFC3339 first (our preferred format)
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
        // Try SQLite's default datetime format (YYYY-MM-DD HH:MM:SS)
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
            return Utc.from_utc_datetime(&dt);
        }
        // Fallback to now if parsing fails
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== GitMode Tests =====

    #[test]
    fn git_mode_default_is_local() {
        assert_eq!(GitMode::default(), GitMode::Local);
    }

    #[test]
    fn git_mode_display_local() {
        assert_eq!(format!("{}", GitMode::Local), "local");
    }

    #[test]
    fn git_mode_display_worktree() {
        assert_eq!(format!("{}", GitMode::Worktree), "worktree");
    }

    #[test]
    fn git_mode_serializes_to_snake_case() {
        let local_json = serde_json::to_string(&GitMode::Local).expect("Should serialize");
        let worktree_json = serde_json::to_string(&GitMode::Worktree).expect("Should serialize");

        assert_eq!(local_json, "\"local\"");
        assert_eq!(worktree_json, "\"worktree\"");
    }

    #[test]
    fn git_mode_deserializes_from_snake_case() {
        let local: GitMode = serde_json::from_str("\"local\"").expect("Should deserialize");
        let worktree: GitMode = serde_json::from_str("\"worktree\"").expect("Should deserialize");

        assert_eq!(local, GitMode::Local);
        assert_eq!(worktree, GitMode::Worktree);
    }

    #[test]
    fn git_mode_clone_works() {
        let mode = GitMode::Worktree;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    #[test]
    fn git_mode_equality_works() {
        assert_eq!(GitMode::Local, GitMode::Local);
        assert_eq!(GitMode::Worktree, GitMode::Worktree);
        assert_ne!(GitMode::Local, GitMode::Worktree);
    }

    // ===== Project Creation Tests =====

    #[test]
    fn project_new_creates_with_defaults() {
        let project = Project::new("My Project".to_string(), "/path/to/project".to_string());

        assert_eq!(project.name, "My Project");
        assert_eq!(project.working_directory, "/path/to/project");
        assert_eq!(project.git_mode, GitMode::Local);
        assert!(project.worktree_path.is_none());
        assert!(project.worktree_branch.is_none());
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
    fn project_new_with_worktree_sets_all_fields() {
        let project = Project::new_with_worktree(
            "Worktree Project".to_string(),
            "/main/repo".to_string(),
            "/worktrees/feature".to_string(),
            "feature-branch".to_string(),
            Some("main".to_string()),
        );

        assert_eq!(project.name, "Worktree Project");
        assert_eq!(project.working_directory, "/main/repo");
        assert_eq!(project.git_mode, GitMode::Worktree);
        assert_eq!(project.worktree_path, Some("/worktrees/feature".to_string()));
        assert_eq!(project.worktree_branch, Some("feature-branch".to_string()));
        assert_eq!(project.base_branch, Some("main".to_string()));
        assert!(project.worktree_parent_directory.is_none());
    }

    #[test]
    fn project_new_with_worktree_no_base_branch() {
        let project = Project::new_with_worktree(
            "No Base".to_string(),
            "/repo".to_string(),
            "/worktree".to_string(),
            "branch".to_string(),
            None,
        );

        assert!(project.base_branch.is_none());
    }

    // ===== Project Method Tests =====

    #[test]
    fn project_is_worktree_returns_true_for_worktree_mode() {
        let project = Project::new_with_worktree(
            "WT".to_string(),
            "/repo".to_string(),
            "/wt".to_string(),
            "branch".to_string(),
            None,
        );

        assert!(project.is_worktree());
    }

    #[test]
    fn project_is_worktree_returns_false_for_local_mode() {
        let project = Project::new("Local".to_string(), "/repo".to_string());

        assert!(!project.is_worktree());
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
        assert!(json.contains("\"git_mode\":\"local\""));
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
        assert_eq!(project.worktree_path, Some("/wt/path".to_string()));
        assert_eq!(project.worktree_branch, Some("feature".to_string()));
        assert_eq!(project.base_branch, Some("main".to_string()));
    }

    #[test]
    fn project_deserializes_with_null_optionals() {
        let json = r#"{
            "id": "test-id",
            "name": "Minimal",
            "working_directory": "/path",
            "git_mode": "local",
            "worktree_path": null,
            "worktree_branch": null,
            "base_branch": null,
            "created_at": "2025-01-24T12:00:00Z",
            "updated_at": "2025-01-24T12:00:00Z"
        }"#;

        let project: Project = serde_json::from_str(json).expect("Should deserialize");

        assert!(project.worktree_path.is_none());
        assert!(project.worktree_branch.is_none());
        assert!(project.base_branch.is_none());
    }

    #[test]
    fn project_roundtrip_serialization() {
        let original = Project::new_with_worktree(
            "Roundtrip".to_string(),
            "/original".to_string(),
            "/wt".to_string(),
            "branch".to_string(),
            Some("main".to_string()),
        );

        let json = serde_json::to_string(&original).expect("Should serialize");
        let restored: Project = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(original.id, restored.id);
        assert_eq!(original.name, restored.name);
        assert_eq!(original.working_directory, restored.working_directory);
        assert_eq!(original.git_mode, restored.git_mode);
        assert_eq!(original.worktree_path, restored.worktree_path);
        assert_eq!(original.worktree_branch, restored.worktree_branch);
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

    // ===== GitMode FromStr Tests =====

    #[test]
    fn git_mode_from_str_local() {
        let mode: GitMode = "local".parse().unwrap();
        assert_eq!(mode, GitMode::Local);
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

    use rusqlite::Connection;
    use chrono::{Datelike, Timelike};

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
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#,
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn project_from_row_local_mode() {
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
        assert_eq!(project.git_mode, GitMode::Local);
        assert!(project.worktree_path.is_none());
        assert!(project.worktree_branch.is_none());
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
        assert_eq!(project.worktree_path, Some("/worktrees/feature".to_string()));
        assert_eq!(project.worktree_branch, Some("feature-branch".to_string()));
        assert_eq!(project.base_branch, Some("main".to_string()));
    }

    #[test]
    fn project_from_row_unknown_git_mode_defaults_to_local() {
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

        // Unknown git mode should default to Local
        assert_eq!(project.git_mode, GitMode::Local);
    }

    #[test]
    fn project_from_row_sqlite_datetime_format() {
        let conn = setup_test_db();
        conn.execute(
            r#"INSERT INTO projects (id, name, working_directory, git_mode,
               worktree_path, worktree_branch, base_branch, created_at, updated_at)
               VALUES ('proj-sql', 'SQL Datetime', '/path', 'local',
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
        assert_eq!(project.worktree_path, Some("/wt".to_string()));
    }
}
