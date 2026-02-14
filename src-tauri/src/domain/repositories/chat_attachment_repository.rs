// Chat attachment repository trait - domain layer abstraction
//
// This trait defines the contract for chat attachment persistence.
// Attachments are files uploaded to conversations and optionally linked to messages.

use async_trait::async_trait;

use crate::domain::entities::{
    ChatAttachment, ChatAttachmentId, ChatConversationId, ChatMessageId,
};
use crate::error::AppResult;

/// Repository trait for ChatAttachment persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait ChatAttachmentRepository: Send + Sync {
    /// Create a new attachment
    async fn create(&self, attachment: ChatAttachment) -> AppResult<ChatAttachment>;

    /// Get attachment by ID
    async fn get_by_id(&self, id: &ChatAttachmentId) -> AppResult<Option<ChatAttachment>>;

    /// Get all attachments for a conversation
    async fn find_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatAttachment>>;

    /// Get all attachments for a specific message
    async fn find_by_message_id(
        &self,
        message_id: &ChatMessageId,
    ) -> AppResult<Vec<ChatAttachment>>;

    /// Update the message_id for an attachment (link it to a message after send)
    async fn update_message_id(
        &self,
        id: &ChatAttachmentId,
        message_id: &ChatMessageId,
    ) -> AppResult<()>;

    /// Update the message_id for multiple attachments (batch link after send)
    async fn update_message_ids(
        &self,
        attachment_ids: &[ChatAttachmentId],
        message_id: &ChatMessageId,
    ) -> AppResult<()>;

    /// Delete an attachment
    async fn delete(&self, id: &ChatAttachmentId) -> AppResult<()>;

    /// Delete all attachments for a conversation (called when conversation is deleted)
    async fn delete_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ChatConversationId;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockChatAttachmentRepository {
        attachments: Vec<ChatAttachment>,
    }

    impl MockChatAttachmentRepository {
        fn new() -> Self {
            Self {
                attachments: vec![],
            }
        }

        fn with_attachments(attachments: Vec<ChatAttachment>) -> Self {
            Self { attachments }
        }
    }

    #[async_trait]
    impl ChatAttachmentRepository for MockChatAttachmentRepository {
        async fn create(&self, attachment: ChatAttachment) -> AppResult<ChatAttachment> {
            Ok(attachment)
        }

        async fn get_by_id(&self, id: &ChatAttachmentId) -> AppResult<Option<ChatAttachment>> {
            Ok(self.attachments.iter().find(|a| a.id == *id).cloned())
        }

        async fn find_by_conversation_id(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<Vec<ChatAttachment>> {
            Ok(self
                .attachments
                .iter()
                .filter(|a| a.conversation_id == *conversation_id)
                .cloned()
                .collect())
        }

        async fn find_by_message_id(
            &self,
            message_id: &ChatMessageId,
        ) -> AppResult<Vec<ChatAttachment>> {
            Ok(self
                .attachments
                .iter()
                .filter(|a| a.message_id.as_ref() == Some(message_id))
                .cloned()
                .collect())
        }

        async fn update_message_id(
            &self,
            _id: &ChatAttachmentId,
            _message_id: &ChatMessageId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn update_message_ids(
            &self,
            _attachment_ids: &[ChatAttachmentId],
            _message_id: &ChatMessageId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ChatAttachmentId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_conversation_id(
            &self,
            _conversation_id: &ChatConversationId,
        ) -> AppResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_trait_object_safety() {
        let repo = MockChatAttachmentRepository::new();
        let _: Arc<dyn ChatAttachmentRepository> = Arc::new(repo);
    }

    #[test]
    fn test_mock_with_attachments() {
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "test.txt",
            "/path/to/test.txt",
            1024,
            Some("text/plain".to_string()),
        );
        let repo = MockChatAttachmentRepository::with_attachments(vec![attachment.clone()]);

        assert_eq!(repo.attachments.len(), 1);
        assert_eq!(repo.attachments[0].id, attachment.id);
    }
}
