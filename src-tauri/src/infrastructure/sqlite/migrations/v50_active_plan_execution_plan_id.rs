// Migration v50: Add execution_plan_id to project_active_plan
//
// Adds execution_plan_id column to project_active_plan for direct linking
// between active plan state and the execution plan being tracked.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v50: Add execution_plan_id to project_active_plan table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "project_active_plan",
        "execution_plan_id",
        "TEXT REFERENCES execution_plans(id)",
    )?;

    Ok(())
}
