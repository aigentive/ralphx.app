// Migration v45: Drop the dead task_blockers table
//
// All blocker/dependency logic now uses the task_dependencies table exclusively.
// The task_blockers table was created in v1 but has been unused since
// task_dependencies was introduced.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS task_blockers;
         DROP INDEX IF EXISTS idx_task_blockers_task_id;
         DROP INDEX IF EXISTS idx_task_blockers_blocker_id;",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
