// Migration v20260410124500: first-class chat message usage columns
//
// Adds additive token/cost fields to chat_messages so assistant turns can carry
// durable per-message usage without reconstructing it from agent_runs later.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    for (column, ty) in [
        ("input_tokens", "INTEGER"),
        ("output_tokens", "INTEGER"),
        ("cache_creation_tokens", "INTEGER"),
        ("cache_read_tokens", "INTEGER"),
        ("estimated_usd", "REAL"),
    ] {
        add_column_if_not_exists(conn, "chat_messages", column, ty)?;
    }

    Ok(())
}
