// Migration v47: Add execution_plan_id to plan_branches
//
// Adds execution_plan_id FK to plan_branches for linking branches to execution attempts.
// Uses unique index to ensure one plan branch per execution plan.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v47: Add execution_plan_id to plan_branches table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Add execution_plan_id column
    helpers::add_column_if_not_exists(
        conn,
        "plan_branches",
        "execution_plan_id",
        "TEXT REFERENCES execution_plans(id)",
    )?;

    // Create unique index on execution_plan_id (NULLs allowed - for backfill)
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_plan_branches_execution_plan
         ON plan_branches(execution_plan_id) WHERE execution_plan_id IS NOT NULL;",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
