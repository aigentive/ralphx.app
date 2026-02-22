use super::*;

#[test]
fn test_tool_call_info_new() {
    let info = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
    assert_eq!(info.tool_name, "Write");
    assert!(info.success);
    assert!(info.error.is_none());
}

#[test]
fn test_tool_call_info_failed() {
    let info = ToolCallInfo::failed("Write", r#"{"path": "test.txt"}"#, "Permission denied");
    assert!(!info.success);
    assert_eq!(info.error, Some("Permission denied".to_string()));
}

#[test]
fn test_tool_call_is_similar() {
    let info1 = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
    let info2 = ToolCallInfo::new("Write", r#"{"path": "test.txt"}"#);
    let info3 = ToolCallInfo::new("Write", r#"{"path": "other.txt"}"#);
    let info4 = ToolCallInfo::new("Read", r#"{"path": "test.txt"}"#);

    assert!(info1.is_similar_to(&info2));
    assert!(!info1.is_similar_to(&info3));
    assert!(!info1.is_similar_to(&info4));
}

#[test]
fn test_error_info_new() {
    let info = ErrorInfo::new("File not found", "Read");
    assert!(info.recoverable);
}

#[test]
fn test_error_info_fatal() {
    let info = ErrorInfo::fatal("System crash", "Kernel");
    assert!(!info.recoverable);
}

#[test]
fn test_progress_info_new() {
    let info = ProgressInfo::new();
    assert!(!info.has_progress());
}

#[test]
fn test_progress_info_has_progress() {
    let mut info = ProgressInfo::new();
    assert!(!info.has_progress());

    info.has_file_changes = true;
    assert!(info.has_progress());

    info.has_file_changes = false;
    info.has_new_commits = true;
    assert!(info.has_progress());

    info.has_new_commits = false;
    info.files_modified = 1;
    assert!(info.has_progress());
}

#[test]
fn test_supervisor_event_task_start() {
    let event = SupervisorEvent::task_start("task-123", "worker");
    assert_eq!(event.task_id(), "task-123");
    assert!(
        matches!(&event, SupervisorEvent::TaskStart { agent_role, .. } if agent_role == "worker"),
        "Expected TaskStart event with agent_role 'worker'"
    );
}

#[test]
fn test_supervisor_event_tool_call() {
    let info = ToolCallInfo::new("Write", "{}");
    let event = SupervisorEvent::tool_call("task-123", info.clone());
    assert_eq!(event.task_id(), "task-123");
    assert!(
        matches!(&event, SupervisorEvent::ToolCall { info: event_info, .. } if event_info.tool_name == "Write"),
        "Expected ToolCall event with tool_name 'Write'"
    );
}

#[test]
fn test_supervisor_event_error() {
    let info = ErrorInfo::new("Error message", "Source");
    let event = SupervisorEvent::error("task-123", info);
    assert_eq!(event.task_id(), "task-123");
}

#[test]
fn test_supervisor_event_progress_tick() {
    let info = ProgressInfo::new();
    let event = SupervisorEvent::progress_tick("task-123", info);
    assert_eq!(event.task_id(), "task-123");
}

#[test]
fn test_supervisor_event_token_threshold() {
    let event = SupervisorEvent::token_threshold("task-123", 60000, 50000);
    assert_eq!(event.task_id(), "task-123");
    assert!(
        matches!(&event, SupervisorEvent::TokenThreshold { tokens_used, threshold, .. }
            if *tokens_used == 60000 && *threshold == 50000),
        "Expected TokenThreshold event with tokens_used=60000, threshold=50000"
    );
}

#[test]
fn test_supervisor_event_time_threshold() {
    let event = SupervisorEvent::time_threshold("task-123", 15, 10);
    assert_eq!(event.task_id(), "task-123");
    assert!(
        matches!(&event, SupervisorEvent::TimeThreshold { elapsed_minutes, threshold_minutes, .. }
            if *elapsed_minutes == 15 && *threshold_minutes == 10),
        "Expected TimeThreshold event with elapsed_minutes=15, threshold_minutes=10"
    );
}

#[test]
fn test_supervisor_event_serialize() {
    let event = SupervisorEvent::task_start("task-123", "worker");
    let json = serde_json::to_string(&event).expect("Failed to serialize SupervisorEvent");
    assert!(json.contains("\"type\":\"task_start\""));
    assert!(json.contains("\"task_id\":\"task-123\""));
}

#[test]
fn test_supervisor_event_deserialize() {
    let json = r#"{
        "type": "task_start",
        "task_id": "task-123",
        "agent_role": "worker",
        "timestamp": "2026-01-24T10:00:00Z"
    }"#;
    let event: SupervisorEvent =
        serde_json::from_str(json).expect("Failed to deserialize SupervisorEvent");
    assert_eq!(event.task_id(), "task-123");
}

#[test]
fn test_supervisor_event_roundtrip() {
    let original = SupervisorEvent::token_threshold("task-456", 75000, 50000);
    let json = serde_json::to_string(&original).expect("Failed to serialize SupervisorEvent");
    let restored: SupervisorEvent =
        serde_json::from_str(&json).expect("Failed to deserialize SupervisorEvent");
    assert_eq!(original.task_id(), restored.task_id());
}
