// Migration v18: Add metadata column to tasks table
//
// Adds a generic TEXT column for structured JSON metadata.
// Used initially for merge error context (error message, branch names).

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v18: Add metadata column to tasks table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "tasks", "metadata", "TEXT DEFAULT NULL")?;
    Ok(())
}
