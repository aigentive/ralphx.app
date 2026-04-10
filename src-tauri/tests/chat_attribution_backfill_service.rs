use std::sync::Arc;

use ralphx_lib::application::chat_attribution_backfill_service::ChatAttributionBackfillService;
use ralphx_lib::domain::agents::AgentHarnessKind;
use ralphx_lib::domain::entities::{
    AgentRun, AttributionBackfillStatus, ChatConversation, ChatMessage, IdeationSessionId,
};
use ralphx_lib::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
};
use ralphx_lib::infrastructure::memory::{
    MemoryAgentRunRepository, MemoryChatConversationRepository, MemoryChatMessageRepository,
};

fn write_transcript(path: &std::path::Path, lines: &[&str]) {
    std::fs::write(path, lines.join("\n")).unwrap();
}

fn make_orchestrator_message(
    conversation_id: ralphx_lib::domain::entities::ChatConversationId,
    content: &str,
) -> ChatMessage {
    let mut message = ChatMessage::orchestrator_in_session(IdeationSessionId::new(), content);
    message.conversation_id = Some(conversation_id);
    message
}

#[tokio::test]
async fn test_backfill_imports_single_run_single_message_conversation() {
    let conversation_repo = Arc::new(MemoryChatConversationRepository::new());
    let chat_message_repo = Arc::new(MemoryChatMessageRepository::new());
    let agent_run_repo = Arc::new(MemoryAgentRunRepository::new());
    let temp = tempfile::tempdir().unwrap();

    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("session-1".to_string());
    let conversation = conversation_repo.create(conversation).await.unwrap();

    let message = make_orchestrator_message(conversation.id, "Final summary");
    let message_id = message.id.clone();
    chat_message_repo.create(message).await.unwrap();

    let run = AgentRun::new(conversation.id);
    let run_id = run.id;
    agent_run_repo.create(run).await.unwrap();

    write_transcript(
        &temp.path().join("session-1.jsonl"),
        &[
            r#"{"type":"assistant","message":{"id":"msg-1","model":"glm-4.7","usage":{"input_tokens":30,"output_tokens":7,"cache_read_input_tokens":11},"content":[{"type":"text","text":"done"}]}}"#,
        ],
    );

    let service = ChatAttributionBackfillService::new(
        conversation_repo.clone(),
        chat_message_repo.clone(),
        agent_run_repo.clone(),
        temp.path().to_path_buf(),
    );

    assert_eq!(service.run_pending_batch(10).await.unwrap(), 1);

    let updated_conversation = conversation_repo
        .get_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_conversation.attribution_backfill_status,
        Some(AttributionBackfillStatus::Completed)
    );
    assert_eq!(
        updated_conversation.provider_harness,
        Some(AgentHarnessKind::Claude)
    );
    assert_eq!(
        updated_conversation.provider_session_id.as_deref(),
        Some("session-1")
    );

    let updated_message = chat_message_repo
        .get_by_id(&message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_message.attribution_source.as_deref(),
        Some("historical_backfill_claude_project_jsonl_z_ai")
    );
    assert_eq!(
        updated_message.provider_harness,
        Some(AgentHarnessKind::Claude)
    );
    assert_eq!(updated_message.logical_model.as_deref(), Some("glm-4.7"));
    assert_eq!(updated_message.input_tokens, Some(30));
    assert_eq!(updated_message.output_tokens, Some(7));
    assert_eq!(updated_message.cache_read_tokens, Some(11));

    let updated_run = agent_run_repo.get_by_id(&run_id).await.unwrap().unwrap();
    assert_eq!(updated_run.harness, Some(AgentHarnessKind::Claude));
    assert_eq!(updated_run.logical_model.as_deref(), Some("glm-4.7"));
    assert_eq!(updated_run.input_tokens, Some(30));
    assert_eq!(updated_run.output_tokens, Some(7));
    assert_eq!(updated_run.cache_read_tokens, Some(11));
}

