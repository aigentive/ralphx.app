// Migration v20: Add merge_validation_mode column to projects table
//
// Adds configurable validation behavior for post-merge validation:
// - block (default): Validation failure → MergeIncomplete
// - warn: Validation failure → proceed to Merged, store warnings
// - off: Skip validation entirely

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v20: Add merge_validation_mode column to projects table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "projects",
        "merge_validation_mode",
        "TEXT NOT NULL DEFAULT 'block'",
    )?;
    Ok(())
}
