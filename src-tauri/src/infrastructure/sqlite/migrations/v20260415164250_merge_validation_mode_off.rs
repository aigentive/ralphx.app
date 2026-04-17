// Migration v20260415164250: merge validation mode off

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute_batch(
        "-- Safety: recover cleanly if a previous repair was interrupted.
         DROP TABLE IF EXISTS projects_new;

         CREATE TABLE projects_new (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            worktree_branch TEXT,
            base_branch TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            worktree_parent_directory TEXT,
            use_feature_branches INTEGER NOT NULL DEFAULT 1,
            detected_analysis TEXT DEFAULT NULL,
            custom_analysis TEXT DEFAULT NULL,
            analyzed_at TEXT DEFAULT NULL,
            merge_validation_mode TEXT NOT NULL DEFAULT 'off',
            merge_strategy TEXT NOT NULL DEFAULT 'rebase',
            github_pr_enabled BOOLEAN NOT NULL DEFAULT 1,
            archived_at TEXT NULL
         );

         INSERT INTO projects_new (
            id,
            name,
            working_directory,
            git_mode,
            worktree_path,
            worktree_branch,
            base_branch,
            created_at,
            updated_at,
            worktree_parent_directory,
            use_feature_branches,
            detected_analysis,
            custom_analysis,
            analyzed_at,
            merge_validation_mode,
            merge_strategy,
            github_pr_enabled,
            archived_at
         )
         SELECT
            id,
            name,
            working_directory,
            git_mode,
            worktree_path,
            worktree_branch,
            base_branch,
            created_at,
            updated_at,
            worktree_parent_directory,
            use_feature_branches,
            detected_analysis,
            custom_analysis,
            analyzed_at,
            'off',
            merge_strategy,
            github_pr_enabled,
            archived_at
         FROM projects;

         DROP TABLE projects;
         ALTER TABLE projects_new RENAME TO projects;

         CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_working_dir
            ON projects(working_directory)
            WHERE working_directory IS NOT NULL;
         CREATE INDEX IF NOT EXISTS idx_projects_archived_at
            ON projects(archived_at);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
