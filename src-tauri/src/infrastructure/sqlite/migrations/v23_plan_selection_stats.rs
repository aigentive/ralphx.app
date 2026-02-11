// Migration v23: Add plan_selection_stats table
//
// Tracks plan selection interactions for ranking and analytics.
// Used by the plan selector UI to intelligently order plans based on
// user interaction patterns.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v23: Create plan_selection_stats table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS plan_selection_stats (
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            ideation_session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            selected_count INTEGER NOT NULL DEFAULT 0,
            last_selected_at TEXT NULL,
            last_selected_source TEXT NULL,
            PRIMARY KEY (project_id, ideation_session_id)
        );

        CREATE INDEX IF NOT EXISTS idx_plan_selection_stats_session
            ON plan_selection_stats(ideation_session_id);

        CREATE INDEX IF NOT EXISTS idx_plan_selection_stats_last_selected
            ON plan_selection_stats(last_selected_at);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
