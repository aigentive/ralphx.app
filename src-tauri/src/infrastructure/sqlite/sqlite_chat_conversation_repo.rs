// SQLite-based ChatConversationRepository implementation
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rusqlite::Connection;

use crate::domain::entities::{ChatContextType, ChatConversation, ChatConversationId};
use crate::domain::repositories::ChatConversationRepository;
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

/// SQLite implementation of ChatConversationRepository
pub struct SqliteChatConversationRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteChatConversationRepository {
    /// Create a new SQLite chat conversation repository with the given connection
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
impl ChatConversationRepository for SqliteChatConversationRepository {
    async fn create(&self, conversation: ChatConversation) -> AppResult<ChatConversation> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO chat_conversations (id, context_type, context_id, claude_session_id, title, message_count, last_message_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                conversation.id.as_str(),
                conversation.context_type.to_string(),
                conversation.context_id,
                conversation.claude_session_id,
                conversation.title,
                conversation.message_count,
                conversation.last_message_at.map(|dt| dt.to_rfc3339()),
                conversation.created_at.to_rfc3339(),
                conversation.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(conversation)
    }

    async fn get_by_id(&self, id: &ChatConversationId) -> AppResult<Option<ChatConversation>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, context_type, context_id, claude_session_id, title, message_count, last_message_at, created_at, updated_at
             FROM chat_conversations WHERE id = ?1",
            [id.as_str()],
            |row| {
                let context_type_str: String = row.get("context_type")?;
                let last_message_at_str: Option<String> = row.get("last_message_at")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;

                // Parse datetimes with fallback (handles both RFC3339 and SQLite formats)
                let created_at = parse_datetime(&created_at_str);
                let updated_at = parse_datetime(&updated_at_str);

                Ok(ChatConversation {
                    id: ChatConversationId::from_string(row.get::<_, String>("id")?),
                    context_type: context_type_str.parse().unwrap_or(ChatContextType::Ideation),
                    context_id: row.get("context_id")?,
                    claude_session_id: row.get("claude_session_id")?,
                    title: row.get("title")?,
                    message_count: row.get("message_count")?,
                    last_message_at: last_message_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    created_at,
                    updated_at,
                })
            },
        );

        match result {
            Ok(conversation) => Ok(Some(conversation)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Vec<ChatConversation>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, context_type, context_id, claude_session_id, title, message_count, last_message_at, created_at, updated_at
                 FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let conversations = stmt
            .query_map([context_type.to_string(), context_id.to_string()], |row| {
                let context_type_str: String = row.get("context_type")?;
                let last_message_at_str: Option<String> = row.get("last_message_at")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;

                // Parse datetimes with fallback (handles both RFC3339 and SQLite formats)
                let created_at = parse_datetime(&created_at_str);
                let updated_at = parse_datetime(&updated_at_str);

                Ok(ChatConversation {
                    id: ChatConversationId::from_string(row.get::<_, String>("id")?),
                    context_type: context_type_str.parse().unwrap_or(ChatContextType::Ideation),
                    context_id: row.get("context_id")?,
                    claude_session_id: row.get("claude_session_id")?,
                    title: row.get("title")?,
                    message_count: row.get("message_count")?,
                    last_message_at: last_message_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    created_at,
                    updated_at,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(conversations)
    }

    async fn get_active_for_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Option<ChatConversation>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, context_type, context_id, claude_session_id, title, message_count, last_message_at, created_at, updated_at
             FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2 ORDER BY created_at DESC LIMIT 1",
            [context_type.to_string(), context_id.to_string()],
            |row| {
                let context_type_str: String = row.get("context_type")?;
                let last_message_at_str: Option<String> = row.get("last_message_at")?;
                let created_at_str: String = row.get("created_at")?;
                let updated_at_str: String = row.get("updated_at")?;

                // Parse datetimes with fallback (handles both RFC3339 and SQLite formats)
                let created_at = parse_datetime(&created_at_str);
                let updated_at = parse_datetime(&updated_at_str);

                Ok(ChatConversation {
                    id: ChatConversationId::from_string(row.get::<_, String>("id")?),
                    context_type: context_type_str.parse().unwrap_or(ChatContextType::Ideation),
                    context_id: row.get("context_id")?,
                    claude_session_id: row.get("claude_session_id")?,
                    title: row.get("title")?,
                    message_count: row.get("message_count")?,
                    last_message_at: last_message_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    created_at,
                    updated_at,
                })
            },
        );

        match result {
            Ok(conversation) => Ok(Some(conversation)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn update_claude_session_id(
        &self,
        id: &ChatConversationId,
        claude_session_id: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE chat_conversations SET claude_session_id = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![claude_session_id, Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_title(&self, id: &ChatConversationId, title: &str) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE chat_conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![title, Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_message_stats(
        &self,
        id: &ChatConversationId,
        message_count: i64,
        last_message_at: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE chat_conversations SET message_count = ?1, last_message_at = ?2, updated_at = ?3 WHERE id = ?4",
            rusqlite::params![message_count, last_message_at.to_rfc3339(), Utc::now().to_rfc3339(), id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ChatConversationId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM chat_conversations WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_conversations WHERE context_type = ?1 AND context_id = ?2",
            [context_type.to_string(), context_id.to_string()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
