use super::*;
use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{AttributionBackfillStatus, IdeationSessionId};

#[tokio::test]
async fn test_create_and_get() {
    let repo = MemoryChatConversationRepository::new();
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id);
    let id = conv.id;

    repo.create(conv.clone()).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, id);
}

#[tokio::test]
async fn test_get_by_context() {
    let repo = MemoryChatConversationRepository::new();
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id.clone());

    repo.create(conv.clone()).await.unwrap();

    let convos = repo
        .get_by_context(ChatContextType::Ideation, session_id.as_str())
        .await
        .unwrap();
    assert_eq!(convos.len(), 1);
}

#[tokio::test]
async fn test_update_claude_session_id() {
    let repo = MemoryChatConversationRepository::new();
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id);
    let id = conv.id;

    repo.create(conv).await.unwrap();
    repo.update_claude_session_id(&id, "test-session-123")
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(
        retrieved.claude_session_id,
        Some("test-session-123".to_string())
    );
    assert_eq!(
        retrieved.provider_session_id,
        Some("test-session-123".to_string())
    );
    assert_eq!(retrieved.provider_harness, Some(AgentHarnessKind::Claude));
}

#[tokio::test]
async fn test_update_provider_session_ref_for_codex() {
    let repo = MemoryChatConversationRepository::new();
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id);
    let id = conv.id;

    repo.create(conv).await.unwrap();
    repo.update_provider_session_ref(
        &id,
        &ProviderSessionRef {
            harness: AgentHarnessKind::Codex,
            provider_session_id: "codex-session-1".to_string(),
        },
    )
    .await
    .unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.provider_harness, Some(AgentHarnessKind::Codex));
    assert_eq!(
        retrieved.provider_session_id,
        Some("codex-session-1".to_string())
    );
    assert_eq!(retrieved.claude_session_id, None);
}

#[tokio::test]
async fn test_update_provider_origin() {
    let repo = MemoryChatConversationRepository::new();
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id);
    let id = conv.id;

    repo.create(conv).await.unwrap();
    repo.update_provider_origin(&id, Some("z_ai"), Some("z_ai"))
        .await
        .unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.upstream_provider.as_deref(), Some("z_ai"));
    assert_eq!(retrieved.provider_profile.as_deref(), Some("z_ai"));
}

#[tokio::test]
async fn test_get_attribution_backfill_summary_counts_legacy_states() {
    let repo = MemoryChatConversationRepository::new();

    let session_a = IdeationSessionId::new();
    let session_b = IdeationSessionId::new();
    let session_c = IdeationSessionId::new();

    let mut pending = ChatConversation::new_ideation(session_a);
    pending.claude_session_id = Some("claude-pending".to_string());

    let mut running = ChatConversation::new_ideation(session_b);
    running.claude_session_id = Some("claude-running".to_string());
    running.attribution_backfill_status = Some(AttributionBackfillStatus::Running);

    let mut partial = ChatConversation::new_ideation(session_c);
    partial.claude_session_id = Some("claude-partial".to_string());
    partial.attribution_backfill_status = Some(AttributionBackfillStatus::Partial);

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();
    repo.create(partial).await.unwrap();
    repo.create(ChatConversation::new_project(crate::domain::entities::ProjectId::from_string("project-1".to_string())))
        .await
        .unwrap();

    let summary = repo.get_attribution_backfill_summary().await.unwrap();

    assert_eq!(summary.eligible_conversation_count, 3);
    assert_eq!(summary.pending_count, 1);
    assert_eq!(summary.running_count, 1);
    assert_eq!(summary.partial_count, 1);
    assert_eq!(summary.completed_count, 0);
    assert_eq!(summary.remaining_count(), 2);
    assert_eq!(summary.attention_count(), 1);
}
