// Migration v31: Add session linking schema
//
// This migration:
// 1. Adds parent_session_id column to ideation_sessions (self-reference with ON DELETE SET NULL)
// 2. Creates session_links table with rich metadata for documenting session relationships
// 3. Creates indexes for efficient lookups on both foreign keys
// 4. Enforces UNIQUE constraint on parent/child pairs and self-reference CHECK

use rusqlite::Connection;

use crate::error::{AppError, AppResult};
use super::helpers;

/// Migration v31: Add session linking schema
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Add parent_session_id column to ideation_sessions
    helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "parent_session_id",
        "TEXT REFERENCES ideation_sessions(id) ON DELETE SET NULL",
    )?;

    // Create index on parent_session_id for efficient lookups
    helpers::create_index_if_not_exists(
        conn,
        "idx_ideation_sessions_parent",
        "ideation_sessions",
        "parent_session_id",
    )?;

    // Create session_links table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS session_links (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            child_session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            relationship TEXT NOT NULL DEFAULT 'follow_on',
            notes TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            UNIQUE(parent_session_id, child_session_id),
            CHECK(parent_session_id != child_session_id)
        );

        CREATE INDEX IF NOT EXISTS idx_session_links_parent
            ON session_links(parent_session_id);
        CREATE INDEX IF NOT EXISTS idx_session_links_child
            ON session_links(child_session_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
