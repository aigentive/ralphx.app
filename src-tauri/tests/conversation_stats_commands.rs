use ralphx_lib::commands::conversation_stats_commands::build_conversation_stats_response;
use ralphx_lib::domain::agents::{AgentHarnessKind, LogicalEffort, ProviderSessionRef};
use ralphx_lib::domain::entities::{
    AgentRun, AttributionBackfillStatus, ChatConversation, ChatMessage, IdeationSessionId,
};

#[test]
fn test_conversation_stats_prefers_message_usage_when_available() {
    let session_id = IdeationSessionId::new();
    let mut conversation = ChatConversation::new_ideation(session_id.clone());
    conversation.set_provider_session_ref(ProviderSessionRef {
        harness: AgentHarnessKind::Codex,
        provider_session_id: "thread-1".to_string(),
    });
    conversation.set_provider_origin(Some("openai".to_string()), None);
    conversation.attribution_backfill_status = Some(AttributionBackfillStatus::Completed);
    conversation.attribution_backfill_source = Some("native_runtime".to_string());

    let mut message = ChatMessage::orchestrator_in_session(session_id.clone(), "done");
    message.conversation_id = Some(conversation.id);
    message.provider_harness = Some(AgentHarnessKind::Codex);
    message.provider_session_id = Some("thread-1".to_string());
    message.upstream_provider = Some("openai".to_string());
    message.effective_model_id = Some("gpt-5.4".to_string());
    message.effective_effort = Some("high".to_string());
    message.input_tokens = Some(120);
    message.output_tokens = Some(40);
    message.cache_creation_tokens = Some(5);
    message.cache_read_tokens = Some(8);
    message.estimated_usd = Some(0.42);

    let mut run = AgentRun::new(conversation.id);
    run.harness = Some(AgentHarnessKind::Codex);
    run.upstream_provider = Some("openai".to_string());
    run.effective_model_id = Some("gpt-5.4".to_string());
    run.logical_effort = Some(LogicalEffort::High);
    run.effective_effort = Some("high".to_string());
    run.input_tokens = Some(999);
    run.output_tokens = Some(111);
    run.estimated_usd = Some(1.25);

    let response = build_conversation_stats_response(&conversation, &[message], &[run]);

    assert_eq!(response.usage_coverage.effective_totals_source, "messages");
    assert_eq!(response.message_usage_totals.input_tokens, 120);
    assert_eq!(response.message_usage_totals.output_tokens, 40);
    assert_eq!(response.effective_usage_totals.input_tokens, 120);
    assert_eq!(response.effective_usage_totals.output_tokens, 40);
    assert_eq!(response.effective_usage_totals.estimated_usd, Some(0.42));
    assert_eq!(response.by_harness[0].key, "codex");
    assert_eq!(response.by_harness[0].usage.input_tokens, 120);
    assert_eq!(response.by_model[0].key, "gpt-5.4");
    assert_eq!(response.by_effort[0].key, "high");
}

#[test]
fn test_conversation_stats_falls_back_to_run_usage_when_messages_lack_usage() {
    let session_id = IdeationSessionId::new();
    let mut conversation = ChatConversation::new_ideation(session_id.clone());
    conversation.set_provider_session_ref(ProviderSessionRef {
        harness: AgentHarnessKind::Claude,
        provider_session_id: "session-1".to_string(),
    });
    conversation.set_provider_origin(Some("z_ai".to_string()), Some("z_ai".to_string()));

    let mut message = ChatMessage::orchestrator_in_session(session_id, "hello");
    message.conversation_id = Some(conversation.id);
    message.provider_harness = Some(AgentHarnessKind::Claude);

    let mut run = AgentRun::new(conversation.id);
    run.harness = Some(AgentHarnessKind::Claude);
    run.upstream_provider = Some("z_ai".to_string());
    run.provider_profile = Some("z_ai".to_string());
    run.effective_model_id = Some("glm-4.7".to_string());
    run.effective_effort = Some("medium".to_string());
    run.input_tokens = Some(300);
    run.output_tokens = Some(120);
    run.cache_creation_tokens = Some(30);
    run.cache_read_tokens = Some(12);

    let response = build_conversation_stats_response(&conversation, &[message], &[run]);

    assert_eq!(response.usage_coverage.effective_totals_source, "runs");
    assert_eq!(response.message_usage_totals.input_tokens, 0);
    assert_eq!(response.run_usage_totals.input_tokens, 300);
    assert_eq!(response.effective_usage_totals.input_tokens, 300);
    assert_eq!(response.by_upstream_provider[0].key, "z_ai");
    assert_eq!(response.by_model[0].key, "glm-4.7");
    assert_eq!(response.attribution_coverage.provider_messages_with_attribution, 1);
}
