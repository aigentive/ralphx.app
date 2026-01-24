// Database migrations for SQLite
// Creates and updates schema as needed

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Current schema version
pub const SCHEMA_VERSION: i32 = 5;

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    // Create migrations table if it doesn't exist
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_schema_version(conn)?;

    // Run migrations sequentially
    if current_version < 1 {
        migrate_v1(conn)?;
        set_schema_version(conn, 1)?;
    }

    if current_version < 2 {
        migrate_v2(conn)?;
        set_schema_version(conn, 2)?;
    }

    if current_version < 3 {
        migrate_v3(conn)?;
        set_schema_version(conn, 3)?;
    }

    if current_version < 4 {
        migrate_v4(conn)?;
        set_schema_version(conn, 4)?;
    }

    if current_version < 5 {
        migrate_v5(conn)?;
        set_schema_version(conn, 5)?;
    }

    Ok(())
}

/// Create the migrations tracking table
fn create_migrations_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Get the current schema version
fn get_schema_version(conn: &Connection) -> AppResult<i32> {
    let result: Result<i32, _> = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    );

    result.map_err(|e| AppError::Database(e.to_string()))
}

/// Set the schema version after a migration
fn set_schema_version(conn: &Connection, version: i32) -> AppResult<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version) VALUES (?1)",
        [version],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Migration v1: Create core tables (projects, tasks, task_state_history)
