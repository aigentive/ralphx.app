// Migration v34: Add chat_attachments table
//
// This migration creates the chat_attachments table for storing file
// attachments associated with chat messages. Attachments are initially
// linked to a conversation (when uploaded), then linked to a message
// after the message is sent.
//
// Storage pattern: {app_data_dir}/chat_attachments/{conversation_id}/{attachment_id}/{file_name}

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v34: Create chat_attachments table
pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "-- Chat attachments table
        CREATE TABLE IF NOT EXISTS chat_attachments (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            message_id TEXT,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            mime_type TEXT,
            file_size INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            FOREIGN KEY (conversation_id) REFERENCES chat_conversations(id) ON DELETE CASCADE
        );

        -- Indexes for chat_attachments
        CREATE INDEX IF NOT EXISTS idx_chat_attachments_conversation
            ON chat_attachments(conversation_id);

        CREATE INDEX IF NOT EXISTS idx_chat_attachments_message
            ON chat_attachments(message_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
