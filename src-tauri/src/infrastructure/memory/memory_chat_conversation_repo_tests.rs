use super::*;
use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::IdeationSessionId;

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
