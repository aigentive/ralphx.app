// Migration v20260406000000: add verifier_subagent_model column to ideation_model_settings

use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "ALTER TABLE ideation_model_settings ADD COLUMN verifier_subagent_model TEXT NOT NULL DEFAULT 'inherit';",
    )
    .map_err(|e| crate::error::AppError::Database(format!("Migration v20260406000000 failed: {}", e)))?;
    Ok(())
}
