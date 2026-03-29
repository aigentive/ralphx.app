// Migration v20260329113000: Add blocker_fingerprint to ideation follow-up sessions
//
// Adds blocker_fingerprint to ideation_sessions so autonomous follow-up creation can
// dedupe the same blocker across worker/reviewer contexts without relying on wording.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "ideation_sessions", "blocker_fingerprint", "TEXT")?;

    tracing::info!("v20260329113000: added blocker_fingerprint column to ideation_sessions");

    Ok(())
}
