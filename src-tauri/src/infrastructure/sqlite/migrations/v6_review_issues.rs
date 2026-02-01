// Migration v6: Add review_issues table
//
// This migration adds the review_issues table for tracking structured issues
// from reviews. Issues have a lifecycle (open → in_progress → addressed → verified)
// and can be linked to specific task steps.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Create the review_issues table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS review_issues (
            id TEXT PRIMARY KEY,
            review_note_id TEXT NOT NULL REFERENCES review_notes(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            step_id TEXT REFERENCES task_steps(id) ON DELETE SET NULL,
            no_step_reason TEXT,

            -- Issue details
            title TEXT NOT NULL,
            description TEXT,
            severity TEXT NOT NULL CHECK (severity IN ('critical', 'major', 'minor', 'suggestion')),
            category TEXT CHECK (category IN ('bug', 'missing', 'quality', 'design')),

            -- Location (optional)
            file_path TEXT,
            line_number INTEGER,
            code_snippet TEXT,

            -- Status lifecycle
            status TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'in_progress', 'addressed', 'verified', 'wontfix')),

            -- Resolution tracking
            resolution_notes TEXT,
            addressed_in_attempt INTEGER,
            verified_by_review_id TEXT REFERENCES review_notes(id) ON DELETE SET NULL,

            -- Timestamps
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for querying by task
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_issues_task_id ON review_issues(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for filtering by status
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_issues_status ON review_issues(status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for querying by review note
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_review_issues_review_note ON review_issues(review_note_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
