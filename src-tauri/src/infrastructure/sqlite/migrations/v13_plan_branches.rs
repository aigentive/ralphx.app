// Migration v13: Plan branches
//
// Adds plan_branches table for feature branch workflow and
// use_feature_branches column to projects table.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v13: Create plan_branches table and add use_feature_branches to projects
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Create plan_branches table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS plan_branches (
            id TEXT PRIMARY KEY,
            plan_artifact_id TEXT NOT NULL UNIQUE,
            session_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            branch_name TEXT NOT NULL,
            source_branch TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            merge_task_id TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            merged_at TEXT
        );",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    // Add use_feature_branches column to projects
    helpers::add_column_if_not_exists(
        conn,
        "projects",
        "use_feature_branches",
        "INTEGER NOT NULL DEFAULT 1",
    )?;

    Ok(())
}
