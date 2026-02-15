// Migration v36: Add spawn_orchestrator_jobs table
//
// This migration creates a table to track orchestrator spawn jobs for
// background task execution. Jobs track the spawning of orchestrator
// sessions for task ideation and planning.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v36: Create spawn_orchestrator_jobs table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS spawn_orchestrator_jobs (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            description TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'done', 'failed')),
            error_message TEXT,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            started_at TEXT,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_spawn_orchestrator_jobs_status ON spawn_orchestrator_jobs(status);
        CREATE INDEX IF NOT EXISTS idx_spawn_orchestrator_jobs_session ON spawn_orchestrator_jobs(session_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
