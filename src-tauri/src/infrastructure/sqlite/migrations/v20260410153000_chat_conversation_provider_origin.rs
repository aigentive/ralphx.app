// Migration v20260410153000: conversation-level provider origin metadata
//
// Adds additive provider-origin fields on chat_conversations so aggregate
// surfaces do not need to reconstruct the active provider/profile from runs or
// messages each time.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(conn, "chat_conversations", "upstream_provider", "TEXT")?;
    add_column_if_not_exists(conn, "chat_conversations", "provider_profile", "TEXT")?;
    Ok(())
}
