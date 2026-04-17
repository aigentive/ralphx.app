use super::*;
use crate::domain::agents::{AgentHarnessKind, LogicalEffort};
use crate::domain::entities::{AgentRunUsage, ChatMessageAttribution};

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

#[tokio::test]
async fn test_update_usage_updates_message_usage_fields() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::orchestrator_in_session(session_id, "Usage message");
    let message_id = message.id.clone();

    repo.create(message).await.unwrap();
    repo.update_usage(
        &message_id,
        &AgentRunUsage {
            input_tokens: Some(90),
            output_tokens: Some(24),
            cache_creation_tokens: Some(8),
            cache_read_tokens: Some(33),
            estimated_usd: Some(0.015),
        },
    )
    .await
    .unwrap();

    let updated = repo.get_by_id(&message_id).await.unwrap().unwrap();
    assert_eq!(updated.input_tokens, Some(90));
    assert_eq!(updated.output_tokens, Some(24));
    assert_eq!(updated.cache_creation_tokens, Some(8));
    assert_eq!(updated.cache_read_tokens, Some(33));
    assert_eq!(updated.estimated_usd, Some(0.015));
}

#[tokio::test]
async fn test_update_attribution_updates_message_attribution_fields() {
    let repo = MemoryChatMessageRepository::new();
    let session_id = IdeationSessionId::new();
    let message = ChatMessage::orchestrator_in_session(session_id, "Attributed message");
    let message_id = message.id.clone();

    repo.create(message).await.unwrap();
    repo.update_attribution(
        &message_id,
        &ChatMessageAttribution {
            attribution_source: Some("historical_backfill_claude_project_jsonl_z_ai".to_string()),
            provider_harness: Some(AgentHarnessKind::Claude),
            provider_session_id: Some("claude-session-123".to_string()),
            upstream_provider: Some("z_ai".to_string()),
            provider_profile: Some("z_ai".to_string()),
            logical_model: Some("glm-4.7".to_string()),
            effective_model_id: Some("glm-4.7".to_string()),
            logical_effort: Some(LogicalEffort::High),
            effective_effort: Some("high".to_string()),
        },
    )
    .await
    .unwrap();

    let updated = repo.get_by_id(&message_id).await.unwrap().unwrap();
    assert_eq!(
        updated.attribution_source.as_deref(),
        Some("historical_backfill_claude_project_jsonl_z_ai")
    );
    assert_eq!(updated.provider_harness, Some(AgentHarnessKind::Claude));
    assert_eq!(
        updated.provider_session_id.as_deref(),
        Some("claude-session-123")
    );
    assert_eq!(updated.upstream_provider.as_deref(), Some("z_ai"));
    assert_eq!(updated.provider_profile.as_deref(), Some("z_ai"));
    assert_eq!(updated.logical_model.as_deref(), Some("glm-4.7"));
    assert_eq!(updated.effective_model_id.as_deref(), Some("glm-4.7"));
    assert_eq!(updated.logical_effort, Some(LogicalEffort::High));
    assert_eq!(updated.effective_effort.as_deref(), Some("high"));
}
