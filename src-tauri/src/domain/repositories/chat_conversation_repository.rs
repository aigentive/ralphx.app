// Chat conversation repository trait - domain layer abstraction
//
// This trait defines the contract for chat conversation persistence.
// Conversations track Claude CLI sessions linked to contexts (ideation, task, project).

use async_trait::async_trait;

use crate::domain::entities::{ChatContextType, ChatConversation, ChatConversationId};
use crate::error::AppResult;

/// Repository trait for ChatConversation persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ChatConversationRepository: Send + Sync {
    /// Create a new conversation
    async fn create(&self, conversation: ChatConversation) -> AppResult<ChatConversation>;

    /// Get conversation by ID
    async fn get_by_id(&self, id: &ChatConversationId) -> AppResult<Option<ChatConversation>>;

    /// Get all conversations for a specific context
    async fn get_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Vec<ChatConversation>>;

    /// Get the active (most recent) conversation for a context
    async fn get_active_for_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Option<ChatConversation>>;

    /// Update the Claude session ID for a conversation
    async fn update_claude_session_id(
        &self,
        id: &ChatConversationId,
        claude_session_id: &str,
    ) -> AppResult<()>;

    /// Clear the Claude session ID for a conversation
    async fn clear_claude_session_id(&self, id: &ChatConversationId) -> AppResult<()>;

    /// Update conversation title
    async fn update_title(&self, id: &ChatConversationId, title: &str) -> AppResult<()>;

    /// Update message count and last message timestamp
    /// This is typically called by a database trigger, but can be manually updated if needed
    async fn update_message_stats(
        &self,
        id: &ChatConversationId,
        message_count: i64,
        last_message_at: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<()>;

    /// Delete a conversation and all its messages
    async fn delete(&self, id: &ChatConversationId) -> AppResult<()>;

    /// Delete all conversations for a context
    async fn delete_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ChatContextType, ChatConversation, IdeationSessionId};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockChatConversationRepository {
        conversations: Vec<ChatConversation>,
    }

    impl MockChatConversationRepository {
        fn new() -> Self {
            Self {
                conversations: vec![],
            }
        }

        fn with_conversations(conversations: Vec<ChatConversation>) -> Self {
            Self { conversations }
        }
    }

    #[async_trait]
    impl ChatConversationRepository for MockChatConversationRepository {
        async fn create(&self, conversation: ChatConversation) -> AppResult<ChatConversation> {
            Ok(conversation)
        }

        async fn get_by_id(&self, id: &ChatConversationId) -> AppResult<Option<ChatConversation>> {
            Ok(self.conversations.iter().find(|c| c.id == *id).cloned())
        }

        async fn get_by_context(
            &self,
            context_type: ChatContextType,
            context_id: &str,
        ) -> AppResult<Vec<ChatConversation>> {
            Ok(self
                .conversations
                .iter()
                .filter(|c| c.context_type == context_type && c.context_id == context_id)
                .cloned()
                .collect())
        }

        async fn get_active_for_context(
            &self,
            context_type: ChatContextType,
            context_id: &str,
        ) -> AppResult<Option<ChatConversation>> {
            Ok(self
                .conversations
                .iter()
                .filter(|c| c.context_type == context_type && c.context_id == context_id)
                .max_by_key(|c| c.created_at)
                .cloned())
        }

        async fn update_claude_session_id(
            &self,
            _id: &ChatConversationId,
            _claude_session_id: &str,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn clear_claude_session_id(&self, _id: &ChatConversationId) -> AppResult<()> {
            Ok(())
        }

        async fn update_title(&self, _id: &ChatConversationId, _title: &str) -> AppResult<()> {
            Ok(())
        }

        async fn update_message_stats(
            &self,
            _id: &ChatConversationId,
            _message_count: i64,
            _last_message_at: chrono::DateTime<chrono::Utc>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ChatConversationId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_context(
            &self,
            _context_type: ChatContextType,
            _context_id: &str,
        ) -> AppResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_trait_object_safety() {
        let repo = MockChatConversationRepository::new();
        let _: Arc<dyn ChatConversationRepository> = Arc::new(repo);
    }

    #[test]
    fn test_mock_with_conversations() {
        let session_id = IdeationSessionId::new();
        let conv = ChatConversation::new_ideation(session_id);
        let repo = MockChatConversationRepository::with_conversations(vec![conv.clone()]);

        assert_eq!(repo.conversations.len(), 1);
        assert_eq!(repo.conversations[0].id, conv.id);
    }
}
