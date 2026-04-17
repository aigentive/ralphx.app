use crate::error::{AppError, AppResult};
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS delegated_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            parent_context_type TEXT NOT NULL,
            parent_context_id TEXT NOT NULL,
            parent_turn_id TEXT,
            parent_message_id TEXT,
            agent_name TEXT NOT NULL,
            title TEXT,
            harness TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'running',
            provider_session_id TEXT,
            error TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            completed_at DATETIME
        );

        CREATE INDEX IF NOT EXISTS idx_delegated_sessions_parent_context
            ON delegated_sessions(parent_context_type, parent_context_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_delegated_sessions_project
            ON delegated_sessions(project_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_delegated_sessions_status
            ON delegated_sessions(status, updated_at DESC);",
    )
    .map_err(|error| AppError::Database(error.to_string()))?;

    Ok(())
}
