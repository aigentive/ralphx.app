// Migration v14: App state singleton table
//
// Creates app_state singleton table with CHECK(id=1) constraint
// to persist active_project_id across app restarts.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v14: Create app_state singleton table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            active_project_id TEXT DEFAULT NULL,
            updated_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO app_state (id, active_project_id, updated_at)
        VALUES (1, NULL, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'));",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
