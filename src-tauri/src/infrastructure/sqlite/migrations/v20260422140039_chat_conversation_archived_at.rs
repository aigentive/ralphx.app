// Migration v20260422140039: chat conversation archived at

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(conn, "chat_conversations", "archived_at", "TEXT NULL")?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_conversations_archived_at
             ON chat_conversations(archived_at)",
        [],
    )?;
    Ok(())
}
