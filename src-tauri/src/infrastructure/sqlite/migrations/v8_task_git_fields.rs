// Migration v8: Add git branch isolation fields to tasks
//
// This migration adds fields for per-task git branch isolation (Phase 66):
// - task_branch: Git branch name for this task
// - worktree_path: Worktree path (Worktree mode only)
// - merge_commit_sha: Commit SHA after merge to base branch

use rusqlite::Connection;

use super::helpers;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Add task_branch column for storing the git branch name
    // Format: ralphx/{project-slug}/task-{task-id}
    helpers::add_column_if_not_exists(conn, "tasks", "task_branch", "TEXT DEFAULT NULL")?;

    // Add worktree_path column for worktree mode
    // Only populated when project.git_mode == Worktree
    helpers::add_column_if_not_exists(conn, "tasks", "worktree_path", "TEXT DEFAULT NULL")?;

    // Add merge_commit_sha column for tracking the merge commit
    // Set when task transitions to Merged state
    helpers::add_column_if_not_exists(conn, "tasks", "merge_commit_sha", "TEXT DEFAULT NULL")?;

    Ok(())
}
