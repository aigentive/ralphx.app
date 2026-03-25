// Migration v20260325131500: execution ideation allocation settings

use rusqlite::Connection;

use super::helpers;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(
        conn,
        "execution_settings",
        "project_ideation_max",
        "INTEGER NOT NULL DEFAULT 2",
    )?;
    helpers::add_column_if_not_exists(
        conn,
        "global_execution_settings",
        "global_ideation_max",
        "INTEGER NOT NULL DEFAULT 4",
    )?;
    helpers::add_column_if_not_exists(
        conn,
        "global_execution_settings",
        "allow_ideation_borrow_idle_execution",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}
