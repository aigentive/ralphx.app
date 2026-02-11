// Migration v22: Project active plan table
//
// Creates project_active_plan table to persist the active plan (ideation session)
// per project. Supports get/set/clear operations with validation that only
// accepted sessions can be set as active.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v22: Create project_active_plan and plan_selection_stats tables
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_active_plan (
            project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
            ideation_session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );
        CREATE INDEX IF NOT EXISTS idx_project_active_plan_session ON project_active_plan(ideation_session_id);

        CREATE TABLE IF NOT EXISTS plan_selection_stats (
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            ideation_session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            selected_count INTEGER NOT NULL DEFAULT 0,
            last_selected_at TEXT NULL,
            last_selected_source TEXT NULL,
            PRIMARY KEY (project_id, ideation_session_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plan_selection_stats_session ON plan_selection_stats(ideation_session_id);
        CREATE INDEX IF NOT EXISTS idx_plan_selection_stats_last_selected ON plan_selection_stats(last_selected_at);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
