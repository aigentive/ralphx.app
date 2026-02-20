use super::*;

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
