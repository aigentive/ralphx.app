use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "pending_permissions", "agent_type", "TEXT")?;
    helpers::add_column_if_not_exists(conn, "pending_permissions", "task_id", "TEXT")?;
    helpers::add_column_if_not_exists(conn, "pending_permissions", "context_type", "TEXT")?;
    helpers::add_column_if_not_exists(conn, "pending_permissions", "context_id", "TEXT")?;
    Ok(())
}
