// Chat conversation repository trait - domain layer abstraction
//
// This trait defines the contract for chat conversation persistence.
// Conversations track provider sessions linked to contexts (ideation, task, project).

use async_trait::async_trait;

use crate::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{
    ChatContextType, ChatConversation, ChatConversationId, ConversationAttributionBackfillState,
};
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

    /// Update the provider session reference for a conversation.
    async fn update_provider_session_ref(
        &self,
        id: &ChatConversationId,
        session_ref: &ProviderSessionRef,
    ) -> AppResult<()>;

    /// Clear the provider session reference for a conversation.
    async fn clear_provider_session_ref(&self, id: &ChatConversationId) -> AppResult<()>;

    /// Update provider-origin metadata for a conversation.
    async fn update_provider_origin(
        &self,
        id: &ChatConversationId,
        upstream_provider: Option<&str>,
        provider_profile: Option<&str>,
    ) -> AppResult<()>;

    /// Compatibility helper for legacy Claude-specific callers.
    async fn update_claude_session_id(
        &self,
        id: &ChatConversationId,
        claude_session_id: &str,
    ) -> AppResult<()> {
        self.update_provider_session_ref(
            id,
            &ProviderSessionRef {
                harness: AgentHarnessKind::Claude,
                provider_session_id: claude_session_id.to_string(),
            },
        )
        .await
    }

    /// Compatibility helper for legacy Claude-specific callers.
    async fn clear_claude_session_id(&self, id: &ChatConversationId) -> AppResult<()> {
        self.clear_provider_session_ref(id).await
    }

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

    /// List conversations backed by historical Claude sessions that still need
    /// attribution backfill work.
    async fn list_needing_attribution_backfill(
        &self,
        limit: u32,
    ) -> AppResult<Vec<ChatConversation>>;

    /// Update attribution backfill workflow state for a conversation.
    async fn update_attribution_backfill_state(
        &self,
        id: &ChatConversationId,
        state: ConversationAttributionBackfillState,
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
#[path = "chat_conversation_repository_tests.rs"]
mod tests;
