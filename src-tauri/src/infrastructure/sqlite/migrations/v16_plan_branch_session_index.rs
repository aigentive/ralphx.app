// Migration v16: Add UNIQUE index on plan_branches.session_id
//
// Enables efficient get_by_session_id() lookup and enforces
// 1:1 mapping between ideation sessions and plan branches.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v16: Add UNIQUE index on plan_branches.session_id
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_plan_branches_session_id
         ON plan_branches(session_id);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
