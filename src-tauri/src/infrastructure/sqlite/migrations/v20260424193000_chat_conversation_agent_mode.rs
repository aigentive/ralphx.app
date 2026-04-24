// Migration v20260424193000: Agents conversation mode

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "agent_mode",
        "TEXT CHECK(agent_mode IN ('chat', 'edit', 'ideation'))",
    )?;
    Ok(())
}
