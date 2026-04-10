// Migration v20260410101500: first-class chat message attribution columns
//
// Adds additive provider/model/effort attribution to chat_messages so assistant
// turns can carry durable run metadata without relying on free-form JSON only.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::{add_column_if_not_exists, create_index_if_not_exists};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    for (column, ty) in [
        ("attribution_source", "TEXT"),
        ("provider_harness", "TEXT"),
        ("provider_session_id", "TEXT"),
        ("logical_model", "TEXT"),
        ("effective_model_id", "TEXT"),
        ("logical_effort", "TEXT"),
        ("effective_effort", "TEXT"),
    ] {
        add_column_if_not_exists(conn, "chat_messages", column, ty)?;
    }

    create_index_if_not_exists(
        conn,
        "idx_chat_messages_provider_session",
        "chat_messages",
        "conversation_id, provider_harness, provider_session_id",
    )?;

    Ok(())
}
