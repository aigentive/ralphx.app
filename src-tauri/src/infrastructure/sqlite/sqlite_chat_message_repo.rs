// SQLite-based ChatMessageRepository implementation for production use
// Uses DbConnection for non-blocking SQLite access via spawn_blocking

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::entities::{
    ChatConversationId, ChatMessage, ChatMessageId, IdeationSessionId, ProjectId, TaskId,
};
use crate::domain::repositories::ChatMessageRepository;
use crate::error::AppResult;
use crate::infrastructure::sqlite::DbConnection;

/// SQLite implementation of ChatMessageRepository for production use
pub struct SqliteChatMessageRepository {
    pub(crate) db: DbConnection,
}

impl SqliteChatMessageRepository {
    /// Create a new SQLite chat message repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl ChatMessageRepository for SqliteChatMessageRepository {
    async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage> {
        let id = message.id.as_str().to_string();
        let session_id = message.session_id.as_ref().map(|id| id.as_str().to_string());
        let project_id = message.project_id.as_ref().map(|id| id.as_str().to_string());
        let task_id = message.task_id.as_ref().map(|id| id.as_str().to_string());
        let conversation_id = message.conversation_id.as_ref().map(|id| id.as_str().to_string());
        let role = message.role.to_string();
        let content = message.content.clone();
        let metadata = message.metadata.clone();
        let parent_message_id = message.parent_message_id.as_ref().map(|id| id.as_str().to_string());
        let tool_calls = message.tool_calls.clone();
        let content_blocks = message.content_blocks.clone();
        let created_at = message.created_at.to_rfc3339();

        self.db.run(move |conn| {
            conn.execute(
                "INSERT INTO chat_messages (id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    id, session_id, project_id, task_id, conversation_id,
                    role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at,
                ],
            )?;
            Ok(())
        }).await?;

        Ok(message)
    }

    async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>> {
        let id_str = id.as_str().to_string();
        self.db.query_optional(move |conn| {
            conn.query_row(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE id = ?1",
                [&id_str],
                ChatMessage::from_row,
            )
        }).await
    }

    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>> {
        let session_id_str = session_id.as_str().to_string();
        self.db.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC, rowid ASC",
            )?;
            let messages = stmt
                .query_map([session_id_str], ChatMessage::from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        }).await
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>> {
        let project_id_str = project_id.as_str().to_string();
        self.db.run(move |conn| {
            // Get messages that belong to a project but NOT to a session (direct project chat)
            let mut stmt = conn.prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE project_id = ?1 AND session_id IS NULL ORDER BY created_at ASC, rowid ASC",
            )?;
            let messages = stmt
                .query_map([project_id_str], ChatMessage::from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        }).await
    }

    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>> {
        let task_id_str = task_id.as_str().to_string();
        self.db.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE task_id = ?1 ORDER BY created_at ASC, rowid ASC",
            )?;
            let messages = stmt
                .query_map([task_id_str], ChatMessage::from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        }).await
    }

    async fn get_by_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatMessage>> {
        let conv_id_str = conversation_id.as_str().to_string();
        self.db.run(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE conversation_id = ?1 ORDER BY created_at ASC, rowid ASC",
            )?;
            let messages = stmt
                .query_map([conv_id_str], ChatMessage::from_row)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(messages)
        }).await
    }

    async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        let session_id_str = session_id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute("DELETE FROM chat_messages WHERE session_id = ?1", [session_id_str])?;
            Ok(())
        }).await
    }

    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()> {
        let project_id_str = project_id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute("DELETE FROM chat_messages WHERE project_id = ?1", [project_id_str])?;
            Ok(())
        }).await
    }

    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
        let task_id_str = task_id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute("DELETE FROM chat_messages WHERE task_id = ?1", [task_id_str])?;
            Ok(())
        }).await
    }

    async fn delete(&self, id: &ChatMessageId) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        self.db.run(move |conn| {
            conn.execute("DELETE FROM chat_messages WHERE id = ?1", [id_str])?;
            Ok(())
        }).await
    }

    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let session_id_str = session_id.as_str().to_string();
        self.db.run(move |conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1",
                [session_id_str],
                |row| row.get(0),
            )?;
            Ok(count as u32)
        }).await
    }

    async fn get_recent_by_session(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>> {
        let session_id_str = session_id.as_str().to_string();
        self.db.run(move |conn| {
            // Get the most recent messages, but return them in ascending order
            let mut stmt = conn.prepare(
                "SELECT id, session_id, project_id, task_id, conversation_id, role, content, metadata, parent_message_id, tool_calls, content_blocks, created_at
                 FROM chat_messages WHERE session_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT ?2",
            )?;
            let mut messages: Vec<ChatMessage> = stmt
                .query_map(rusqlite::params![session_id_str, limit], |row| {
                    ChatMessage::from_row(row)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            // Reverse to return in ascending order (oldest to newest)
            messages.reverse();
            Ok(messages)
        }).await
    }

    async fn update_content(
        &self,
        id: &ChatMessageId,
        content: &str,
        tool_calls: Option<&str>,
        content_blocks: Option<&str>,
    ) -> AppResult<()> {
        let id_str = id.as_str().to_string();
        let content = content.to_string();
        let tool_calls = tool_calls.map(|s| s.to_string());
        let content_blocks = content_blocks.map(|s| s.to_string());
        self.db.run(move |conn| {
            conn.execute(
                "UPDATE chat_messages SET content = ?1, tool_calls = ?2, content_blocks = ?3 WHERE id = ?4",
                rusqlite::params![content, tool_calls, content_blocks, id_str],
            )?;
            Ok(())
        }).await
    }
}
