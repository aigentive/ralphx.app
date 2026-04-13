// Migration v20260413043153: drop agent lane settings fallback harness

use rusqlite::Connection;

use super::helpers::{column_exists, create_index_if_not_exists, table_exists};
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    if !table_exists(conn, "agent_lane_settings")
        || !column_exists(conn, "agent_lane_settings", "fallback_harness")
    {
        return Ok(());
    }

    conn.execute_batch(
        "BEGIN IMMEDIATE;
         CREATE TABLE agent_lane_settings_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scope_type TEXT NOT NULL,
            scope_id TEXT,
            lane TEXT NOT NULL,
            harness TEXT NOT NULL,
            model TEXT,
            effort TEXT,
            approval_policy TEXT,
            sandbox_mode TEXT,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
         );
         INSERT INTO agent_lane_settings_new (
            id, scope_type, scope_id, lane, harness, model, effort,
            approval_policy, sandbox_mode, updated_at
         )
         SELECT
            id, scope_type, scope_id, lane, harness, model, effort,
            approval_policy, sandbox_mode, updated_at
         FROM agent_lane_settings;
         DROP TABLE agent_lane_settings;
         ALTER TABLE agent_lane_settings_new RENAME TO agent_lane_settings;
         COMMIT;",
    )?;

    create_index_if_not_exists(
        conn,
        "idx_agent_lane_settings_scope_lane",
        "agent_lane_settings",
        "scope_type, scope_id, lane",
    )?;
    create_index_if_not_exists(
        conn,
        "idx_agent_lane_settings_scope",
        "agent_lane_settings",
        "scope_type, scope_id",
    )?;

    Ok(())
}
