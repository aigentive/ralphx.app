// Chat message repository trait - domain layer abstraction
//
// This trait defines the contract for chat message persistence.
// Messages can belong to ideation sessions, projects, or specific tasks.

use async_trait::async_trait;

use crate::domain::entities::{ChatMessage, ChatMessageId, ChatConversationId, IdeationSessionId, ProjectId, TaskId};
use crate::error::AppResult;

/// Repository trait for ChatMessage persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ChatMessageRepository: Send + Sync {
    /// Create a new chat message
    async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage>;

    /// Get message by ID
    async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>>;

    /// Get all messages for an ideation session, ordered by created_at ASC
    async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a project (not in any session), ordered by created_at ASC
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a specific task, ordered by created_at ASC
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>>;

    /// Get all messages for a specific conversation, ordered by created_at ASC
    async fn get_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<Vec<ChatMessage>>;

    /// Delete all messages for a session
    async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()>;

    /// Delete all messages for a project
    async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()>;

    /// Delete all messages for a task
    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()>;

    /// Delete a single message
    async fn delete(&self, id: &ChatMessageId) -> AppResult<()>;

    /// Count messages in a session
    async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32>;

    /// Get recent messages for a session (with limit)
    async fn get_recent_by_session(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockChatMessageRepository {
        return_message: Option<ChatMessage>,
        messages: Vec<ChatMessage>,
    }

    impl MockChatMessageRepository {
        fn new() -> Self {
            Self {
                return_message: None,
                messages: vec![],
            }
        }

        fn with_message(message: ChatMessage) -> Self {
            Self {
                return_message: Some(message.clone()),
                messages: vec![message],
            }
        }

        fn with_messages(messages: Vec<ChatMessage>) -> Self {
            Self {
                return_message: messages.first().cloned(),
                messages,
            }
        }
    }

    #[async_trait]
    impl ChatMessageRepository for MockChatMessageRepository {
        async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage> {
            Ok(message)
        }

        async fn get_by_id(&self, _id: &ChatMessageId) -> AppResult<Option<ChatMessage>> {
            Ok(self.return_message.clone())
        }

        async fn get_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<ChatMessage>> {
            let mut filtered: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .cloned()
                .collect();
            filtered.sort_by_key(|m| m.created_at);
            Ok(filtered)
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>> {
            let mut filtered: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.project_id.as_ref() == Some(project_id) && m.session_id.is_none())
                .cloned()
                .collect();
            filtered.sort_by_key(|m| m.created_at);
            Ok(filtered)
        }

        async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>> {
            let mut filtered: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.task_id.as_ref() == Some(task_id))
                .cloned()
                .collect();
            filtered.sort_by_key(|m| m.created_at);
            Ok(filtered)
        }

        async fn get_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<Vec<ChatMessage>> {
            let mut filtered: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.conversation_id.as_ref() == Some(conversation_id))
                .cloned()
                .collect();
            filtered.sort_by_key(|m| m.created_at);
            Ok(filtered)
        }

        async fn delete_by_session(&self, _session_id: &IdeationSessionId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_project(&self, _project_id: &ProjectId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_task(&self, _task_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ChatMessageId) -> AppResult<()> {
            Ok(())
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .messages
                .iter()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .count() as u32)
        }

        async fn get_recent_by_session(
            &self,
            session_id: &IdeationSessionId,
            limit: u32,
        ) -> AppResult<Vec<ChatMessage>> {
            let mut filtered: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .cloned()
                .collect();
            filtered.sort_by_key(|m| std::cmp::Reverse(m.created_at));
            filtered.truncate(limit as usize);
            filtered.reverse(); // Return in ascending order
            Ok(filtered)
        }
    }

    fn create_test_message_in_session(session_id: &IdeationSessionId) -> ChatMessage {
        ChatMessage::user_in_session(session_id.clone(), "Test message")
    }

    fn create_test_message_in_project(project_id: &ProjectId) -> ChatMessage {
        ChatMessage::user_in_project(project_id.clone(), "Test message")
    }

    fn create_test_message_about_task(task_id: &TaskId) -> ChatMessage {
        ChatMessage::user_about_task(task_id.clone(), "Test message")
    }

    #[test]
    fn test_chat_message_repository_trait_can_be_object_safe() {
        // Verify that ChatMessageRepository can be used as a trait object
        let repo: Arc<dyn ChatMessageRepository> = Arc::new(MockChatMessageRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create() {
        let repo = MockChatMessageRepository::new();
        let session_id = IdeationSessionId::new();
        let message = create_test_message_in_session(&session_id);

        let result = repo.create(message.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, message.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_none() {
        let repo = MockChatMessageRepository::new();
        let message_id = ChatMessageId::new();

        let result = repo.get_by_id(&message_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_id_returns_message() {
        let session_id = IdeationSessionId::new();
        let message = create_test_message_in_session(&session_id);
        let repo = MockChatMessageRepository::with_message(message.clone());

        let result = repo.get_by_id(&message.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, message.id);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_empty() {
        let repo = MockChatMessageRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.get_by_session(&session_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_with_messages() {
        let session_id = IdeationSessionId::new();
        let message1 = create_test_message_in_session(&session_id);
        let message2 = create_test_message_in_session(&session_id);

        let repo = MockChatMessageRepository::with_messages(vec![message1.clone(), message2.clone()]);

        let result = repo.get_by_session(&session_id).await;
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_session_filters_by_session() {
        let session_id1 = IdeationSessionId::new();
        let session_id2 = IdeationSessionId::new();
        let message1 = create_test_message_in_session(&session_id1);
        let message2 = create_test_message_in_session(&session_id2);

        let repo = MockChatMessageRepository::with_messages(vec![message1.clone(), message2.clone()]);

        let result = repo.get_by_session(&session_id1).await;
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].session_id, Some(session_id1));
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_project_empty() {
        let repo = MockChatMessageRepository::new();
        let project_id = ProjectId::new();

        let result = repo.get_by_project(&project_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_project_with_messages() {
        let project_id = ProjectId::new();
        let message1 = create_test_message_in_project(&project_id);
        let message2 = create_test_message_in_project(&project_id);

        let repo = MockChatMessageRepository::with_messages(vec![message1.clone(), message2.clone()]);

        let result = repo.get_by_project(&project_id).await;
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_task_empty() {
        let repo = MockChatMessageRepository::new();
        let task_id = TaskId::new();

        let result = repo.get_by_task(&task_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_task_with_messages() {
        let task_id = TaskId::new();
        let message1 = create_test_message_about_task(&task_id);
        let message2 = create_test_message_about_task(&task_id);

        let repo = MockChatMessageRepository::with_messages(vec![message1.clone(), message2.clone()]);

        let result = repo.get_by_task(&task_id).await;
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_delete_by_session() {
        let repo = MockChatMessageRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.delete_by_session(&session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete_by_project() {
        let repo = MockChatMessageRepository::new();
        let project_id = ProjectId::new();

        let result = repo.delete_by_project(&project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete_by_task() {
        let repo = MockChatMessageRepository::new();
        let task_id = TaskId::new();

        let result = repo.delete_by_task(&task_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockChatMessageRepository::new();
        let message_id = ChatMessageId::new();

        let result = repo.delete(&message_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_session_zero() {
        let repo = MockChatMessageRepository::new();
        let session_id = IdeationSessionId::new();

        let result = repo.count_by_session(&session_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_repository_count_by_session_counts_correctly() {
        let session_id = IdeationSessionId::new();
        let other_session_id = IdeationSessionId::new();
        let message1 = create_test_message_in_session(&session_id);
        let message2 = create_test_message_in_session(&session_id);
        let message3 = create_test_message_in_session(&other_session_id);

        let repo = MockChatMessageRepository::with_messages(vec![
            message1.clone(),
            message2.clone(),
            message3.clone(),
        ]);

        let result = repo.count_by_session(&session_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mock_repository_get_recent_by_session() {
        let session_id = IdeationSessionId::new();
        let message1 = create_test_message_in_session(&session_id);
        let message2 = create_test_message_in_session(&session_id);
        let message3 = create_test_message_in_session(&session_id);

        let repo = MockChatMessageRepository::with_messages(vec![
            message1.clone(),
            message2.clone(),
            message3.clone(),
        ]);

        let result = repo.get_recent_by_session(&session_id, 2).await;
        assert!(result.is_ok());
        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_repository_trait_object_in_arc() {
        let session_id = IdeationSessionId::new();
        let message = create_test_message_in_session(&session_id);
        let repo: Arc<dyn ChatMessageRepository> =
            Arc::new(MockChatMessageRepository::with_message(message.clone()));

        // Use through trait object
        let result = repo.get_by_id(&message.id).await;
        assert!(result.is_ok());

        let all = repo.get_by_session(&session_id).await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_repository_trait_object_delete_operations() {
        let session_id = IdeationSessionId::new();
        let message = create_test_message_in_session(&session_id);
        let repo: Arc<dyn ChatMessageRepository> =
            Arc::new(MockChatMessageRepository::with_message(message.clone()));

        let result = repo.delete_by_session(&session_id).await;
        assert!(result.is_ok());

        let result = repo.delete(&message.id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_repository_trait_object_count_and_recent() {
        let session_id = IdeationSessionId::new();
        let message = create_test_message_in_session(&session_id);
        let repo: Arc<dyn ChatMessageRepository> =
            Arc::new(MockChatMessageRepository::with_message(message.clone()));

        let count = repo.count_by_session(&session_id).await;
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 1);

        let recent = repo.get_recent_by_session(&session_id, 10).await;
        assert!(recent.is_ok());
        assert_eq!(recent.unwrap().len(), 1);
    }
}
