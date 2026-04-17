// Migration v20260410113000: first-class agent run usage columns
//
// Adds additive token/cache/cost fields to agent_runs so native runtime capture
// and historical backfill can persist durable usage without hiding it in JSON.

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
        add_column_if_not_exists(conn, "agent_runs", column, ty)?;
    }

    Ok(())
}
