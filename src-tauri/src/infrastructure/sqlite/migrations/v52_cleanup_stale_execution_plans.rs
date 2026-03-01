// Migration v52: Clean up stale execution plans
//
// Supersede execution_plans left in 'active' status due to the duplicate-apply
// bug (rapid double-clicks on Accept Plan). Only plans referenced by
// project_active_plan should remain active.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "UPDATE execution_plans SET status = 'superseded'
         WHERE status = 'active'
           AND id NOT IN (
               SELECT execution_plan_id FROM project_active_plan
               WHERE execution_plan_id IS NOT NULL
           )",
        [],
    )
    .map_err(|e| {
        AppError::Database(format!(
            "v52: failed to supersede stale execution plans: {}",
            e
        ))
    })?;

    tracing::info!("v52: superseded stale execution plans");

    Ok(())
}
