// Migration v20260405045108: ideation external overrides

use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "ALTER TABLE ideation_settings ADD COLUMN ext_require_verification_for_accept INTEGER NULL DEFAULT NULL;
         ALTER TABLE ideation_settings ADD COLUMN ext_require_verification_for_proposals INTEGER NULL DEFAULT NULL;
         ALTER TABLE ideation_settings ADD COLUMN ext_require_accept_for_finalize INTEGER NULL DEFAULT NULL;",
    )
    .map_err(|e| crate::error::AppError::Database(format!("Migration v20260405045108 failed: {}", e)))?;
    Ok(())
}
