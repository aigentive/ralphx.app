// Migration v2: Add reason column to proposal_dependencies
//
// This migration adds a `reason` column to store the AI's explanation
// for why a dependency exists (e.g., "API needs database schema to exist").

use rusqlite::Connection;

use super::helpers;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "proposal_dependencies", "reason", "TEXT DEFAULT NULL")?;
    Ok(())
}
