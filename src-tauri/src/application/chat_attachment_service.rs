// ChatAttachmentService
// Application service for managing chat attachments: upload, link, list, delete

use std::path::PathBuf;
use std::sync::Arc;

use crate::domain::entities::{
    ChatAttachment, ChatAttachmentId, ChatConversationId, ChatMessageId,
};
use crate::domain::repositories::ChatAttachmentRepository;
use crate::error::{AppError, AppResult};

/// Service for managing chat file attachments
pub struct ChatAttachmentService<R: ChatAttachmentRepository> {
    /// Repository for ChatAttachment records
    repository: Arc<R>,
    /// Base directory for attachment storage
    storage_base_path: PathBuf,
}

impl<R: ChatAttachmentRepository> ChatAttachmentService<R> {
    /// Create a new chat attachment service
    pub fn new(repository: Arc<R>, storage_base_path: impl Into<PathBuf>) -> Self {
        Self {
            repository,
            storage_base_path: storage_base_path.into(),
        }
    }

    /// Upload a file attachment and create a database record
    ///
    /// Storage path: {base}/chat_attachments/{conversation_id}/{attachment_id}/{file_name}
    ///
    /// Returns the created attachment with generated ID
    pub async fn upload(
        &self,
        conversation_id: &ChatConversationId,
        file_name: impl Into<String>,
        file_data: &[u8],
        mime_type: Option<String>,
    ) -> AppResult<ChatAttachment> {
        let file_name = file_name.into();
        let file_size = file_data.len() as i64;

        // Generate attachment ID and determine storage path
        let attachment_id = ChatAttachmentId::new();
        let file_path = self.build_file_path(conversation_id, &attachment_id, &file_name);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Infrastructure(format!("Failed to create directory: {}", e))
            })?;
        }

        // Write file to disk
        std::fs::write(&file_path, file_data)
            .map_err(|e| AppError::Infrastructure(format!("Failed to write file: {}", e)))?;

        // Create database record
        let attachment = ChatAttachment::new(
            *conversation_id,
            file_name,
            file_path.to_string_lossy().to_string(),
            file_size,
            mime_type,
        );

        self.repository.create(attachment).await
    }

    /// Link one or more attachments to a message (after message is sent)
    pub async fn link_to_message(
        &self,
        attachment_ids: &[ChatAttachmentId],
        message_id: &ChatMessageId,
    ) -> AppResult<()> {
        if attachment_ids.is_empty() {
            return Ok(());
        }

        self.repository
            .update_message_ids(attachment_ids, message_id)
            .await
    }

    /// List all attachments for a conversation
    pub async fn list_for_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<ChatAttachment>> {
        self.repository
            .find_by_conversation_id(conversation_id)
            .await
    }

    /// List all attachments for a specific message
    pub async fn list_for_message(
        &self,
        message_id: &ChatMessageId,
    ) -> AppResult<Vec<ChatAttachment>> {
        self.repository.find_by_message_id(message_id).await
    }

    /// Delete an attachment (removes file and database record)
    pub async fn delete(&self, id: &ChatAttachmentId) -> AppResult<()> {
        // Get attachment to find file path
        let attachment = self
            .repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Attachment {} not found", id)))?;

        // Delete file from disk (ignore errors if file doesn't exist)
        let file_path = PathBuf::from(&attachment.file_path);
        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| AppError::Infrastructure(format!("Failed to delete file: {}", e)))?;

            // Try to clean up empty parent directories
            if let Some(parent) = file_path.parent() {
                let _ = std::fs::remove_dir(parent); // Ignore errors - dir may not be empty
                if let Some(grandparent) = parent.parent() {
                    let _ = std::fs::remove_dir(grandparent); // Clean up conversation dir if empty
                }
            }
        }

        // Delete database record
        self.repository.delete(id).await
    }

    /// Build the file storage path for an attachment
    ///
    /// Pattern: {base}/chat_attachments/{conversation_id}/{attachment_id}/{file_name}
    fn build_file_path(
        &self,
        conversation_id: &ChatConversationId,
        attachment_id: &ChatAttachmentId,
        file_name: &str,
    ) -> PathBuf {
        self.storage_base_path
            .join("chat_attachments")
            .join(conversation_id.as_str())
            .join(attachment_id.as_str())
            .join(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::ChatAttachmentRepository;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // Mock repository for testing
    struct MockChatAttachmentRepository {
        attachments: Arc<Mutex<HashMap<String, ChatAttachment>>>,
    }

    impl MockChatAttachmentRepository {
        fn new() -> Self {
            Self {
                attachments: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl ChatAttachmentRepository for MockChatAttachmentRepository {
        async fn create(&self, attachment: ChatAttachment) -> AppResult<ChatAttachment> {
            let mut map = self.attachments.lock().await;
            map.insert(attachment.id.as_str().to_string(), attachment.clone());
            Ok(attachment)
        }

        async fn get_by_id(&self, id: &ChatAttachmentId) -> AppResult<Option<ChatAttachment>> {
            let map = self.attachments.lock().await;
            Ok(map.get(&id.as_str().to_string()).cloned())
        }

        async fn find_by_conversation_id(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<Vec<ChatAttachment>> {
            let map = self.attachments.lock().await;
            Ok(map
                .values()
                .filter(|a| a.conversation_id == *conversation_id)
                .cloned()
                .collect())
        }

        async fn find_by_message_id(
            &self,
            message_id: &ChatMessageId,
        ) -> AppResult<Vec<ChatAttachment>> {
            let map = self.attachments.lock().await;
            Ok(map
                .values()
                .filter(|a| a.message_id.as_ref() == Some(message_id))
                .cloned()
                .collect())
        }

        async fn update_message_id(
            &self,
            id: &ChatAttachmentId,
            message_id: &ChatMessageId,
        ) -> AppResult<()> {
            let mut map = self.attachments.lock().await;
            if let Some(attachment) = map.get_mut(&id.as_str().to_string()) {
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
            let mut map = self.attachments.lock().await;
            map.remove(&id.as_str().to_string());
            Ok(())
        }

        async fn delete_by_conversation_id(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<()> {
            let mut map = self.attachments.lock().await;
            map.retain(|_, a| a.conversation_id != *conversation_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_upload_creates_file_and_db_record() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Arc::new(MockChatAttachmentRepository::new());
        let service = ChatAttachmentService::new(repo.clone(), temp_dir.path());

        let conversation_id = ChatConversationId::new();
        let file_data = b"Hello, world!";

        let result = service
            .upload(
                &conversation_id,
                "test.txt",
                file_data,
                Some("text/plain".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let attachment = result.unwrap();
        assert_eq!(attachment.file_name, "test.txt");
        assert_eq!(attachment.file_size, 13);
        assert_eq!(attachment.conversation_id, conversation_id);

        // Verify file exists
        let file_path = PathBuf::from(&attachment.file_path);
        assert!(file_path.exists());
        let content = std::fs::read(&file_path).unwrap();
        assert_eq!(content, file_data);

        // Verify DB record
        let stored = repo.get_by_id(&attachment.id).await.unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_link_to_message() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Arc::new(MockChatAttachmentRepository::new());
        let service = ChatAttachmentService::new(repo.clone(), temp_dir.path());

        let conversation_id = ChatConversationId::new();
        let attachment = service
            .upload(&conversation_id, "file.txt", b"data", None)
            .await
            .unwrap();

        let message_id = ChatMessageId::new();
        let result = service.link_to_message(&[attachment.id], &message_id).await;

        assert!(result.is_ok());

        // Verify attachment is linked
        let updated = repo.get_by_id(&attachment.id).await.unwrap().unwrap();
        assert_eq!(updated.message_id, Some(message_id));
    }

    #[tokio::test]
    async fn test_list_for_conversation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Arc::new(MockChatAttachmentRepository::new());
        let service = ChatAttachmentService::new(repo.clone(), temp_dir.path());

        let conversation_id = ChatConversationId::new();
        service
            .upload(&conversation_id, "file1.txt", b"data1", None)
            .await
            .unwrap();
        service
            .upload(&conversation_id, "file2.txt", b"data2", None)
            .await
            .unwrap();

        let attachments = service
            .list_for_conversation(&conversation_id)
            .await
            .unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_removes_file_and_record() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = Arc::new(MockChatAttachmentRepository::new());
        let service = ChatAttachmentService::new(repo.clone(), temp_dir.path());

        let conversation_id = ChatConversationId::new();
        let attachment = service
            .upload(&conversation_id, "file.txt", b"data", None)
            .await
            .unwrap();

        let file_path = PathBuf::from(&attachment.file_path);
        assert!(file_path.exists());

        let result = service.delete(&attachment.id).await;
        assert!(result.is_ok());

        // Verify file is deleted
        assert!(!file_path.exists());

        // Verify DB record is deleted
        let stored = repo.get_by_id(&attachment.id).await.unwrap();
        assert!(stored.is_none());
    }
}
