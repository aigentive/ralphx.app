// Migration v20260407073000: add provider-neutral harness metadata
//
// Adds cross-provider session metadata to chat_conversations and agent_runs.
// This is an additive compatibility migration only; runtime reads/writes
// continue to use existing Claude-shaped fields until later slices switch over.

use rusqlite::Connection;

use super::helpers::{add_column_if_not_exists, create_index_if_not_exists};
use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(conn, "chat_conversations", "provider_session_id", "TEXT")?;
    add_column_if_not_exists(conn, "chat_conversations", "provider_harness", "TEXT")?;
    create_index_if_not_exists(
        conn,
        "idx_chat_conversations_provider_session",
        "chat_conversations",
        "provider_harness, provider_session_id",
    )?;

    add_column_if_not_exists(conn, "agent_runs", "harness", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "provider_session_id", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "logical_model", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "effective_model_id", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "logical_effort", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "effective_effort", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "approval_policy", "TEXT")?;
    add_column_if_not_exists(conn, "agent_runs", "sandbox_mode", "TEXT")?;
    create_index_if_not_exists(
        conn,
        "idx_agent_runs_provider_session",
        "agent_runs",
        "harness, provider_session_id",
    )?;

    Ok(())
}
