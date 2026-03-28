use rusqlite::Connection;

use crate::error::AppResult;
use crate::infrastructure::sqlite::migrations::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "task_proposals", "affected_paths", "TEXT")?;

    tracing::info!("v20260328210000: added affected_paths column to task_proposals");

    Ok(())
}
