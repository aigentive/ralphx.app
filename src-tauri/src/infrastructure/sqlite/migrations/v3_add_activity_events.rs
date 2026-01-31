// Migration v3: Add activity_events table
//
// This migration adds the activity_events table for persistent storage of
// activity stream events (thinking blocks, tool calls, tool results, text, errors).
// Events belong to either a task or an ideation session (mutually exclusive).

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Create the activity_events table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS activity_events (
            id TEXT PRIMARY KEY,
            -- Context (polymorphic: exactly one must be set)
            task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
            ideation_session_id TEXT REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            -- State snapshot when event occurred
            internal_status TEXT,
            -- Event data
            event_type TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'agent',
            content TEXT NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),

            CHECK ((task_id IS NOT NULL) != (ideation_session_id IS NOT NULL))
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for querying by task
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_task_id ON activity_events(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for querying by session
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_session_id ON activity_events(ideation_session_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for filtering by event type
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_type ON activity_events(event_type)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for ordering by creation time (descending for recent-first queries)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_created_at ON activity_events(created_at DESC)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Composite index for cursor-based pagination by task
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_task_cursor ON activity_events(task_id, created_at DESC, id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Composite index for cursor-based pagination by session
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_events_session_cursor ON activity_events(ideation_session_id, created_at DESC, id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
