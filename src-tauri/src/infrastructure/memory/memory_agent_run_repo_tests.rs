use super::*;

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);
    let id = run.id;

    repo.create(run.clone()).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, id);
}

#[tokio::test]
async fn test_get_active_for_conversation() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);

    repo.create(run.clone()).await.unwrap();

    let active = repo
        .get_active_for_conversation(&conversation_id)
        .await
        .unwrap();
    assert!(active.is_some());
    assert!(active.unwrap().is_active());
}

#[tokio::test]
async fn test_complete() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);
    let id = run.id;

    repo.create(run).await.unwrap();
    repo.complete(&id).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, AgentRunStatus::Completed);
    assert!(retrieved.completed_at.is_some());
}

#[tokio::test]
async fn test_fail() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);
    let id = run.id;

    repo.create(run).await.unwrap();
    repo.fail(&id, "Test error").await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, AgentRunStatus::Failed);
    assert_eq!(retrieved.error_message, Some("Test error".to_string()));
}
