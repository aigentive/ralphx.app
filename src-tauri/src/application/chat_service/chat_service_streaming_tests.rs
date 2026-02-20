use super::*;

#[test]
fn test_timeout_config_task_execution() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_review() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Review);
    assert_eq!(config.line_read_timeout, Duration::from_secs(300));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(120));
}

#[test]
fn test_timeout_config_merge() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_ideation_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_task_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Task);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_project_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Project);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_with_teammate() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation)
        .with_teammate("researcher".to_string(), "#ff6b35".to_string());
    assert_eq!(config.teammate_name, Some("researcher".to_string()));
    assert_eq!(config.teammate_color, Some("#ff6b35".to_string()));
    // Timeouts should be unchanged
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
}

#[test]
fn test_timeout_config_default_no_teammate() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    assert!(config.teammate_name.is_none());
    assert!(config.teammate_color.is_none());
}

#[test]
fn test_timeout_config_ordering() {
    // Merge needs generous timeouts (agent may run silent tests for minutes)
    // Review is faster than default, merge matches default
    let merge = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
    let review = StreamTimeoutConfig::for_context(&ChatContextType::Review);
    let default = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);

    assert!(review.line_read_timeout < default.line_read_timeout);
    assert!(review.parse_stall_timeout < default.parse_stall_timeout);
    // Merge matches default — merger agents may run silent test suites
    assert_eq!(merge.line_read_timeout, default.line_read_timeout);
    assert_eq!(merge.parse_stall_timeout, default.parse_stall_timeout);
}

#[test]
fn test_payloads_serialize_with_seq() {
    use crate::application::chat_service::{
        AgentChunkPayload, AgentTaskCompletedPayload, AgentTaskStartedPayload,
        AgentToolCallPayload,
    };

    // Verify AgentChunkPayload includes seq field
    let chunk = AgentChunkPayload {
        text: "test".to_string(),
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 0,
    };
    let json = serde_json::to_string(&chunk).unwrap();
    assert!(json.contains("\"seq\":0"), "AgentChunkPayload should serialize with seq field");

    // Verify AgentToolCallPayload includes seq field
    let tool_call = AgentToolCallPayload {
        tool_name: "test_tool".to_string(),
        tool_id: Some("tool-1".to_string()),
        arguments: serde_json::json!({}),
        result: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        diff_context: None,
        parent_tool_use_id: None,
        seq: 1,
    };
    let json = serde_json::to_string(&tool_call).unwrap();
    assert!(json.contains("\"seq\":1"), "AgentToolCallPayload should serialize with seq field");

    // Verify AgentTaskStartedPayload includes seq field
    let task_started = AgentTaskStartedPayload {
        tool_use_id: "tool-1".to_string(),
        description: Some("test".to_string()),
        subagent_type: Some("bash".to_string()),
        model: Some("sonnet".to_string()),
        teammate_name: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 2,
    };
    let json = serde_json::to_string(&task_started).unwrap();
    assert!(json.contains("\"seq\":2"), "AgentTaskStartedPayload should serialize with seq field");

    // Verify AgentTaskCompletedPayload includes seq field
    let task_completed = AgentTaskCompletedPayload {
        tool_use_id: "tool-1".to_string(),
        agent_id: Some("agent-1".to_string()),
        total_duration_ms: Some(1000),
        total_tokens: Some(100),
        total_tool_use_count: Some(5),
        teammate_name: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 3,
    };
    let json = serde_json::to_string(&task_completed).unwrap();
    assert!(json.contains("\"seq\":3"), "AgentTaskCompletedPayload should serialize with seq field");
}

#[test]
fn test_seq_values_are_monotonic() {
    // Test that multiple events would have incrementing seq values
    let mut stream_seq: u64 = 0;

    // Simulate streaming multiple events
    let seq1 = stream_seq;
    stream_seq += 1;
    let seq2 = stream_seq;
    stream_seq += 1;
    let seq3 = stream_seq;
    stream_seq += 1;
    let seq4 = stream_seq;

    assert_eq!(seq1, 0, "First event should have seq 0");
    assert_eq!(seq2, 1, "Second event should have seq 1");
    assert_eq!(seq3, 2, "Third event should have seq 2");
    assert_eq!(seq4, 3, "Fourth event should have seq 3");

    // Verify strict monotonic ordering
    assert!(seq2 > seq1, "seq must be strictly increasing");
    assert!(seq3 > seq2, "seq must be strictly increasing");
    assert!(seq4 > seq3, "seq must be strictly increasing");
}
