use super::*;

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
    };

    let response = SendAgentMessageResponse::from(result);
    assert_eq!(response.conversation_id, "conv-123");
    assert_eq!(response.agent_run_id, "run-456");
    assert!(response.is_new_conversation);
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
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("conversation_id")); // snake_case (Rust default)
    assert!(json.contains("agent_run_id"));
    assert!(json.contains("is_new_conversation"));
}
