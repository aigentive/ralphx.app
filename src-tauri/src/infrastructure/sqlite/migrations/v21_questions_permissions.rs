// Migration v21: Add pending_questions and pending_permissions tables
//
// Persists question/permission state to SQLite for restart resilience
// and audit trail. In-memory channels remain for long-poll signaling.

use rusqlite::Connection;

use crate::error::AppResult;

/// Migration v21: Create pending_questions and pending_permissions tables
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pending_questions (
            request_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            question TEXT NOT NULL,
            header TEXT,
            options TEXT NOT NULL DEFAULT '[]',
            multi_select INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            answer_selected_options TEXT,
            answer_text TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            resolved_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_pending_questions_status
            ON pending_questions(status);

        CREATE INDEX IF NOT EXISTS idx_pending_questions_session_id
            ON pending_questions(session_id);

        CREATE TABLE IF NOT EXISTS pending_permissions (
            request_id TEXT PRIMARY KEY,
            tool_name TEXT NOT NULL,
            tool_input TEXT NOT NULL DEFAULT '{}',
            context TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            decision TEXT,
            decision_message TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            resolved_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_pending_permissions_status
            ON pending_permissions(status);",
    )
    .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

    Ok(())
}
