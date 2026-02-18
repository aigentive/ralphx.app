// Migration v40: Add source column to proposal_dependencies
//
// Adds source TEXT NOT NULL DEFAULT 'auto' to distinguish manually-set
// dependencies from auto-suggested ones. All existing rows default to 'auto'.
// This allows preserving manual dependencies when re-running auto-suggestion.

use crate::error::AppResult;
use rusqlite::Connection;

use super::helpers::add_column_if_not_exists;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(
        conn,
        "proposal_dependencies",
        "source",
        "TEXT NOT NULL DEFAULT 'auto'",
    )?;
    Ok(())
}
