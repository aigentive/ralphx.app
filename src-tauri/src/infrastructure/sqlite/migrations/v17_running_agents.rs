// Migration v17: Add running_agents table
//
// Persists agent PIDs to SQLite so orphaned processes
// can be killed on app restart.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v17: Create running_agents table for PID persistence
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS running_agents (
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL,
            pid INTEGER NOT NULL,
            conversation_id TEXT NOT NULL,
            agent_run_id TEXT NOT NULL,
            started_at TEXT NOT NULL,
            PRIMARY KEY (context_type, context_id)
        );",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
