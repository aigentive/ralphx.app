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
    // Step 1: Add project_id column to execution_settings
    // NULL means global defaults (the existing row with id=1)
    helpers::add_column_if_not_exists(
        conn,
        "execution_settings",
        "project_id",
        "TEXT DEFAULT NULL",
    )?;

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
