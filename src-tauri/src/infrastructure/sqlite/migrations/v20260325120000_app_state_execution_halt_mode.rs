// Migration v20260325120000: Persist global execution halt mode in app_state
//
// Adds a durable halt-mode column so startup recovery can distinguish
// normal boots from user-requested Pause/Stop waves.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "app_state",
        "execution_halt_mode",
        "TEXT NOT NULL DEFAULT 'running'",
    )?;

    conn.execute(
        "UPDATE app_state
         SET execution_halt_mode = COALESCE(execution_halt_mode, 'running')
         WHERE id = 1",
        [],
    )?;

    Ok(())
}
