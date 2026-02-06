// SQLite-based ChatMessageRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{ChatMessage, ChatMessageId, ChatConversationId, IdeationSessionId, ProjectId, TaskId};
use crate::domain::repositories::ChatMessageRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of ChatMessageRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteChatMessageRepository {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl SqliteChatMessageRepository {
    /// Create a new SQLite chat message repository with the given connection
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
impl ChatMessageRepository for SqliteChatMessageRepository {
    async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO chat_messages (id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                message.id.as_str(),
                message.session_id.as_ref().map(|id| id.as_str()),
                message.project_id.as_ref().map(|id| id.as_str()),
                message.task_id.as_ref().map(|id| id.as_str()),
                message.conversation_id.as_ref().map(|id| id.as_str()),
                message.role.to_string(),
                message.content,
                message.metadata,
                message.parent_message_id.as_ref().map(|id| id.as_str()),
                message.tool_calls,
                message.content_blocks,
                message.created_at.to_rfc3339(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(message)
    }

    async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
             FROM chat_messages WHERE id = ?1",
            [id.as_str()],
            ChatMessage::from_row,
        );

        match result {
            Ok(message) => Ok(Some(message)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let messages = stmt
            .query_map([session_id.as_str()], ChatMessage::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(messages)
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().await;

        // Get messages that belong to a project but NOT to a session (direct project chat)
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE project_id = ?1 AND session_id IS NULL ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let messages = stmt
            .query_map([project_id.as_str()], ChatMessage::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(messages)
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE task_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let messages = stmt
            .query_map([task_id.as_str()], ChatMessage::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(messages)
    }

    async fn get_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let messages = stmt
            .query_map([conversation_id.as_str()], ChatMessage::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(messages)
    }

    async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_messages WHERE session_id = ?1",
            [session_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_messages WHERE project_id = ?1",
            [project_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM chat_messages WHERE task_id = ?1",
            [task_id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &ChatMessageId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM chat_messages WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let conn = self.conn.lock().await;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1",
                [session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    async fn get_recent_by_session(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>> {
        let conn = self.conn.lock().await;

        // Get the most recent messages, but return them in ascending order
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut messages: Vec<ChatMessage> = stmt
            .query_map(rusqlite::params![session_id.as_str(), limit], |row| {
                ChatMessage::from_row(row)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Reverse to return in ascending order (oldest to newest)
        messages.reverse();

        Ok(messages)
    }

    async fn update_content(
        &self,
        id: &ChatMessageId,
        content: &str,
        tool_calls: Option<&str>,
        content_blocks: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE chat_messages SET content = ?1, tool_calls = ?2, content_blocks = ?3 WHERE id = ?4",
            rusqlite::params![content, tool_calls, content_blocks, id.as_str()],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
