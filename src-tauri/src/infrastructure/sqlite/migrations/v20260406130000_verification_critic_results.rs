// Migration v20260406130000: add verification_critic_results table

use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS verification_critic_results (
            id TEXT PRIMARY KEY,
            parent_session_id TEXT NOT NULL,
            verification_session_id TEXT NOT NULL,
            verification_generation INTEGER NOT NULL,
            round INTEGER NOT NULL,
            critic_kind TEXT NOT NULL,
            artifact_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'complete',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (parent_session_id, verification_generation, round, critic_kind)
        );",
    )
    .map_err(|e| crate::error::AppError::Database(format!("Migration v20260406130000 failed: {}", e)))?;
    Ok(())
}
