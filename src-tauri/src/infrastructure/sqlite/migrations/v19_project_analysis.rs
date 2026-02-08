// Migration v19: Add project analysis columns to projects table
//
// Adds columns for intelligent project validation & worktree setup:
// - detected_analysis: JSON array of path-based commands (written by analyzer agent)
// - custom_analysis: JSON array of user overrides
// - analyzed_at: RFC3339 datetime of last analysis

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

/// Migration v19: Add project analysis columns to projects table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "projects", "detected_analysis", "TEXT DEFAULT NULL")?;
    helpers::add_column_if_not_exists(conn, "projects", "custom_analysis", "TEXT DEFAULT NULL")?;
    helpers::add_column_if_not_exists(conn, "projects", "analyzed_at", "TEXT DEFAULT NULL")?;
    Ok(())
}
