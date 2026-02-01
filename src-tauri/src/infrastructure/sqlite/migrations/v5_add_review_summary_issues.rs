// Migration v5: Add summary and issues columns to review_notes table
//
// These fields allow storing review data cleanly:
// - summary: Short description for timeline display
// - issues: JSON array of review issues

use rusqlite::Connection;

use super::helpers::add_column_if_not_exists;
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Add summary column
    add_column_if_not_exists(conn, "review_notes", "summary", "TEXT")?;

    // Add issues column (JSON array stored as text)
    add_column_if_not_exists(conn, "review_notes", "issues", "TEXT")?;

    Ok(())
}
