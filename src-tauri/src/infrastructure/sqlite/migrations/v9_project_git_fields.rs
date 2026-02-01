// Migration v9: Add worktree_parent_directory column to projects table
//
// This migration adds the worktree_parent_directory column to support
// per-task git branch isolation with worktree mode.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

/// Migration v9: Add worktree_parent_directory column
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Add worktree_parent_directory column with NULL default
    // This column stores the parent directory for task worktrees
    // Default value is handled at application level (~/ralphx-worktrees)
    add_column_if_not_exists(conn, "projects", "worktree_parent_directory", "TEXT")?;

    Ok(())
}
