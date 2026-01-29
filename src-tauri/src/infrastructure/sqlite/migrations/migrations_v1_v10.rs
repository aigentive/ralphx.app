// Database migrations v1-v10
// Creates initial schema and early updates

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v1: Create core tables (projects, tasks, task_state_history)
pub(super) fn migrate_v1(conn: &Connection) -> AppResult<()> {
    // Projects table
    conn.execute(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            git_integration_enabled INTEGER NOT NULL DEFAULT 1,
            git_author_name TEXT,
            git_author_email TEXT,
            model_name TEXT,
            model_provider TEXT,
            instructions TEXT,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            updated_at DATETIME NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Tasks table
    conn.execute(
        "CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            updated_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Task state history table
    conn.execute(
        "CREATE TABLE task_state_history (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            from_status TEXT,
            to_status TEXT NOT NULL,
            transitioned_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create indexes
    conn.execute(
        "CREATE INDEX idx_tasks_project_id ON tasks(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_task_state_history_task_id ON task_state_history(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v2: Add priority field to tasks
pub(super) fn migrate_v2(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN priority INTEGER NOT NULL DEFAULT 1",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on priority for sorting
    conn.execute("CREATE INDEX idx_tasks_priority ON tasks(priority)", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v3: Add git_branch field to tasks
pub(super) fn migrate_v3(conn: &Connection) -> AppResult<()> {
    conn.execute("ALTER TABLE tasks ADD COLUMN git_branch TEXT", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v4: Add verification tables
pub(super) fn migrate_v4(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE task_verifications (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            build_passed INTEGER NOT NULL DEFAULT 0,
            tests_passed INTEGER NOT NULL DEFAULT 0,
            lints_passed INTEGER NOT NULL DEFAULT 0,
            verified_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_task_verifications_task_id ON task_verifications(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v5: Add artifacts and artifact flows
pub(super) fn migrate_v5(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE artifacts (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE TABLE artifact_flows (
            id TEXT PRIMARY KEY,
            from_artifact_id TEXT NOT NULL,
            to_artifact_id TEXT NOT NULL,
            flow_type TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (from_artifact_id) REFERENCES artifacts (id) ON DELETE CASCADE,
            FOREIGN KEY (to_artifact_id) REFERENCES artifacts (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_artifacts_task_id ON artifacts(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_artifact_flows_from ON artifact_flows(from_artifact_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_artifact_flows_to ON artifact_flows(to_artifact_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v6: Add methodology tables
pub(super) fn migrate_v6(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE methodologies (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            created_at DATETIME NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v7: Add research tables
pub(super) fn migrate_v7(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE research_sessions (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            query TEXT NOT NULL,
            findings TEXT,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE TABLE research_sources (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url TEXT,
            content TEXT,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (session_id) REFERENCES research_sessions (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_research_sessions_task_id ON research_sessions(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_research_sources_session_id ON research_sources(session_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v8: Add dependencies table
pub(super) fn migrate_v8(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE task_dependencies (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            depends_on_task_id TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE,
            FOREIGN KEY (depends_on_task_id) REFERENCES tasks (id) ON DELETE CASCADE
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_task_dependencies_task_id ON task_dependencies(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v9: Add internal_status to tasks
pub(super) fn migrate_v9(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN internal_status TEXT NOT NULL DEFAULT 'todo'",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v10: Add notes to tasks
pub(super) fn migrate_v10(conn: &Connection) -> AppResult<()> {
    conn.execute("ALTER TABLE tasks ADD COLUMN notes TEXT", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
