use crate::application::chat_service::AgentRunStartedPayload;

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
    };

    let value = serde_json::to_value(&payload).expect("serialization failed");

    // Fields must be camelCase (serde rename_all = "camelCase")
    assert_eq!(value["effectiveModelId"], "claude-sonnet-4-6");
    assert_eq!(value["effectiveModelLabel"], "Sonnet 4.6");

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
    };

    let value = serde_json::to_value(&payload).expect("serialization failed");

    // None fields with skip_serializing_if should be absent
    assert!(value.get("effectiveModelId").is_none());
    assert!(value.get("effectiveModelLabel").is_none());
    assert!(value.get("runChainId").is_none());
    assert!(value.get("parentRunId").is_none());
}
