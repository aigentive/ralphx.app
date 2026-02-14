// In-memory ChatAttachmentRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe in-memory storage

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{
    ChatAttachment, ChatAttachmentId, ChatConversationId, ChatMessageId,
};
use crate::domain::repositories::ChatAttachmentRepository;
use crate::error::AppResult;

/// In-memory implementation of ChatAttachmentRepository for testing
pub struct MemoryChatAttachmentRepository {
    attachments: RwLock<HashMap<String, ChatAttachment>>,
}

impl MemoryChatAttachmentRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            attachments: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryChatAttachmentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChatAttachmentRepository for MemoryChatAttachmentRepository {
    async fn create(&self, attachment: ChatAttachment) -> AppResult<ChatAttachment> {
        self.attachments
            .write()
            .unwrap()
            .insert(attachment.id.as_str(), attachment.clone());
        Ok(attachment)
    }

    async fn get_by_id(&self, id: &ChatAttachmentId) -> AppResult<Option<ChatAttachment>> {
        Ok(self
            .attachments
            .read()
            .unwrap()
            .get(&id.as_str())
            .cloned())
    }

    async fn find_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatAttachment>> {
        let mut attachments: Vec<_> = self
            .attachments
            .read()
            .unwrap()
            .values()
            .filter(|a| a.conversation_id == *conversation_id)
            .cloned()
            .collect();
        attachments.sort_by_key(|a| a.created_at);
        Ok(attachments)
    }

    async fn find_by_message_id(
        &self,
        message_id: &ChatMessageId,
    ) -> AppResult<Vec<ChatAttachment>> {
        let mut attachments: Vec<_> = self
            .attachments
            .read()
            .unwrap()
            .values()
            .filter(|a| a.message_id.as_ref() == Some(message_id))
            .cloned()
            .collect();
        attachments.sort_by_key(|a| a.created_at);
        Ok(attachments)
    }

    async fn update_message_id(
        &self,
        id: &ChatAttachmentId,
        message_id: &ChatMessageId,
    ) -> AppResult<()> {
        if let Some(attachment) = self.attachments.write().unwrap().get_mut(&id.as_str()) {
            attachment.set_message_id(message_id.clone());
        }
        Ok(())
    }

    async fn update_message_ids(
        &self,
        attachment_ids: &[ChatAttachmentId],
        message_id: &ChatMessageId,
    ) -> AppResult<()> {
        for id in attachment_ids {
            self.update_message_id(id, message_id).await?;
        }
        Ok(())
    }

    async fn delete(&self, id: &ChatAttachmentId) -> AppResult<()> {
        self.attachments.write().unwrap().remove(&id.as_str());
        Ok(())
    }

    async fn delete_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<()> {
        self.attachments
            .write()
            .unwrap()
            .retain(|_, a| a.conversation_id != *conversation_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get() {
        let repo = MemoryChatAttachmentRepository::new();
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "test.txt",
            "/path/to/test.txt",
            1024,
            Some("text/plain".to_string()),
        );

        let created = repo.create(attachment.clone()).await.unwrap();
        assert_eq!(created.id, attachment.id);

        let found = repo.get_by_id(&attachment.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().file_name, "test.txt");
    }

    #[tokio::test]
    async fn test_find_by_conversation_id() {
        let repo = MemoryChatAttachmentRepository::new();
        let conversation_id = ChatConversationId::new();

        let attachment1 = ChatAttachment::new(
            conversation_id,
            "file1.txt",
            "/path/to/file1.txt",
            512,
            None,
        );
        let attachment2 = ChatAttachment::new(
            conversation_id,
            "file2.txt",
            "/path/to/file2.txt",
            1024,
            None,
        );

        repo.create(attachment1).await.unwrap();
        repo.create(attachment2).await.unwrap();

        let attachments = repo.find_by_conversation_id(&conversation_id).await.unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[tokio::test]
    async fn test_update_message_id() {
        let repo = MemoryChatAttachmentRepository::new();
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "test.txt",
            "/path/to/test.txt",
            1024,
            None,
        );

        let created = repo.create(attachment.clone()).await.unwrap();
        assert_eq!(created.message_id, None);

        let message_id = ChatMessageId::new();
        repo.update_message_id(&attachment.id, &message_id)
            .await
            .unwrap();

        let updated = repo.get_by_id(&attachment.id).await.unwrap().unwrap();
        assert_eq!(updated.message_id, Some(message_id));
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MemoryChatAttachmentRepository::new();
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "test.txt",
            "/path/to/test.txt",
            1024,
            None,
        );

        repo.create(attachment.clone()).await.unwrap();
        assert!(repo.get_by_id(&attachment.id).await.unwrap().is_some());

        repo.delete(&attachment.id).await.unwrap();
        assert!(repo.get_by_id(&attachment.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_by_conversation_id() {
        let repo = MemoryChatAttachmentRepository::new();
        let conversation_id = ChatConversationId::new();

        let attachment1 = ChatAttachment::new(
            conversation_id,
            "file1.txt",
            "/path/to/file1.txt",
            512,
            None,
        );
        let attachment2 = ChatAttachment::new(
            conversation_id,
            "file2.txt",
            "/path/to/file2.txt",
            1024,
            None,
        );

        repo.create(attachment1).await.unwrap();
        repo.create(attachment2).await.unwrap();

        let attachments = repo.find_by_conversation_id(&conversation_id).await.unwrap();
        assert_eq!(attachments.len(), 2);

        repo.delete_by_conversation_id(&conversation_id)
            .await
            .unwrap();

        let attachments = repo.find_by_conversation_id(&conversation_id).await.unwrap();
        assert_eq!(attachments.len(), 0);
    }
}
