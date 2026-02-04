// Migration v11: Add per-project execution settings and global concurrency cap
//
// This migration:
// 1. Adds project_id column to execution_settings for per-project settings
// 2. Creates global_execution_settings table for global_max_concurrent cap
// 3. Removes id=1 constraint to allow multiple project-specific rows
//
// Phase 82: Project-scoped execution control

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::migrations::helpers;

/// Migration v11: Add per-project execution settings and global concurrency cap
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Step 1: Recreate execution_settings WITHOUT the CHECK(id=1) constraint.
    // The v10 table used a singleton pattern; v11 needs multiple rows (one per project).
    // SQLite doesn't support DROP CONSTRAINT, so we recreate the table.
    let has_project_id = helpers::column_exists(conn, "execution_settings", "project_id");
    if !has_project_id {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS execution_settings_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                max_concurrent_tasks INTEGER NOT NULL DEFAULT 2,
                auto_commit INTEGER NOT NULL DEFAULT 1,
                pause_on_failure INTEGER NOT NULL DEFAULT 1,
                updated_at TEXT NOT NULL,
                project_id TEXT DEFAULT NULL
            );
            INSERT OR IGNORE INTO execution_settings_new (id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
                SELECT id, max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, NULL
                FROM execution_settings;
            DROP TABLE IF EXISTS execution_settings;
            ALTER TABLE execution_settings_new RENAME TO execution_settings;",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
    }

    // Step 2: Create global_execution_settings table for cross-project settings
    // Singleton pattern with id=1 constraint
    conn.execute(
        "CREATE TABLE IF NOT EXISTS global_execution_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            global_max_concurrent INTEGER NOT NULL DEFAULT 20,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Step 3: Insert default global settings if they don't exist
    conn.execute(
        "INSERT OR IGNORE INTO global_execution_settings (id, global_max_concurrent, updated_at)
         VALUES (1, 20, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Step 4: Create unique index on project_id (NULL for global, unique per project)
    // This allows one row per project plus one global row (where project_id IS NULL)
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_execution_settings_project_id
         ON execution_settings(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
