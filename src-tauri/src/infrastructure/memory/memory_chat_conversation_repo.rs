// In-memory implementation of ChatConversationRepository for testing

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::domain::entities::{ChatContextType, ChatConversation, ChatConversationId};
use crate::domain::repositories::ChatConversationRepository;
use crate::error::AppResult;

/// In-memory implementation of ChatConversationRepository for testing
pub struct MemoryChatConversationRepository {
    conversations: RwLock<HashMap<ChatConversationId, ChatConversation>>,
}

impl MemoryChatConversationRepository {
    pub fn new() -> Self {
        Self {
            conversations: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryChatConversationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatConversationRepository for MemoryChatConversationRepository {
    async fn create(&self, conversation: ChatConversation) -> AppResult<ChatConversation> {
        let mut convos = self.conversations.write().await;
        convos.insert(conversation.id, conversation.clone());
        Ok(conversation)
    }

    async fn get_by_id(&self, id: &ChatConversationId) -> AppResult<Option<ChatConversation>> {
        let convos = self.conversations.read().await;
        Ok(convos.get(id).cloned())
    }

    async fn get_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Vec<ChatConversation>> {
        let convos = self.conversations.read().await;
        let filtered: Vec<ChatConversation> = convos
            .values()
            .filter(|c| c.context_type == context_type && c.context_id == context_id)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn get_active_for_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<Option<ChatConversation>> {
        let convos = self.conversations.read().await;
        Ok(convos
            .values()
            .filter(|c| c.context_type == context_type && c.context_id == context_id)
            .max_by_key(|c| c.created_at)
            .cloned())
    }

    async fn update_claude_session_id(
        &self,
        id: &ChatConversationId,
        claude_session_id: &str,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conv) = convos.get_mut(id) {
            conv.claude_session_id = Some(claude_session_id.to_string());
            conv.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_title(&self, id: &ChatConversationId, title: &str) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conv) = convos.get_mut(id) {
            conv.title = Some(title.to_string());
            conv.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_message_stats(
        &self,
        id: &ChatConversationId,
        message_count: i64,
        last_message_at: DateTime<Utc>,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conv) = convos.get_mut(id) {
            conv.message_count = message_count;
            conv.last_message_at = Some(last_message_at);
            conv.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, id: &ChatConversationId) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        convos.remove(id);
        Ok(())
    }

    async fn delete_by_context(
        &self,
        context_type: ChatContextType,
        context_id: &str,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        convos.retain(|_, c| !(c.context_type == context_type && c.context_id == context_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::IdeationSessionId;

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryChatConversationRepository::new();
        let session_id = IdeationSessionId::new();
        let conv = ChatConversation::new_ideation(session_id);
        let id = conv.id;

        repo.create(conv.clone()).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    async fn test_get_by_context() {
        let repo = MemoryChatConversationRepository::new();
        let session_id = IdeationSessionId::new();
        let conv = ChatConversation::new_ideation(session_id.clone());

        repo.create(conv.clone()).await.unwrap();

        let convos = repo
            .get_by_context(ChatContextType::Ideation, session_id.as_str())
            .await
            .unwrap();
        assert_eq!(convos.len(), 1);
    }

    #[tokio::test]
    async fn test_update_claude_session_id() {
        let repo = MemoryChatConversationRepository::new();
        let session_id = IdeationSessionId::new();
        let conv = ChatConversation::new_ideation(session_id);
        let id = conv.id;

        repo.create(conv).await.unwrap();
        repo.update_claude_session_id(&id, "test-session-123")
            .await
            .unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(
            retrieved.claude_session_id,
            Some("test-session-123".to_string())
        );
    }
}
