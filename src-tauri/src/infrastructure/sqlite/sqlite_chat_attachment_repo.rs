// SQLite-based ChatAttachmentRepository implementation
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::{
    ChatAttachment, ChatAttachmentId, ChatConversationId, ChatMessageId,
};
use crate::domain::repositories::ChatAttachmentRepository;
use crate::error::{AppError, AppResult};

/// Parse datetime string handling both RFC3339 and SQLite's CURRENT_TIMESTAMP formats
fn parse_datetime(s: &str) -> DateTime<Utc> {
    // Try RFC3339 first (e.g., "2026-01-26T06:42:37.662598+00:00")
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }

    // Try SQLite's CURRENT_TIMESTAMP format (e.g., "2026-01-26 07:06:32")
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Utc.from_utc_datetime(&ndt);
    }

    // Fallback to now
    Utc::now()
}

/// SQLite implementation of ChatAttachmentRepository
pub struct SqliteChatAttachmentRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteChatAttachmentRepository {
    /// Create a new SQLite chat attachment repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl ChatAttachmentRepository for SqliteChatAttachmentRepository {
    async fn create(&self, attachment: ChatAttachment) -> AppResult<ChatAttachment> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO chat_attachments (id, conversation_id, message_id, file_name, file_path, mime_type, file_size, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                attachment.id.as_str(),
                attachment.conversation_id.as_str(),
                attachment.message_id.as_ref().map(|id| id.as_str()),
                attachment.file_name,
                attachment.file_path,
                attachment.mime_type,
                attachment.file_size,
                attachment.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(attachment)
    }

    async fn get_by_id(&self, id: &ChatAttachmentId) -> AppResult<Option<ChatAttachment>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, conversation_id, message_id, file_name, file_path, mime_type, file_size, created_at
             FROM chat_attachments WHERE id = ?1",
            [id.as_str()],
            |row| {
                let created_at_str: String = row.get("created_at")?;
                let created_at = parse_datetime(&created_at_str);

                Ok(ChatAttachment {
                    id: ChatAttachmentId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(
                        row.get::<_, String>("conversation_id")?,
                    ),
                    message_id: row
                        .get::<_, Option<String>>("message_id")?
                        .map(ChatMessageId::from_string),
                    file_name: row.get("file_name")?,
                    file_path: row.get("file_path")?,
                    mime_type: row.get("mime_type")?,
                    file_size: row.get("file_size")?,
                    created_at,
                })
            },
        );

        match result {
            Ok(attachment) => Ok(Some(attachment)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn find_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatAttachment>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_id, message_id, file_name, file_path, mime_type, file_size, created_at
                 FROM chat_attachments WHERE conversation_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let attachments = stmt
            .query_map([conversation_id.as_str()], |row| {
                let created_at_str: String = row.get("created_at")?;
                let created_at = parse_datetime(&created_at_str);

                Ok(ChatAttachment {
                    id: ChatAttachmentId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(
                        row.get::<_, String>("conversation_id")?,
                    ),
                    message_id: row
                        .get::<_, Option<String>>("message_id")?
                        .map(ChatMessageId::from_string),
                    file_name: row.get("file_name")?,
                    file_path: row.get("file_path")?,
                    mime_type: row.get("mime_type")?,
                    file_size: row.get("file_size")?,
                    created_at,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(attachments)
    }

    async fn find_by_message_id(
        &self,
        message_id: &ChatMessageId,
    ) -> AppResult<Vec<ChatAttachment>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_id, message_id, file_name, file_path, mime_type, file_size, created_at
                 FROM chat_attachments WHERE message_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let attachments = stmt
            .query_map([message_id.as_str()], |row| {
                let created_at_str: String = row.get("created_at")?;
                let created_at = parse_datetime(&created_at_str);

                Ok(ChatAttachment {
                    id: ChatAttachmentId::from_string(row.get::<_, String>("id")?),
                    conversation_id: ChatConversationId::from_string(
                        row.get::<_, String>("conversation_id")?,
                    ),
                    message_id: row
                        .get::<_, Option<String>>("message_id")?
                        .map(ChatMessageId::from_string),
                    file_name: row.get("file_name")?,
                    file_path: row.get("file_path")?,
                    mime_type: row.get("mime_type")?,
                    file_size: row.get("file_size")?,
                    created_at,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(attachments)
    }

    async fn update_message_id(
        &self,
        id: &ChatAttachmentId,
        message_id: &ChatMessageId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE chat_attachments SET message_id = ?1 WHERE id = ?2",
            rusqlite::params![message_id.as_str(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_message_ids(
        &self,
        attachment_ids: &[ChatAttachmentId],
        message_id: &ChatMessageId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // Convert IDs to strings first to avoid lifetime issues
        let message_id_str = message_id.as_str().to_string();
        let id_strings: Vec<String> = attachment_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();

        // Build a list of placeholders for the IN clause
        let placeholders = id_strings
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "UPDATE chat_attachments SET message_id = ?1 WHERE id IN ({})",
            placeholders
        );

        // Build params using the owned strings
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&message_id_str as &dyn rusqlite::ToSql];
        for id_str in &id_strings {
            params.push(id_str as &dyn rusqlite::ToSql);
        }

        conn.execute(&query, params.as_slice())
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ChatAttachmentId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_attachments WHERE id = ?1",
            [id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_attachments WHERE conversation_id = ?1",
            [conversation_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
