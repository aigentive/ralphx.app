// Migration v81: Repair skipped external session reliability columns
//
// Some upgraded databases recorded an earlier meaning of schema version 79
// before the external session reliability migration was later assigned to the
// same version number. Those databases advanced to version 80 without ever
// adding the external-session columns. This forward-only repair migration
// backfills the missing columns and idempotency index.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    helpers::add_column_if_not_exists(conn, "ideation_sessions", "api_key_id", "TEXT")?;

    helpers::add_column_if_not_exists(conn, "ideation_sessions", "idempotency_key", "TEXT")?;

    helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "external_activity_phase",
        "TEXT",
    )?;

    helpers::add_column_if_not_exists(
        conn,
        "ideation_sessions",
        "external_last_read_message_id",
        "TEXT",
    )?;

    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_ideation_sessions_idempotency \
         ON ideation_sessions(api_key_id, idempotency_key) \
         WHERE idempotency_key IS NOT NULL",
    )?;

    tracing::info!(
        "v81: backfilled api_key_id, idempotency_key, external_activity_phase, \
         external_last_read_message_id columns + partial UNIQUE index to ideation_sessions"
    );

    Ok(())
}
