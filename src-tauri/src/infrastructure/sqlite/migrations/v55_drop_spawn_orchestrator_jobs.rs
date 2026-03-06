// Migration v55: Drop orphaned spawn_orchestrator_jobs table
//
// This table was created in v36 but has no runtime code reading or writing it.
// Dropping it eliminates stale "running" rows after a crash and removes dead weight.

use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("DROP TABLE IF EXISTS spawn_orchestrator_jobs;")?;
    Ok(())
}
