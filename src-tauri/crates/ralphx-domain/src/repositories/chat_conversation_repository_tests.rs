use super::*;
use crate::agents::ProviderSessionRef;
use crate::domain::entities::{
    ChatContextType, ChatConversation, ConversationAttributionBackfillState, IdeationSessionId,
};
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

    async fn update_provider_session_ref(
        &self,
        _id: &ChatConversationId,
        _session_ref: &ProviderSessionRef,
    ) -> AppResult<()> {
        Ok(())
    }

    async fn clear_provider_session_ref(&self, _id: &ChatConversationId) -> AppResult<()> {
        Ok(())
    }

    async fn update_provider_origin(
        &self,
        _id: &ChatConversationId,
        _upstream_provider: Option<&str>,
        _provider_profile: Option<&str>,
    ) -> AppResult<()> {
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

    async fn list_needing_attribution_backfill(
        &self,
        _limit: u32,
    ) -> AppResult<Vec<ChatConversation>> {
        Ok(self.conversations.clone())
    }

    async fn update_attribution_backfill_state(
        &self,
        _id: &ChatConversationId,
        _state: ConversationAttributionBackfillState,
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
