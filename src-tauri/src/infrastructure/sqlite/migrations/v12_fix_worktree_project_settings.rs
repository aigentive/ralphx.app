// Migration v12: Fix worktree project settings
//
// Projects with git_mode = 'worktree' may have NULL or empty base_branch
// and worktree_parent_directory. This migration sets sensible defaults.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v12: Set default base_branch and worktree_parent_directory
/// for worktree-mode projects that have missing values
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Fix base_branch for worktree projects with NULL or empty value
    conn.execute(
        "UPDATE projects SET base_branch = 'main'
         WHERE git_mode = 'worktree'
           AND (base_branch IS NULL OR base_branch = '')",
        [],
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    // Fix worktree_parent_directory for worktree projects with NULL or empty value
    conn.execute(
        "UPDATE projects SET worktree_parent_directory = '~/ralphx-worktrees'
         WHERE git_mode = 'worktree'
           AND (worktree_parent_directory IS NULL OR worktree_parent_directory = '')",
        [],
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
