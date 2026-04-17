// Migration v20260410143000: first-class upstream provider/profile metadata
//
// Adds additive provider-origin fields so native and historical attribution can
// distinguish harness (`claude`, `codex`) from the upstream provider/profile
// behind that harness (`anthropic`, `z_ai`, `openai`, etc.).

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    for table in ["agent_runs", "chat_messages"] {
        add_column_if_not_exists(conn, table, "upstream_provider", "TEXT")?;
        add_column_if_not_exists(conn, table, "provider_profile", "TEXT")?;
    }

    Ok(())
}
