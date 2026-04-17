// Migration v20260410093000: conversation-level attribution backfill workflow state
//
// Adds additive columns to chat_conversations so startup/background services can
// track historical Claude transcript recovery without overloading free-form JSON.

use rusqlite::Connection;

use crate::error::AppResult;

use super::helpers::{add_column_if_not_exists, create_index_if_not_exists};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_status",
        "TEXT",
    )?;
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_source",
        "TEXT",
    )?;
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_source_path",
        "TEXT",
    )?;
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_last_attempted_at",
        "TEXT",
    )?;
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_completed_at",
        "TEXT",
    )?;
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "attribution_backfill_error_summary",
        "TEXT",
    )?;

    create_index_if_not_exists(
        conn,
        "idx_chat_conversations_attribution_backfill_status",
        "chat_conversations",
        "attribution_backfill_status, updated_at",
    )?;

    Ok(())
}
