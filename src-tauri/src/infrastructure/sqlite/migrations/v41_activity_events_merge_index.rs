// Migration v40: Add composite index on activity_events for efficient merge audit queries
//
// Adds a composite index (task_id, internal_status, created_at DESC) to support
// efficient per-task-status queries such as finding the latest activity event for
// a task in a specific status (e.g. `SELECT MAX(created_at) ... WHERE task_id=? AND internal_status=?`).
//
// This enables the activity-based timeout query used by the reconciler to determine
// whether a merge agent is still active.

use crate::error::AppResult;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_task_status_time
         ON activity_events(task_id, internal_status, created_at DESC);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
