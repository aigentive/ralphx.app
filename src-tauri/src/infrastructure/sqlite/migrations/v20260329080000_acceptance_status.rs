//! Migration v20260329080000: Add acceptance_status to ideation_sessions and require_accept_for_finalize to ideation settings
//!
//! Adds:
//! - `acceptance_status TEXT` column to `ideation_sessions` table (NULL = gate not triggered)
//! - `require_accept_for_finalize INTEGER DEFAULT 0` column to `ideation_settings` table

use rusqlite::Connection;

use crate::error::AppResult;
use crate::infrastructure::sqlite::migrations::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "ideation_sessions", "acceptance_status", "TEXT")?;
    helpers::add_column_if_not_exists(
        conn,
        "ideation_settings",
        "require_accept_for_finalize",
        "INTEGER NOT NULL DEFAULT 0",
    )?;

    tracing::info!(
        "v20260329080000: added acceptance_status to ideation_sessions and require_accept_for_finalize to ideation_settings"
    );

    Ok(())
}
