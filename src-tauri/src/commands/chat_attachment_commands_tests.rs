use super::*;

#[test]
fn test_chat_attachment_response_serialization() {
    use crate::domain::entities::{ChatAttachment, ChatConversationId};

    let conversation_id = ChatConversationId::new();
    let attachment = ChatAttachment::new(
        conversation_id,
        "test.txt",
        "/path/to/test.txt",
        1024,
        Some("text/plain".to_string()),
    );

    let response = ChatAttachmentResponse::from(attachment);

    // Verify serialization includes camelCase fields
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("conversationId"));
    assert!(json.contains("fileName"));
    assert!(json.contains("filePath"));
    assert!(json.contains("mimeType"));
    assert!(json.contains("fileSize"));
    assert!(json.contains("createdAt"));
}

#[test]
fn test_upload_input_deserialization() {
    let json = r#"{
        "conversationId": "conv-123",
        "fileName": "test.txt",
        "fileData": [72, 101, 108, 108, 111],
        "mimeType": "text/plain"
    }"#;

    let input: UploadChatAttachmentInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.conversation_id, "conv-123");
    assert_eq!(input.file_name, "test.txt");
    assert_eq!(input.file_data, vec![72, 101, 108, 108, 111]); // "Hello"
    assert_eq!(input.mime_type, Some("text/plain".to_string()));
}

#[test]
fn test_link_input_deserialization() {
    let json = r#"{
        "attachmentIds": ["att-1", "att-2"],
        "messageId": "msg-123"
    }"#;

    let input: LinkAttachmentsInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.attachment_ids.len(), 2);
    assert_eq!(input.message_id, "msg-123");
}
