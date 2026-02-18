// Migration v39: Add parent_conversation_id to chat_conversations
//
// Adds parent_conversation_id column to support linking execution generations.
// When a task is re-executed, a new conversation row is created and this column
// points to the prior run's conversation for UI navigation.

use crate::error::AppResult;
use rusqlite::Connection;

use super::helpers::{add_column_if_not_exists, create_index_if_not_exists};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Nullable FK referencing chat_conversations(id), ON DELETE SET NULL
    add_column_if_not_exists(
        conn,
        "chat_conversations",
        "parent_conversation_id",
        "TEXT REFERENCES chat_conversations(id) ON DELETE SET NULL",
    )?;

    // Index for efficient parent→children lookups
    create_index_if_not_exists(
        conn,
        "idx_chat_conversations_parent_id",
        "chat_conversations",
        "parent_conversation_id",
    )?;

    Ok(())
}
