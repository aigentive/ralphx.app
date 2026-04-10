use super::*;
use crate::domain::agents::{AgentHarnessKind, LogicalEffort};
use crate::domain::entities::AgentRunUsage;

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let mut run = AgentRun::new(conversation_id);
    run.harness = Some(AgentHarnessKind::Codex);
    run.provider_session_id = Some("session-123".to_string());
    run.logical_effort = Some(LogicalEffort::Medium);
    run.input_tokens = Some(123);
    run.output_tokens = Some(45);
    run.cache_creation_tokens = Some(6);
    run.cache_read_tokens = Some(78);
    run.estimated_usd = Some(0.009);
    let id = run.id;

    repo.create(run.clone()).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.harness, Some(AgentHarnessKind::Codex));
    assert_eq!(retrieved.provider_session_id, Some("session-123".to_string()));
    assert_eq!(retrieved.logical_effort, Some(LogicalEffort::Medium));
    assert_eq!(retrieved.input_tokens, Some(123));
    assert_eq!(retrieved.output_tokens, Some(45));
    assert_eq!(retrieved.cache_creation_tokens, Some(6));
    assert_eq!(retrieved.cache_read_tokens, Some(78));
    assert_eq!(retrieved.estimated_usd, Some(0.009));
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
async fn test_update_usage() {
    let repo = MemoryAgentRunRepository::new();
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);
    let id = run.id;

    repo.create(run).await.unwrap();
    repo.update_usage(
        &id,
        &AgentRunUsage {
            input_tokens: Some(50),
            output_tokens: Some(20),
            cache_creation_tokens: Some(5),
            cache_read_tokens: Some(10),
            estimated_usd: Some(0.0035),
        },
    )
    .await
    .unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.input_tokens, Some(50));
    assert_eq!(retrieved.output_tokens, Some(20));
    assert_eq!(retrieved.cache_creation_tokens, Some(5));
    assert_eq!(retrieved.cache_read_tokens, Some(10));
    assert_eq!(retrieved.estimated_usd, Some(0.0035));
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
