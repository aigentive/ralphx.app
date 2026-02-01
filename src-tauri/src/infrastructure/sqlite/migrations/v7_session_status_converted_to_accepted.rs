//! Migration v7: Update session status 'converted' to 'accepted'
//!
//! This migration renames the 'converted' session status to 'accepted' for clarity.
//! Part of the terminology changes: Converted → Accepted

use rusqlite::Connection;

use crate::error::AppResult;

/// Run the migration to update session status from 'converted' to 'accepted'
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Update any existing sessions with 'converted' status to 'accepted'
    conn.execute(
        "UPDATE ideation_sessions SET status = 'accepted' WHERE status = 'converted'",
        [],
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