#[tokio::test]
async fn test_backfill_marks_multi_message_conversation_partial() {
    let conversation_repo = Arc::new(MemoryChatConversationRepository::new());
    let chat_message_repo = Arc::new(MemoryChatMessageRepository::new());
    let agent_run_repo = Arc::new(MemoryAgentRunRepository::new());
    let temp = tempfile::tempdir().unwrap();

    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("session-2".to_string());
    let conversation = conversation_repo.create(conversation).await.unwrap();

    let first = make_orchestrator_message(conversation.id, "First");
    let second = make_orchestrator_message(conversation.id, "Second");
    let first_id = first.id.clone();
    let second_id = second.id.clone();
    chat_message_repo.create(first).await.unwrap();
    chat_message_repo.create(second).await.unwrap();

    let run = AgentRun::new(conversation.id);
    let run_id = run.id;
    agent_run_repo.create(run).await.unwrap();

    write_transcript(
        &temp.path().join("session-2.jsonl"),
        &[
            r#"{"type":"assistant","message":{"id":"msg-a","model":"claude-sonnet-4-6","usage":{"input_tokens":5,"output_tokens":2},"content":[{"type":"text","text":"a"}]}}"#,
            r#"{"type":"assistant","message":{"id":"msg-b","model":"claude-sonnet-4-6","usage":{"input_tokens":7,"output_tokens":3},"content":[{"type":"text","text":"b"}]}}"#,
        ],
    );

    let service = ChatAttributionBackfillService::new(
        conversation_repo.clone(),
        chat_message_repo.clone(),
        agent_run_repo.clone(),
        temp.path().to_path_buf(),
    );

    assert_eq!(service.run_pending_batch(10).await.unwrap(), 1);

    let updated_conversation = conversation_repo
        .get_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_conversation.attribution_backfill_status,
        Some(AttributionBackfillStatus::Partial)
    );
    assert!(updated_conversation
        .attribution_backfill_error_summary
        .as_deref()
        .unwrap_or("")
        .contains("provider messages"));

    let first_message = chat_message_repo
        .get_by_id(&first_id)
        .await
        .unwrap()
        .unwrap();
    let second_message = chat_message_repo
        .get_by_id(&second_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        first_message.attribution_source.as_deref(),
        Some("historical_backfill_claude_project_jsonl_anthropic")
    );
    assert_eq!(
        second_message.attribution_source.as_deref(),
        Some("historical_backfill_claude_project_jsonl_anthropic")
    );
    assert_eq!(first_message.input_tokens, None);
    assert_eq!(second_message.input_tokens, None);

    let updated_run = agent_run_repo.get_by_id(&run_id).await.unwrap().unwrap();
    assert_eq!(updated_run.input_tokens, Some(12));
    assert_eq!(updated_run.output_tokens, Some(5));
}

#[tokio::test]
async fn test_backfill_marks_missing_transcript_not_found() {
    let conversation_repo = Arc::new(MemoryChatConversationRepository::new());
    let chat_message_repo = Arc::new(MemoryChatMessageRepository::new());
    let agent_run_repo = Arc::new(MemoryAgentRunRepository::new());
    let temp = tempfile::tempdir().unwrap();

    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("missing-session".to_string());
    let conversation = conversation_repo.create(conversation).await.unwrap();

    let service = ChatAttributionBackfillService::new(
        conversation_repo.clone(),
        chat_message_repo,
        agent_run_repo,
        temp.path().to_path_buf(),
    );

    assert_eq!(service.run_pending_batch(10).await.unwrap(), 1);

    let updated_conversation = conversation_repo
        .get_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_conversation.attribution_backfill_status,
        Some(AttributionBackfillStatus::SessionNotFound)
    );
}

#[tokio::test]
async fn test_backfill_indexes_nested_transcript_paths_once() {
    let conversation_repo = Arc::new(MemoryChatConversationRepository::new());
    let chat_message_repo = Arc::new(MemoryChatMessageRepository::new());
    let agent_run_repo = Arc::new(MemoryAgentRunRepository::new());
    let temp = tempfile::tempdir().unwrap();

    let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
    conversation.claude_session_id = Some("nested-session".to_string());
    let conversation = conversation_repo.create(conversation).await.unwrap();

    let message = make_orchestrator_message(conversation.id, "Nested summary");
    let message_id = message.id.clone();
    chat_message_repo.create(message).await.unwrap();

    let run = AgentRun::new(conversation.id);
    let run_id = run.id;
    agent_run_repo.create(run).await.unwrap();

    let nested_dir = temp.path().join("project-a").join("subagent");
    std::fs::create_dir_all(&nested_dir).unwrap();
    write_transcript(
        &nested_dir.join("nested-session.jsonl"),
        &[
            r#"{"type":"assistant","message":{"id":"msg-nested","model":"claude-sonnet-4-6","usage":{"input_tokens":9,"output_tokens":4},"content":[{"type":"text","text":"nested"}]}}"#,
        ],
    );

    let service = ChatAttributionBackfillService::new(
        conversation_repo.clone(),
        chat_message_repo.clone(),
        agent_run_repo.clone(),
        temp.path().to_path_buf(),
    );

    assert_eq!(service.run_pending_batch(10).await.unwrap(), 1);

    let updated_conversation = conversation_repo
        .get_by_id(&conversation.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated_conversation.attribution_backfill_status,
        Some(AttributionBackfillStatus::Completed)
    );
    assert!(updated_conversation
        .attribution_backfill_source_path
        .as_deref()
        .unwrap_or("")
        .contains("project-a/subagent/nested-session.jsonl"));

    let updated_message = chat_message_repo
        .get_by_id(&message_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_message.input_tokens, Some(9));
    assert_eq!(updated_message.output_tokens, Some(4));

    let updated_run = agent_run_repo.get_by_id(&run_id).await.unwrap().unwrap();
    assert_eq!(updated_run.input_tokens, Some(9));
    assert_eq!(updated_run.output_tokens, Some(4));
}
