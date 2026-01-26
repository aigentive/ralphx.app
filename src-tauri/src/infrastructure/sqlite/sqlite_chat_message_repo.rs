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
    conn: Arc<Mutex<Connection>>,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::IdeationSession;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    fn setup_test_db() -> Connection {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn create_test_project(conn: &Connection, id: &ProjectId, name: &str, path: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'single_branch', datetime('now'), datetime('now'))",
            rusqlite::params![id.as_str(), name, path],
        )
        .unwrap();
    }

    fn create_test_session(conn: &Connection, project_id: &ProjectId) -> IdeationSessionId {
        let session = IdeationSession::builder()
            .project_id(project_id.clone())
            .title("Test Session")
            .build();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', datetime('now'), datetime('now'))",
            rusqlite::params![
                session.id.as_str(),
                project_id.as_str(),
                "Test Session"
            ],
        )
        .unwrap();

        session.id
    }

    fn create_test_task(conn: &Connection, project_id: &ProjectId) -> TaskId {
        let task_id = TaskId::new();
        conn.execute(
            "INSERT INTO tasks (id, project_id, title, category, internal_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'feature', 'draft', datetime('now'), datetime('now'))",
            rusqlite::params![task_id.as_str(), project_id.as_str(), "Test Task"],
        )
        .unwrap();
        task_id
    }

    // ==================== CREATE TESTS ====================

    #[tokio::test]
    async fn test_create_inserts_message_and_returns_it() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello, world!");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.id, message.id);
        assert_eq!(created.content, "Hello, world!");
        assert_eq!(created.session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_create_message_with_metadata() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id, "With metadata")
            .with_metadata(r#"{"key": "value"}"#);

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.metadata, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[tokio::test]
    async fn test_create_message_with_parent() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create parent message
        let parent = ChatMessage::user_in_session(session_id.clone(), "Parent message");
        repo.create(parent.clone()).await.unwrap();

        // Create child message
        let child = ChatMessage::orchestrator_in_session(session_id, "Reply to parent")
            .with_parent(parent.id.clone());
        let result = repo.create(child.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.parent_message_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_create_duplicate_id_fails() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id, "Duplicate");

        repo.create(message.clone()).await.unwrap();
        let result = repo.create(message).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_project_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_project(project_id.clone(), "Project-level chat");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.project_id, Some(project_id));
        assert!(created.session_id.is_none());
    }

    #[tokio::test]
    async fn test_create_task_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_about_task(task_id.clone(), "Task-specific chat");

        let result = repo.create(message.clone()).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.task_id, Some(task_id));
    }

    // ==================== GET BY ID TESTS ====================

    #[tokio::test]
    async fn test_get_by_id_retrieves_message_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::user_in_session(session_id.clone(), "Find me");

        repo.create(message.clone()).await.unwrap();
        let result = repo.get_by_id(&message.id).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let found_message = found.unwrap();
        assert_eq!(found_message.id, message.id);
        assert_eq!(found_message.content, "Find me");
        assert_eq!(found_message.session_id, Some(session_id));
    }

    #[tokio::test]
    async fn test_get_by_id_returns_none_for_nonexistent() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let id = ChatMessageId::new();

        let result = repo.get_by_id(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_preserves_all_fields() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);
        let message = ChatMessage::orchestrator_in_session(session_id.clone(), "Full message")
            .with_metadata(r#"{"context": "test"}"#);

        repo.create(message.clone()).await.unwrap();
        let found = repo.get_by_id(&message.id).await.unwrap().unwrap();

        assert_eq!(found.id, message.id);
        assert_eq!(found.session_id, Some(session_id));
        assert_eq!(found.content, "Full message");
        assert_eq!(found.metadata, Some(r#"{"context": "test"}"#.to_string()));
        assert!(found.is_orchestrator());
    }

    // ==================== GET BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_by_session_returns_all_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_get_by_session_ordered_by_created_at_asc() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create messages with slight delays to ensure different timestamps
        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        repo.create(msg1.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        repo.create(msg2.clone()).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");
        repo.create(msg3.clone()).await.unwrap();

        let messages = repo.get_by_session(&session_id).await.unwrap();

        // Should be in ascending order (oldest first)
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    #[tokio::test]
    async fn test_get_by_session_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_session(&session_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "Session 1 message");
        let msg2 = ChatMessage::user_in_session(session_id2.clone(), "Session 2 message");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_session(&session_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].session_id, Some(session_id1));
    }

    // ==================== GET BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_get_by_project_returns_project_messages_only() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create a project message (no session)
        let project_msg = ChatMessage::user_in_project(project_id.clone(), "Project chat");
        // Create a session message
        let session_msg = ChatMessage::user_in_session(session_id.clone(), "Session chat");

        repo.create(project_msg).await.unwrap();
        repo.create(session_msg).await.unwrap();

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        // Should only return the project message, not the session message
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Project chat");
        assert!(messages[0].session_id.is_none());
    }

    #[tokio::test]
    async fn test_get_by_project_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_project(&project_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_project_filters_by_project() {
        let conn = setup_test_db();
        let project_id1 = ProjectId::new();
        let project_id2 = ProjectId::new();
        create_test_project(&conn, &project_id1, "Project 1", "/path1");
        create_test_project(&conn, &project_id2, "Project 2", "/path2");

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_project(project_id1.clone(), "P1 message");
        let msg2 = ChatMessage::user_in_project(project_id2.clone(), "P2 message");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_project(&project_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].project_id, Some(project_id1));
    }

    // ==================== GET BY TASK TESTS ====================

    #[tokio::test]
    async fn test_get_by_task_returns_task_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id.clone(), "Task question");
        let msg2 = ChatMessage::user_about_task(task_id.clone(), "Follow-up");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let result = repo.get_by_task(&task_id).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_task_returns_empty_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.get_by_task(&task_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_by_task_filters_by_task() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id1 = create_test_task(&conn, &project_id);
        let task_id2 = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id1.clone(), "Task 1 msg");
        let msg2 = ChatMessage::user_about_task(task_id2.clone(), "Task 2 msg");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let messages = repo.get_by_task(&task_id1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].task_id, Some(task_id1));
    }

    // ==================== DELETE BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_delete_by_session_removes_all_session_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Message 1");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_session(&session_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_session(&session_id).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_delete_by_session_does_not_affect_other_sessions() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "Session 1");
        let msg2 = ChatMessage::user_in_session(session_id2.clone(), "Session 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        repo.delete_by_session(&session_id1).await.unwrap();

        let session1_messages = repo.get_by_session(&session_id1).await.unwrap();
        let session2_messages = repo.get_by_session(&session_id2).await.unwrap();

        assert!(session1_messages.is_empty());
        assert_eq!(session2_messages.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_by_session_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let session_id = IdeationSessionId::new();

        let result = repo.delete_by_session(&session_id).await;
        assert!(result.is_ok());
    }

    // ==================== DELETE BY PROJECT TESTS ====================

    #[tokio::test]
    async fn test_delete_by_project_removes_all_project_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_project(project_id.clone(), "Message 1");
        let msg2 = ChatMessage::user_in_project(project_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_project(&project_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_project(&project_id).await.unwrap();
        assert!(messages.is_empty());
    }

    // ==================== DELETE BY TASK TESTS ====================

    #[tokio::test]
    async fn test_delete_by_task_removes_all_task_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let task_id = create_test_task(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_about_task(task_id.clone(), "Message 1");
        let msg2 = ChatMessage::user_about_task(task_id.clone(), "Message 2");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();

        let delete_result = repo.delete_by_task(&task_id).await;
        assert!(delete_result.is_ok());

        let messages = repo.get_by_task(&task_id).await.unwrap();
        assert!(messages.is_empty());
    }

    // ==================== DELETE SINGLE MESSAGE TESTS ====================

    #[tokio::test]
    async fn test_delete_removes_single_message() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Keep");
        let msg2 = ChatMessage::user_in_session(session_id.clone(), "Delete");

        repo.create(msg1.clone()).await.unwrap();
        repo.create(msg2.clone()).await.unwrap();

        let delete_result = repo.delete(&msg2.id).await;
        assert!(delete_result.is_ok());

        let found = repo.get_by_id(&msg2.id).await.unwrap();
        assert!(found.is_none());

        // Other message should still exist
        let kept = repo.get_by_id(&msg1.id).await.unwrap();
        assert!(kept.is_some());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_succeeds() {
        let conn = setup_test_db();
        let repo = SqliteChatMessageRepository::new(conn);
        let id = ChatMessageId::new();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    // ==================== COUNT BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_count_by_session_returns_zero_for_no_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let result = repo.count_by_session(&session_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_by_session_counts_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "One");
        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Two");
        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Three");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let result = repo.count_by_session(&session_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_count_by_session_filters_by_session() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id1 = create_test_session(&conn, &project_id);
        let session_id2 = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id1.clone(), "S1-1");
        let msg2 = ChatMessage::user_in_session(session_id1.clone(), "S1-2");
        let msg3 = ChatMessage::user_in_session(session_id2.clone(), "S2-1");

        repo.create(msg1).await.unwrap();
        repo.create(msg2).await.unwrap();
        repo.create(msg3).await.unwrap();

        let count1 = repo.count_by_session(&session_id1).await.unwrap();
        let count2 = repo.count_by_session(&session_id2).await.unwrap();

        assert_eq!(count1, 2);
        assert_eq!(count2, 1);
    }

    // ==================== GET RECENT BY SESSION TESTS ====================

    #[tokio::test]
    async fn test_get_recent_by_session_returns_limited_messages() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create 5 messages
        for i in 1..=5 {
            let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
            repo.create(msg).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let result = repo.get_recent_by_session(&session_id, 3).await;

        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_most_recent() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        // Create messages with delays
        for i in 1..=5 {
            let msg = ChatMessage::user_in_session(session_id.clone(), format!("Message {}", i));
            repo.create(msg).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let messages = repo.get_recent_by_session(&session_id, 2).await.unwrap();

        // Should return the last 2 messages in ascending order
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Message 4");
        assert_eq!(messages[1].content, "Message 5");
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_all_if_fewer_than_limit() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "Only one");
        repo.create(msg1).await.unwrap();

        let messages = repo.get_recent_by_session(&session_id, 10).await.unwrap();

        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_get_recent_by_session_returns_in_ascending_order() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg1 = ChatMessage::user_in_session(session_id.clone(), "First");
        repo.create(msg1).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let msg2 = ChatMessage::orchestrator_in_session(session_id.clone(), "Second");
        repo.create(msg2).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let msg3 = ChatMessage::user_in_session(session_id.clone(), "Third");
        repo.create(msg3).await.unwrap();

        let messages = repo.get_recent_by_session(&session_id, 3).await.unwrap();

        // Should be in ascending order (oldest to newest)
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    // ==================== SHARED CONNECTION TESTS ====================

    #[tokio::test]
    async fn test_from_shared_works_correctly() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let shared_conn = Arc::new(Mutex::new(conn));
        let repo = SqliteChatMessageRepository::from_shared(shared_conn);

        let message = ChatMessage::user_in_session(session_id, "Shared connection test");

        let result = repo.create(message.clone()).await;
        assert!(result.is_ok());

        let found = repo.get_by_id(&message.id).await.unwrap();
        assert!(found.is_some());
    }

    // ==================== ROLE TESTS ====================

    #[tokio::test]
    async fn test_message_roles_are_preserved() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let user_msg = ChatMessage::user_in_session(session_id.clone(), "User");
        let orch_msg = ChatMessage::orchestrator_in_session(session_id.clone(), "Orchestrator");
        let sys_msg = ChatMessage::system_in_session(session_id.clone(), "System");

        repo.create(user_msg.clone()).await.unwrap();
        repo.create(orch_msg.clone()).await.unwrap();
        repo.create(sys_msg.clone()).await.unwrap();

        let found_user = repo.get_by_id(&user_msg.id).await.unwrap().unwrap();
        let found_orch = repo.get_by_id(&orch_msg.id).await.unwrap().unwrap();
        let found_sys = repo.get_by_id(&sys_msg.id).await.unwrap().unwrap();

        assert!(found_user.is_user());
        assert!(found_orch.is_orchestrator());
        assert!(found_sys.is_system());
    }

    // ==================== CASCADE DELETE TESTS ====================

    #[tokio::test]
    async fn test_cascade_delete_when_session_deleted() {
        let conn = setup_test_db();
        let project_id = ProjectId::new();
        create_test_project(&conn, &project_id, "Test Project", "/test/path");
        let session_id = create_test_session(&conn, &project_id);

        let repo = SqliteChatMessageRepository::new(conn);

        let msg = ChatMessage::user_in_session(session_id.clone(), "Will be cascaded");
        repo.create(msg.clone()).await.unwrap();

        // Delete the session directly
        {
            let conn = repo.conn.lock().await;
            conn.execute(
                "DELETE FROM ideation_sessions WHERE id = ?1",
                [session_id.as_str()],
            )
            .unwrap();
        }

        // Message should be gone due to CASCADE
        let found = repo.get_by_id(&msg.id).await.unwrap();
        assert!(found.is_none());
    }
}
