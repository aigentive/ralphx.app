// Migration v25: Update default max_concurrent_tasks from 2 to 10
//
// SQLite doesn't support ALTER COLUMN DEFAULT, so we recreate the table.
// Existing rows keep their current values; only the DEFAULT constraint changes.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE execution_settings_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            max_concurrent_tasks INTEGER NOT NULL DEFAULT 10,
            auto_commit INTEGER NOT NULL DEFAULT 1,
            pause_on_failure INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL,
            project_id TEXT DEFAULT NULL
        );
        INSERT INTO execution_settings_new (id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
            SELECT id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id
            FROM execution_settings;
        DROP TABLE execution_settings;
        ALTER TABLE execution_settings_new RENAME TO execution_settings;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_execution_settings_project_id
            ON execution_settings(project_id);
        UPDATE execution_settings SET max_concurrent_tasks = 10
            WHERE max_concurrent_tasks = 2;",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
