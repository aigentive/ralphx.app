// Migration v15: Add ideation_session_id column to tasks
//
// Adds a direct link from tasks to their originating ideation session.
// Backfills existing tasks from task_proposals join.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v15: Add ideation_session_id to tasks and backfill from proposals
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "tasks", "ideation_session_id", "TEXT DEFAULT NULL")?;

    // Backfill from proposals: task → proposal.created_task_id → proposal.session_id
    conn.execute(
        "UPDATE tasks SET ideation_session_id = (
            SELECT tp.session_id FROM task_proposals tp WHERE tp.created_task_id = tasks.id LIMIT 1
        ) WHERE ideation_session_id IS NULL
          AND EXISTS (SELECT 1 FROM task_proposals tp WHERE tp.created_task_id = tasks.id)",
        [],
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
