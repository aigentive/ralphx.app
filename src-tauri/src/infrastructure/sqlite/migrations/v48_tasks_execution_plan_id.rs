// Migration v48: Add execution_plan_id to tasks
//
// Adds execution_plan_id FK to tasks for linking tasks to execution attempts.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v48: Add execution_plan_id to tasks table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "tasks",
        "execution_plan_id",
        "TEXT REFERENCES execution_plans(id)",
    )?;

    // Create index for querying tasks by execution plan
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_tasks_execution_plan
         ON tasks(execution_plan_id);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
