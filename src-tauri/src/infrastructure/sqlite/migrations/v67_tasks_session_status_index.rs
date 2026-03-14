// Migration v67: Add composite index on tasks(ideation_session_id, internal_status)
//
// Required by group count and paginated session listing queries to avoid full
// table scans at 10k+ sessions.

use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_tasks_session_status
         ON tasks(ideation_session_id, internal_status);",
    )?;

    tracing::info!("v67: added composite index idx_tasks_session_status on tasks(ideation_session_id, internal_status)");

    Ok(())
}
