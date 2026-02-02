// Migration v10: Add execution_settings table
//
// This migration adds the execution_settings table for persistent storage of
// execution settings (max concurrent tasks, auto-commit, pause on failure).
// Uses singleton pattern with id=1 constraint (following ideation_settings pattern).

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v10: Add execution_settings table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Create the execution_settings table (singleton pattern with id=1)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS execution_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            max_concurrent_tasks INTEGER NOT NULL DEFAULT 2,
            auto_commit INTEGER NOT NULL DEFAULT 1,
            pause_on_failure INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Insert default row if it doesn't exist
    conn.execute(
        "INSERT OR IGNORE INTO execution_settings (id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at)
         VALUES (1, 2, 1, 1, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
