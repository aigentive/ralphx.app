// Migration v79: External session reliability columns
//
// Adds four columns and a partial UNIQUE index to support behavioral
// guardrails for external MCP agents:
// - api_key_id: which API key created this session (NULL for internal sessions)
// - idempotency_key: client-provided dedup key (NULL if not provided)
// - external_activity_phase: lifecycle phase tracking (NULL for internal sessions)
// - external_last_read_message_id: last message ID the external agent fetched (NULL = never read)
//
// The partial UNIQUE index enforces idempotency: same api_key_id + idempotency_key
// can only map to one session. Rows where idempotency_key IS NULL are excluded
// (NULLs are treated as distinct in SQLite unique indexes).

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

    // Partial UNIQUE index: idempotency is scoped to (api_key_id, idempotency_key) pairs
    // WHERE idempotency_key IS NOT NULL — rows with NULL idempotency_key are excluded.
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_ideation_sessions_idempotency \
         ON ideation_sessions(api_key_id, idempotency_key) \
         WHERE idempotency_key IS NOT NULL",
    )?;

    tracing::info!(
        "v79: added api_key_id, idempotency_key, external_activity_phase, \
         external_last_read_message_id columns + partial UNIQUE index to ideation_sessions"
    );

    Ok(())
}
