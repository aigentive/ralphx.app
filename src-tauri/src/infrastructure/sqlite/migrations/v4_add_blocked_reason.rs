// Migration v4: Add blocked_reason column to tasks
//
// This migration adds a `blocked_reason` column to store the reason
// why a task is blocked, enabling users to document blockers.

use rusqlite::Connection;

use super::helpers;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "tasks", "blocked_reason", "TEXT DEFAULT NULL")?;
    Ok(())
}