fn migrate_v1(conn: &Connection) -> AppResult<()> {
    // Projects table
    conn.execute(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            worktree_branch TEXT,
            base_branch TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Tasks table
    conn.execute(
        "CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id),
            category TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            priority INTEGER DEFAULT 0,
            internal_status TEXT NOT NULL DEFAULT 'backlog',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            started_at DATETIME,
            completed_at DATETIME
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on project_id for faster lookups
    conn.execute(
        "CREATE INDEX idx_tasks_project_id ON tasks(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on internal_status for filtering
    conn.execute(
        "CREATE INDEX idx_tasks_internal_status ON tasks(internal_status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Task state history table (audit log)
    conn.execute(
        "CREATE TABLE task_state_history (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            from_status TEXT,
            to_status TEXT NOT NULL,
            changed_by TEXT NOT NULL,
            reason TEXT,
            metadata JSON,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on task_id for history lookups
    conn.execute(
        "CREATE INDEX idx_task_state_history_task_id ON task_state_history(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v2: Create task_blockers table for dependency tracking
fn migrate_v2(conn: &Connection) -> AppResult<()> {
    // Task blockers table (many-to-many relationship)
    // task_id is blocked BY blocker_id
    conn.execute(
        "CREATE TABLE task_blockers (
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            blocker_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (task_id, blocker_id)
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for "what blocks this task?" queries
    conn.execute(
        "CREATE INDEX idx_task_blockers_task_id ON task_blockers(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on blocker_id for "what does this task block?" queries
    conn.execute(
        "CREATE INDEX idx_task_blockers_blocker_id ON task_blockers(blocker_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v3: Create task_state_data table for state-local data persistence
///
/// States like QaFailed and Failed have associated data that needs to persist
/// across application restarts. This table stores that data as JSON.
fn migrate_v3(conn: &Connection) -> AppResult<()> {
    // Task state data table
    // Stores state-local data for states like qa_failed and failed
    conn.execute(
        "CREATE TABLE task_state_data (
            task_id TEXT PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
            state_type TEXT NOT NULL,
            data TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on state_type for querying all tasks in a specific state with data
    conn.execute(
        "CREATE INDEX idx_task_state_data_state_type ON task_state_data(state_type)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v4: Create agent_profiles table for storing agent configurations
///
/// Agent profiles define how agents behave - their model, execution settings,
/// skills, and behavioral flags. Built-in profiles are seeded on first run.
fn migrate_v4(conn: &Connection) -> AppResult<()> {
    // Agent profiles table
    conn.execute(
        "CREATE TABLE agent_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL,
            profile_json TEXT NOT NULL,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on role for filtering profiles by role
    conn.execute(
        "CREATE INDEX idx_agent_profiles_role ON agent_profiles(role)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_builtin for filtering built-in vs custom profiles
    conn.execute(
        "CREATE INDEX idx_agent_profiles_is_builtin ON agent_profiles(is_builtin)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v5: Create task_qa table for QA artifacts
///
/// The task_qa table stores QA-related data for each task:
/// - Acceptance criteria generated by QA Prep agent
/// - Test steps (initial and refined)
/// - Test results and screenshots from QA Executor
fn migrate_v5(conn: &Connection) -> AppResult<()> {
    // Task QA table
    conn.execute(
        "CREATE TABLE task_qa (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,

            -- Phase 1: QA Prep (runs in parallel with execution)
            acceptance_criteria TEXT,
            qa_test_steps TEXT,
            prep_agent_id TEXT,
            prep_started_at DATETIME,
            prep_completed_at DATETIME,

            -- Phase 2: QA Refinement (after execution completes)
            actual_implementation TEXT,
            refined_test_steps TEXT,
            refinement_agent_id TEXT,
            refinement_completed_at DATETIME,

            -- Phase 3: Test Execution (browser tests)
            test_results TEXT,
            screenshots TEXT,
            test_agent_id TEXT,
            test_completed_at DATETIME,

            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for efficient lookup
    conn.execute(
        "CREATE INDEX idx_task_qa_task_id ON task_qa(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::connection::open_memory_connection;

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(SCHEMA_VERSION, 5);
    }

    #[test]
    fn test_run_migrations_creates_migrations_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify migrations table exists
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
    fn test_run_migrations_creates_projects_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify projects table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_migrations_creates_tasks_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify tasks table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_migrations_creates_task_state_history_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_state_history table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_state_history'",
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
        assert_eq!(version, 5);
    }

    #[test]
    fn test_run_migrations_is_idempotent() {
        let conn = open_memory_connection().unwrap();

        // Run migrations twice
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Should still work and have correct version
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn test_projects_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Try inserting a complete project record
        let result = conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch)
             VALUES ('test-id', 'Test Project', '/path/to/project', 'local', NULL, NULL, NULL)",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project first (foreign key reference)
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Try inserting a complete task record
        let result = conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'Description', 5, 'backlog')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_state_history_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test')",
            [],
        )
        .unwrap();

        // Try inserting a history record
        let result = conn.execute(
            "INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by, reason, metadata)
             VALUES ('hist-1', 'task-1', 'backlog', 'ready', 'system', 'Started', '{}')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_index_on_project_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_tasks_index_on_internal_status_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_internal_status'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_history_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_state_history_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_projects_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert minimal project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Check default values
        let git_mode: String = conn
            .query_row(
                "SELECT git_mode FROM projects WHERE id = 'proj-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(git_mode, "local");
    }

    #[test]
    fn test_tasks_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Insert minimal task
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test')",
            [],
        )
        .unwrap();

        // Check default values
        let (priority, status): (i32, String) = conn
            .query_row(
                "SELECT priority, internal_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(priority, 0);
        assert_eq!(status, "backlog");
    }

    #[test]
    fn test_run_migrations_creates_task_blockers_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_blockers table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_blockers'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Try inserting a blocker relationship
        let result = conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_blockers_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_blockers_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_index_on_blocker_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_blockers_blocker_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_primary_key_prevents_duplicates() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // First insert should succeed
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Duplicate should fail
        let result = conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_blockers_cascade_delete_on_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Add blocker relationship
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Delete the blocked task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // Blocker relationship should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_blockers_cascade_delete_on_blocker() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Add blocker relationship (task-1 is blocked by task-2)
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Delete the blocker task
        conn.execute("DELETE FROM tasks WHERE id = 'task-2'", []).unwrap();

        // Blocker relationship should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE blocker_id = 'task-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_blockers_multiple_blockers_per_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-3', 'proj-1', 'feature', 'Task 3')",
            [],
        )
        .unwrap();

        // Task 1 is blocked by both task 2 and task 3
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-3')",
            [],
        )
        .unwrap();

        // Count blockers for task-1
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    // ==================
    // V3 Migration Tests: task_state_data table
    // ==================

    #[test]
    fn test_run_migrations_creates_task_state_data_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_state_data table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_state_data'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_data_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1', 'qa_failed')",
            [],
        )
        .unwrap();

        // Try inserting a state data record
        let result = conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data)
             VALUES ('task-1', 'qa_failed', '{\"failures\": [], \"retry_count\": 0}')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_state_data_index_on_state_type_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_state_data_state_type'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_data_primary_key_prevents_duplicates() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // First insert should succeed
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // Duplicate should fail (primary key violation)
        let result = conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'failed', '{}')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_state_data_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert state data
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // State data should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_state_data_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert JSON data
        let json_data = r#"{"failures":[{"test_name":"test_foo","error":"assertion failed"}],"retry_count":2}"#;
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT data FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_task_state_data_can_update() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert initial data
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{\"retry_count\":0}')",
            [],
        )
        .unwrap();

        // Update the data using REPLACE
        conn.execute(
            "INSERT OR REPLACE INTO task_state_data (task_id, state_type, data)
             VALUES ('task-1', 'qa_failed', '{\"retry_count\":1}')",
            [],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT data FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(retrieved.contains("\"retry_count\":1"));
    }

    #[test]
    fn test_task_state_data_updated_at_has_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert without specifying updated_at
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // updated_at should not be null
        let updated_at: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(updated_at.is_some());
    }

    // ==================
    // V4 Migration Tests: agent_profiles table
    // ==================

    #[test]
    fn test_run_migrations_creates_agent_profiles_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify agent_profiles table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_profiles'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Try inserting a complete agent profile record
        let profile_json = r#"{"claude_code":{"agent_definition":"agents/worker.md"}}"#;
        let result = conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin)
             VALUES ('prof-1', 'Worker', 'worker', ?1, 1)",
            [profile_json],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_profiles_name_unique_constraint() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let profile_json = r#"{}"#;

        // First insert should succeed
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', ?1)",
            [profile_json],
        )
        .unwrap();

        // Duplicate name should fail
        let result = conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-2', 'Worker', 'worker', ?1)",
            [profile_json],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_agent_profiles_index_on_role_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_agent_profiles_role'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_index_on_is_builtin_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_agent_profiles_is_builtin'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert minimal profile
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Check default values
        let is_builtin: i32 = conn
            .query_row(
                "SELECT is_builtin FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(is_builtin, 0);
    }

    #[test]
    fn test_agent_profiles_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert JSON profile
        let json_data = r#"{"name":"worker","execution":{"model":"sonnet","max_iterations":30},"behavior":{"autonomy_level":"semi_autonomous"}}"#;
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT profile_json FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_agent_profiles_filter_by_role() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert profiles with different roles
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-2', 'Reviewer', 'reviewer', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-3', 'Another Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Query by role
        let worker_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE role = 'worker'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(worker_count, 2);
    }

    #[test]
    fn test_agent_profiles_filter_by_is_builtin() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert builtin and custom profiles
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin) VALUES ('prof-1', 'Builtin Worker', 'worker', '{}', 1)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin) VALUES ('prof-2', 'Custom Worker', 'worker', '{}', 0)",
            [],
        )
        .unwrap();

        // Query builtin profiles
        let builtin_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE is_builtin = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(builtin_count, 1);

        // Query custom profiles
        let custom_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE is_builtin = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(custom_count, 1);
    }

    #[test]
    fn test_agent_profiles_timestamps() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert profile
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Check timestamps exist
        let (created_at, updated_at): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT created_at, updated_at FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(created_at.is_some());
        assert!(updated_at.is_some());
    }

    // ==================
    // V5 Migration Tests: task_qa table
    // ==================

    #[test]
    fn test_run_migrations_creates_task_qa_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_qa table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_qa'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_qa_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Try inserting a complete task_qa record
        let acceptance_criteria = r#"[{"id":"AC1","description":"Test","testable":true,"type":"visual"}]"#;
        let qa_test_steps = r#"[{"id":"QA1","criteria_id":"AC1","description":"Test","commands":[],"expected":"Pass"}]"#;
        let test_results = r#"{"task_id":"task-1","overall_status":"passed","steps":[]}"#;
        let screenshots = r#"["screenshots/test.png"]"#;

        let result = conn.execute(
            "INSERT INTO task_qa (
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at
            ) VALUES (
                'qa-1', 'task-1',
                ?1, ?2, 'agent-prep-1', '2026-01-24 10:00:00', '2026-01-24 10:05:00',
                'Implemented feature X', ?2, 'agent-refine-1', '2026-01-24 10:10:00',
                ?3, ?4, 'agent-test-1', '2026-01-24 10:15:00'
            )",
            [acceptance_criteria, qa_test_steps, test_results, screenshots],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_qa_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_qa_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_qa_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert task_qa record
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // task_qa should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_qa WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_qa_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert JSON data
        let json_data = r#"[{"id":"AC1","description":"User can see task board","testable":true,"type":"visual"}]"#;
        conn.execute(
            "INSERT INTO task_qa (id, task_id, acceptance_criteria) VALUES ('qa-1', 'task-1', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT acceptance_criteria FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_task_qa_allows_null_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert minimal task_qa (only required columns)
        let result = conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        );

        assert!(result.is_ok());

        // Verify nulls are stored
        let acceptance: Option<String> = conn
            .query_row(
                "SELECT acceptance_criteria FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(acceptance.is_none());
    }

    #[test]
    fn test_task_qa_created_at_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert without created_at
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // created_at should not be null
        let created_at: Option<String> = conn
            .query_row(
                "SELECT created_at FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(created_at.is_some());
    }

    #[test]
    fn test_task_qa_multiple_per_task_prevented() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert first QA record (unique ID)
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // Second QA record for same task but different ID should work
        // (no unique constraint on task_id, just foreign key)
        let result = conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-2', 'task-1')",
            [],
        );

        assert!(result.is_ok());
    }
}
