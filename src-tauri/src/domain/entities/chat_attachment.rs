use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use super::{ChatConversationId, ChatMessageId};

/// Unique identifier for a chat attachment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatAttachmentId(Uuid);

impl ChatAttachmentId {
    /// Create a new random attachment ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Get as string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Create from string (for database deserialization)
    pub fn from_string(s: impl Into<String>) -> Self {
        let s = s.into();
        Self(Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::nil()))
    }
}

impl Default for ChatAttachmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ChatAttachmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ChatAttachmentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ChatAttachmentId> for String {
    fn from(id: ChatAttachmentId) -> Self {
        id.0.to_string()
    }
}

impl std::str::FromStr for ChatAttachmentId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// A file attachment associated with a chat conversation and optionally a specific message
///
/// Attachments are initially created when files are uploaded and linked to a conversation.
/// After the message is sent, the message_id is set to link the attachment to a specific message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAttachment {
    /// Unique identifier for this attachment
    pub id: ChatAttachmentId,
    /// ID of the conversation this attachment belongs to
    pub conversation_id: ChatConversationId,
    /// ID of the message this attachment is linked to (null until message is sent)
    pub message_id: Option<ChatMessageId>,
    /// Original filename
    pub file_name: String,
    /// Absolute path to the file in app data directory
    pub file_path: String,
    /// MIME type of the file (optional)
    pub mime_type: Option<String>,
    /// Size of the file in bytes
    pub file_size: i64,
    /// When this attachment was created (uploaded)
    pub created_at: DateTime<Utc>,
}

impl ChatAttachment {
    /// Create a new attachment for a conversation
    pub fn new(
        conversation_id: ChatConversationId,
        file_name: impl Into<String>,
        file_path: impl Into<String>,
        file_size: i64,
        mime_type: Option<String>,
    ) -> Self {
        Self {
            id: ChatAttachmentId::new(),
            conversation_id,
            message_id: None,
            file_name: file_name.into(),
            file_path: file_path.into(),
            mime_type,
            file_size,
            created_at: Utc::now(),
        }
    }

    /// Link this attachment to a message
    pub fn set_message_id(&mut self, message_id: ChatMessageId) {
        self.message_id = Some(message_id);
    }

    /// Check if this attachment is linked to a message
    pub fn is_linked_to_message(&self) -> bool {
        self.message_id.is_some()
    }

    /// Get a display name for this attachment (filename only, without path)
    pub fn display_name(&self) -> &str {
        &self.file_name
    }

    /// Get the file extension (if any)
    pub fn extension(&self) -> Option<&str> {
        std::path::Path::new(&self.file_name)
            .extension()
            .and_then(|ext| ext.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_id_creation() {
        let id1 = ChatAttachmentId::new();
        let id2 = ChatAttachmentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_attachment_id_from_string() {
        let id = ChatAttachmentId::new();
        let str_id = id.to_string();
        let parsed_id: ChatAttachmentId = str_id.parse().unwrap();
        assert_eq!(id, parsed_id);
    }

    #[test]
    fn test_new_attachment() {
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "test.txt",
            "/path/to/test.txt",
            1024,
            Some("text/plain".to_string()),
        );

        assert_eq!(attachment.conversation_id, conversation_id);
        assert_eq!(attachment.message_id, None);
        assert_eq!(attachment.file_name, "test.txt");
        assert_eq!(attachment.file_path, "/path/to/test.txt");
        assert_eq!(attachment.mime_type, Some("text/plain".to_string()));
        assert_eq!(attachment.file_size, 1024);
        assert!(!attachment.is_linked_to_message());
    }

    #[test]
    fn test_set_message_id() {
        let conversation_id = ChatConversationId::new();
        let mut attachment =
            ChatAttachment::new(conversation_id, "test.txt", "/path/to/test.txt", 1024, None);

        assert!(!attachment.is_linked_to_message());

        let message_id = ChatMessageId::new();
        attachment.set_message_id(message_id.clone());

        assert!(attachment.is_linked_to_message());
        assert_eq!(attachment.message_id, Some(message_id));
    }

    #[test]
    fn test_display_name() {
        let conversation_id = ChatConversationId::new();
        let attachment = ChatAttachment::new(
            conversation_id,
            "my-document.pdf",
            "/path/to/my-document.pdf",
            2048,
            Some("application/pdf".to_string()),
        );

        assert_eq!(attachment.display_name(), "my-document.pdf");
    }

    #[test]
    fn test_extension() {
        let conversation_id = ChatConversationId::new();

        let txt_attachment =
            ChatAttachment::new(conversation_id, "test.txt", "/path/to/test.txt", 1024, None);
        assert_eq!(txt_attachment.extension(), Some("txt"));

        let pdf_attachment = ChatAttachment::new(
            conversation_id,
            "document.pdf",
            "/path/to/document.pdf",
            2048,
            None,
        );
        assert_eq!(pdf_attachment.extension(), Some("pdf"));

        let no_ext_attachment =
            ChatAttachment::new(conversation_id, "README", "/path/to/README", 512, None);
        assert_eq!(no_ext_attachment.extension(), None);
    }
}
