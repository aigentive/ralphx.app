use crate::application::chat_service::{AgentRunCompletedPayload, AgentRunStartedPayload};
use crate::domain::agents::AgentHarnessKind;

#[test]
fn agent_run_started_payload_serde_camel_case() {
    let payload = AgentRunStartedPayload {
        run_id: "run-1".to_string(),
        conversation_id: "conv-1".to_string(),
        context_type: "task_execution".to_string(),
        context_id: "task-1".to_string(),
        run_chain_id: None,
        parent_run_id: None,
        effective_model_id: Some("claude-sonnet-4-6".to_string()),
        effective_model_label: Some("Sonnet 4.6".to_string()),
        provider_harness: Some("claude".to_string()),
        provider_session_id: Some("session-123".to_string()),
    };

    let value = serde_json::to_value(&payload).expect("serialization failed");

    // Fields must be camelCase (serde rename_all = "camelCase")
    assert_eq!(value["effectiveModelId"], "claude-sonnet-4-6");
    assert_eq!(value["effectiveModelLabel"], "Sonnet 4.6");
    assert_eq!(value["providerHarness"], "claude");
    assert_eq!(value["providerSessionId"], "session-123");

    // Confirm snake_case keys are NOT present
    assert!(value.get("effective_model_id").is_none());
    assert!(value.get("effective_model_label").is_none());

    // Confirm other required fields are also camelCase
    assert_eq!(value["runId"], "run-1");
    assert_eq!(value["conversationId"], "conv-1");
    assert_eq!(value["contextType"], "task_execution");
    assert_eq!(value["contextId"], "task-1");
}

#[test]
fn agent_run_started_payload_serde_skips_none_fields() {
    let payload = AgentRunStartedPayload {
        run_id: "run-1".to_string(),
        conversation_id: "conv-1".to_string(),
        context_type: "task_execution".to_string(),
        context_id: "task-1".to_string(),
        run_chain_id: None,
        parent_run_id: None,
        effective_model_id: None,
        effective_model_label: None,
        provider_harness: None,
        provider_session_id: None,
    };

    let value = serde_json::to_value(&payload).expect("serialization failed");

    // None fields with skip_serializing_if should be absent
    assert!(value.get("effectiveModelId").is_none());
    assert!(value.get("effectiveModelLabel").is_none());
    assert!(value.get("providerHarness").is_none());
    assert!(value.get("providerSessionId").is_none());
    assert!(value.get("runChainId").is_none());
    assert!(value.get("parentRunId").is_none());
}

#[test]
fn agent_run_started_payload_helper_serializes_provider_metadata() {
    let payload = AgentRunStartedPayload::with_provider_session(
        "run-1",
        "conv-1",
        "task_execution",
        "task-1",
        None,
        None,
        Some("gpt-4.5".to_string()),
        Some("GPT-4.5".to_string()),
        Some(AgentHarnessKind::Codex),
        Some("thread-123".to_string()),
    );

    assert_eq!(payload.provider_harness, Some("codex".to_string()));
    assert_eq!(payload.provider_session_id, Some("thread-123".to_string()));
    assert_eq!(payload.effective_model_id, Some("gpt-4.5".to_string()));
    assert_eq!(payload.effective_model_label, Some("GPT-4.5".to_string()));
}

#[test]
fn agent_run_completed_payload_sets_legacy_claude_alias_only_for_claude() {
    let claude_payload = AgentRunCompletedPayload::with_provider_session(
        "conv-1",
        "ideation",
        "session-1",
        Some(AgentHarnessKind::Claude),
        Some("claude-session-123".to_string()),
        None,
    );
    let codex_payload = AgentRunCompletedPayload::with_provider_session(
        "conv-2",
        "ideation",
        "session-2",
        Some(AgentHarnessKind::Codex),
        Some("codex-thread-123".to_string()),
        None,
    );

    assert_eq!(
        claude_payload.claude_session_id,
        Some("claude-session-123".to_string())
    );
    assert_eq!(
        claude_payload.provider_harness,
        Some("claude".to_string())
    );
    assert_eq!(
        claude_payload.provider_session_id,
        Some("claude-session-123".to_string())
    );

    assert_eq!(codex_payload.claude_session_id, None);
    assert_eq!(codex_payload.provider_harness, Some("codex".to_string()));
    assert_eq!(
        codex_payload.provider_session_id,
        Some("codex-thread-123".to_string())
    );
}
