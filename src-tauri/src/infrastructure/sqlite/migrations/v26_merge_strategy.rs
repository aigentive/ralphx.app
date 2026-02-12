// Migration v26: Add merge_strategy column to projects table
//
// Adds configurable merge strategy for branch merging:
// - rebase (default): Rebase source onto target, then fast-forward (linear history)
// - merge: Direct merge commit (non-linear)

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v26: Add merge_strategy column to projects table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "projects",
        "merge_strategy",
        "TEXT NOT NULL DEFAULT 'rebase'",
    )?;
    Ok(())
}
