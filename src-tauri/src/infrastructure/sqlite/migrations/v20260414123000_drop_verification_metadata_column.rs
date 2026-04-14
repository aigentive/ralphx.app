use rusqlite::Connection;

use super::helpers;
use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    if !helpers::column_exists(conn, "ideation_sessions", "verification_metadata") {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE ideation_sessions DROP COLUMN verification_metadata",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
