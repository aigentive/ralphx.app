use super::*;

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::user_in_session(session_id.clone(), "Hello");

    repo.create(message.clone()).await.unwrap();

    let retrieved = repo.get_by_id(&message.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, message.id);
}

#[tokio::test]
async fn test_get_by_session() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::user_in_session(session_id.clone(), "Hello");

    repo.create(message).await.unwrap();

    let messages = repo.get_by_session(&session_id).await.unwrap();
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_delete() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::user_in_session(session_id.clone(), "Hello");
    let message_id = message.id.clone();

    repo.create(message).await.unwrap();
    repo.delete(&message_id).await.unwrap();

    let result = repo.get_by_id(&message_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_by_session() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();

    repo.create(ChatMessage::user_in_session(session_id.clone(), "Hello 1"))
        .await
        .unwrap();
    repo.create(ChatMessage::user_in_session(session_id.clone(), "Hello 2"))
        .await
        .unwrap();

    repo.delete_by_session(&session_id).await.unwrap();

    let messages = repo.get_by_session(&session_id).await.unwrap();
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_get_recent_by_session() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();

    for i in 1..=5 {
        repo.create(ChatMessage::user_in_session(
            session_id.clone(),
            format!("Message {}", i),
        ))
        .await
        .unwrap();
    }

    let recent = repo.get_recent_by_session(&session_id, 3).await.unwrap();
    assert_eq!(recent.len(), 3);
}
