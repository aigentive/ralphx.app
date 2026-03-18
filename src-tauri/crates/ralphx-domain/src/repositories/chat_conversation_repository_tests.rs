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
