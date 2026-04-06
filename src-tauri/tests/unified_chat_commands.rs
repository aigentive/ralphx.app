use ralphx_lib::commands::unified_chat_commands::{
    parse_context_type, AgentRunStatusResponse, QueuedMessageResponse, SendAgentMessageResponse,
};
use ralphx_lib::application::SendResult;
use ralphx_lib::domain::entities::ChatContextType;
use ralphx_lib::domain::services::QueuedMessage;

#[test]
fn test_parse_context_type() {
    assert!(matches!(
        parse_context_type("ideation"),
        Ok(ChatContextType::Ideation)
    ));
    assert!(matches!(
        parse_context_type("task"),
        Ok(ChatContextType::Task)
    ));
    assert!(matches!(
        parse_context_type("project"),
        Ok(ChatContextType::Project)
    ));
    assert!(matches!(
        parse_context_type("task_execution"),
        Ok(ChatContextType::TaskExecution)
    ));
    assert!(parse_context_type("invalid").is_err());
}

#[test]
fn test_send_agent_message_response_from() {
    let result = SendResult {
        conversation_id: "conv-123".to_string(),
        agent_run_id: "run-456".to_string(),
        is_new_conversation: true,
        was_queued: false,
        queued_message_id: None,
        queued_as_pending: false,
    };

    let response = SendAgentMessageResponse::from(result);
    assert_eq!(response.conversation_id, "conv-123");
    assert_eq!(response.agent_run_id, "run-456");
    assert!(response.is_new_conversation);
    assert!(!response.was_queued);
    assert!(response.queued_message_id.is_none());
}

#[test]
fn test_send_agent_message_response_queued() {
    let result = SendResult {
        conversation_id: "conv-existing".to_string(),
        agent_run_id: "run-existing".to_string(),
        is_new_conversation: false,
        was_queued: true,
        queued_message_id: Some("queued-msg-123".to_string()),
        queued_as_pending: false,
    };

    let response = SendAgentMessageResponse::from(result);
    assert_eq!(response.conversation_id, "conv-existing");
    assert_eq!(response.agent_run_id, "run-existing");
    assert!(!response.is_new_conversation);
    assert!(response.was_queued);
    assert_eq!(response.queued_message_id.as_deref(), Some("queued-msg-123"));
}

#[test]
fn test_queued_message_response_from() {
    let msg = QueuedMessage::new("Test content".to_string());
    let response = QueuedMessageResponse::from(msg.clone());

    assert_eq!(response.id, msg.id);
    assert_eq!(response.content, "Test content");
    assert!(!response.is_editing);
}

#[test]
fn test_response_serialization() {
    let response = SendAgentMessageResponse {
        conversation_id: "conv-123".to_string(),
        agent_run_id: "run-456".to_string(),
        is_new_conversation: true,
        was_queued: false,
        queued_message_id: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("conversation_id")); // snake_case (Rust default)
    assert!(json.contains("agent_run_id"));
    assert!(json.contains("is_new_conversation"));
}

// ── AgentRunStatusResponse model field tests ──────────────────────────────────

#[test]
fn test_agent_run_status_response_serializes_model_present() {
    let response = AgentRunStatusResponse {
        id: "run-1".to_string(),
        conversation_id: "conv-1".to_string(),
        status: "running".to_string(),
        started_at: "2024-01-01T00:00:00Z".to_string(),
        completed_at: None,
        error_message: None,
        model_id: Some("claude-sonnet-4-6".to_string()),
        model_label: Some("Sonnet 4.6".to_string()),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains(r#""model_id":"claude-sonnet-4-6""#));
    assert!(json.contains(r#""model_label":"Sonnet 4.6""#));
}

#[test]
fn test_agent_run_status_response_serializes_model_absent() {
    let response = AgentRunStatusResponse {
        id: "run-2".to_string(),
        conversation_id: "conv-2".to_string(),
        status: "completed".to_string(),
        started_at: "2024-01-01T00:00:00Z".to_string(),
        completed_at: Some("2024-01-01T01:00:00Z".to_string()),
        error_message: None,
        model_id: None,
        model_label: None,
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains(r#""model_id":null"#));
    assert!(json.contains(r#""model_label":null"#));
}

// ── IPC contract tests ─────────────────────────────────────────────────────────
// Verify camelCase deserialization for unified chat command input structs.

#[cfg(test)]
mod ipc_contract {
    use ralphx_lib::commands::unified_chat_commands::{
        CreateAgentConversationInput, QueueAgentMessageInput, SendAgentMessageInput,
    };

    // ── SendAgentMessageInput ───────────────────────────────────────────────

    #[test]
    fn send_agent_message_input_deserializes_camel_case() {
        let json = r#"{"contextType":"task_execution","contextId":"task-123","content":"Hello agent","target":null}"#;
        let input: SendAgentMessageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.context_type, "task_execution");
        assert_eq!(input.context_id, "task-123");
        assert_eq!(input.content, "Hello agent");
        assert!(input.target.is_none());
    }

    #[test]
    fn send_agent_message_input_with_target() {
        let json = r#"{"contextType":"ideation","contextId":"session-456","content":"Plan this","target":"orchestrator"}"#;
        let input: SendAgentMessageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.context_type, "ideation");
        assert_eq!(input.context_id, "session-456");
        assert_eq!(input.target, Some("orchestrator".to_string()));
    }

    #[test]
    fn send_agent_message_input_snake_case_not_accepted() {
        // context_type in snake_case must not map to context_type field
        let json = r#"{"context_type":"task","context_id":"id-1","content":"msg"}"#;
        let result: Result<SendAgentMessageInput, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "snake_case context_type must not deserialize (missing required camelCase fields)"
        );
    }

    // ── QueueAgentMessageInput ──────────────────────────────────────────────

    #[test]
    fn queue_agent_message_input_deserializes_camel_case() {
        let json = r#"{"contextType":"task","contextId":"task-789","content":"Queued msg","clientId":"client-abc","target":null}"#;
        let input: QueueAgentMessageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.context_type, "task");
        assert_eq!(input.context_id, "task-789");
        assert_eq!(input.content, "Queued msg");
        assert_eq!(input.client_id, Some("client-abc".to_string()));
        assert!(input.target.is_none());
    }

    #[test]
    fn queue_agent_message_input_optional_fields_absent() {
        let json = r#"{"contextType":"project","contextId":"proj-1","content":"Hello"}"#;
        let input: QueueAgentMessageInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.context_type, "project");
        assert!(input.client_id.is_none());
        assert!(input.target.is_none());
    }

    // ── CreateAgentConversationInput ────────────────────────────────────────

    #[test]
    fn create_agent_conversation_input_deserializes_camel_case() {
        let json = r#"{"contextType":"review","contextId":"task-review-123"}"#;
        let input: CreateAgentConversationInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.context_type, "review");
        assert_eq!(input.context_id, "task-review-123");
    }

    #[test]
    fn create_agent_conversation_input_rejects_missing_fields() {
        let json = r#"{"contextType":"ideation"}"#;
        let result: Result<CreateAgentConversationInput, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "missing contextId must cause deserialization failure"
        );
    }
}
