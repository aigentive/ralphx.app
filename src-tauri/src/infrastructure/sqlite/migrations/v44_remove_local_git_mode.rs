// Migration v44: Remove local git mode
//
// Converts all projects with git_mode='local' to 'worktree' and sets a default
// worktree_parent_directory for projects that don't have one.
// This migration supports the removal of GitMode::Local from the codebase.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // 1. Convert all local-mode projects to worktree
    conn.execute(
        "UPDATE projects SET git_mode = 'worktree' WHERE git_mode = 'local'",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // 2. Set default worktree_parent_directory for projects that don't have one.
    // Uses ~/ralphx-worktrees which matches worktree_parent_or_default() fallback.
    // The ~ is resolved at runtime by the application layer.
    conn.execute(
        "UPDATE projects SET worktree_parent_directory = '~/ralphx-worktrees' WHERE worktree_parent_directory IS NULL OR worktree_parent_directory = ''",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
