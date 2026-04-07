// Migration v20260407103000: add agent_lane_settings table
//
// Provider-neutral lane settings storage used for the upcoming multi-harness
// ideation and execution routing model.

use rusqlite::Connection;

use super::helpers::table_exists;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    if table_exists(conn, "agent_lane_settings") {
        return Ok(());
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_lane_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scope_type TEXT NOT NULL,
            scope_id TEXT,
            lane TEXT NOT NULL,
            harness TEXT NOT NULL,
            model TEXT,
            effort TEXT,
            approval_policy TEXT,
            sandbox_mode TEXT,
            fallback_harness TEXT,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_agent_lane_settings_scope_lane
            ON agent_lane_settings(scope_type, scope_id, lane);
        CREATE INDEX IF NOT EXISTS idx_agent_lane_settings_scope
            ON agent_lane_settings(scope_type, scope_id);",
    )?;

    Ok(())
}
