use rusqlite::Connection;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_settings",
        "require_verification_for_accept",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    crate::infrastructure::sqlite::migrations::helpers::add_column_if_not_exists(
        conn,
        "ideation_settings",
        "require_verification_for_proposals",
        "INTEGER NOT NULL DEFAULT 0",
    )
}
