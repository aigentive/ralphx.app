// Migration v76: Add origin column to ideation_sessions
//
// Tracks whether a session was created by an internal user/agent or an external agent
// via the External MCP API. External sessions cannot skip plan verification.
//
// Default: 'internal' — all existing sessions are treated as internal.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "origin",
        "TEXT NOT NULL DEFAULT 'internal'",
    )?;

    tracing::info!("v76: added origin column to ideation_sessions");

    Ok(())
}
