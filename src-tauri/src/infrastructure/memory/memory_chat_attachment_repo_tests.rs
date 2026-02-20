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

    let attachments = repo
        .find_by_conversation_id(&conversation_id)
        .await
        .unwrap();
    assert_eq!(attachments.len(), 2);
}

#[tokio::test]
async fn test_update_message_id() {
    let repo = MemoryChatAttachmentRepository::new();
    let conversation_id = ChatConversationId::new();
    let attachment =
        ChatAttachment::new(conversation_id, "test.txt", "/path/to/test.txt", 1024, None);

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
    let attachment =
        ChatAttachment::new(conversation_id, "test.txt", "/path/to/test.txt", 1024, None);

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

    let attachments = repo
        .find_by_conversation_id(&conversation_id)
        .await
        .unwrap();
    assert_eq!(attachments.len(), 2);

    repo.delete_by_conversation_id(&conversation_id)
        .await
        .unwrap();

    let attachments = repo
        .find_by_conversation_id(&conversation_id)
        .await
        .unwrap();
    assert_eq!(attachments.len(), 0);
}
