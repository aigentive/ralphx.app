// Migration v46: Execution plans
//
// Adds execution_plans table for tracking implementation attempts.
// Each re-accept of a plan creates a new ExecutionPlan with unique ID,
// enabling unique branch names and clean separation of execution attempts.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v46: Create execution_plans table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS execution_plans (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES ideation_sessions(id),
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
