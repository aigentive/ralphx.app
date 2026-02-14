// Migration v35: Add parent_step_id and scope_context columns to task_steps
//
// This migration enables sub-steps (hierarchical step tracking) by adding:
// - parent_step_id: Links a sub-step to its parent step
// - scope_context: JSON containing STRICT SCOPE for sub-steps (file list, boundaries, instructions)

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v35: Add sub-step support to task_steps
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "task_steps", "parent_step_id", "TEXT")?;
    helpers::add_column_if_not_exists(conn, "task_steps", "scope_context", "TEXT")?;
    Ok(())
}
