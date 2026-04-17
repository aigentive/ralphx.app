// In-memory implementation of ChatConversationRepository for testing

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::domain::agents::ProviderSessionRef;
use crate::domain::entities::{
    AttributionBackfillStatus, ChatContextType, ChatConversation, ChatConversationId,
    ConversationAttributionBackfillState, ConversationAttributionBackfillSummary,
};
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

    async fn update_provider_session_ref(
        &self,
        id: &ChatConversationId,
        session_ref: &ProviderSessionRef,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conv) = convos.get_mut(id) {
            conv.set_provider_session_ref(session_ref.clone());
        }
        Ok(())
    }

    async fn clear_provider_session_ref(&self, id: &ChatConversationId) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conversation) = convos.get_mut(id) {
            conversation.clear_provider_session_ref();
        }
        Ok(())
    }

    async fn update_provider_origin(
        &self,
        id: &ChatConversationId,
        upstream_provider: Option<&str>,
        provider_profile: Option<&str>,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conversation) = convos.get_mut(id) {
            conversation.set_provider_origin(
                upstream_provider.map(str::to_string),
                provider_profile.map(str::to_string),
            );
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

    async fn list_needing_attribution_backfill(
        &self,
        limit: u32,
    ) -> AppResult<Vec<ChatConversation>> {
        let convos = self.conversations.read().await;
        let mut filtered: Vec<ChatConversation> = convos
            .values()
            .filter(|conversation| {
                conversation.claude_session_id.is_some()
                    && matches!(
                        conversation.attribution_backfill_status,
                        None
                            | Some(crate::domain::entities::AttributionBackfillStatus::Pending)
                    )
            })
            .cloned()
            .collect();
        filtered.sort_by_key(|conversation| {
            conversation
                .attribution_backfill_last_attempted_at
                .unwrap_or(conversation.created_at)
        });
        filtered.truncate(limit as usize);
        Ok(filtered)
    }

    async fn reset_running_attribution_backfill_to_pending(&self) -> AppResult<u64> {
        let mut convos = self.conversations.write().await;
        let mut updated = 0u64;
        for conversation in convos.values_mut() {
            if conversation.claude_session_id.is_some()
                && matches!(
                    conversation.attribution_backfill_status,
                    Some(AttributionBackfillStatus::Running)
                )
            {
                conversation.attribution_backfill_status = Some(AttributionBackfillStatus::Pending);
                conversation.attribution_backfill_completed_at = None;
                conversation.updated_at = Utc::now();
                updated += 1;
            }
        }
        Ok(updated)
    }

    async fn update_attribution_backfill_state(
        &self,
        id: &ChatConversationId,
        state: ConversationAttributionBackfillState,
    ) -> AppResult<()> {
        let mut convos = self.conversations.write().await;
        if let Some(conversation) = convos.get_mut(id) {
            conversation.update_attribution_backfill_state(state);
        }
        Ok(())
    }

    async fn get_attribution_backfill_summary(
        &self,
    ) -> AppResult<ConversationAttributionBackfillSummary> {
        let convos = self.conversations.read().await;
        let mut summary = ConversationAttributionBackfillSummary::default();

        for conversation in convos.values() {
            if conversation.claude_session_id.is_none() {
                continue;
            }

            summary.eligible_conversation_count += 1;
            match conversation.attribution_backfill_status {
                None | Some(AttributionBackfillStatus::Pending) => summary.pending_count += 1,
                Some(AttributionBackfillStatus::Running) => summary.running_count += 1,
                Some(AttributionBackfillStatus::Completed) => summary.completed_count += 1,
                Some(AttributionBackfillStatus::Partial) => summary.partial_count += 1,
                Some(AttributionBackfillStatus::SessionNotFound) => {
                    summary.session_not_found_count += 1;
                }
                Some(AttributionBackfillStatus::ParseFailed) => summary.parse_failed_count += 1,
            }
        }

        Ok(summary)
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
#[path = "memory_chat_conversation_repo_tests.rs"]
mod tests;
