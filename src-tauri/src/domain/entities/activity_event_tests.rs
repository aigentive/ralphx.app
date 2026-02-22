use super::*;

#[test]
fn activity_event_id_generates_unique_ids() {
    let id1 = ActivityEventId::new();
    let id2 = ActivityEventId::new();
    assert_ne!(id1, id2);
}

#[test]
fn activity_event_id_from_string_preserves_value() {
    let id = ActivityEventId::from_string("test-id");
    assert_eq!(id.as_str(), "test-id");
}

#[test]
fn activity_event_type_display() {
    assert_eq!(ActivityEventType::Thinking.to_string(), "thinking");
    assert_eq!(ActivityEventType::ToolCall.to_string(), "tool_call");
    assert_eq!(ActivityEventType::ToolResult.to_string(), "tool_result");
    assert_eq!(ActivityEventType::Text.to_string(), "text");
    assert_eq!(ActivityEventType::Error.to_string(), "error");
}

#[test]
fn activity_event_type_parsing() {
    assert_eq!(
        "thinking".parse::<ActivityEventType>().unwrap(),
        ActivityEventType::Thinking
    );
    assert_eq!(
        "tool_call".parse::<ActivityEventType>().unwrap(),
        ActivityEventType::ToolCall
    );
    assert_eq!(
        "tool_result".parse::<ActivityEventType>().unwrap(),
        ActivityEventType::ToolResult
    );
    assert_eq!(
        "text".parse::<ActivityEventType>().unwrap(),
        ActivityEventType::Text
    );
    assert_eq!(
        "error".parse::<ActivityEventType>().unwrap(),
        ActivityEventType::Error
    );
    assert!("invalid".parse::<ActivityEventType>().is_err());
}

#[test]
fn activity_event_role_display() {
    assert_eq!(ActivityEventRole::Agent.to_string(), "agent");
    assert_eq!(ActivityEventRole::System.to_string(), "system");
    assert_eq!(ActivityEventRole::User.to_string(), "user");
}

#[test]
fn activity_event_role_parsing() {
    assert_eq!(
        "agent".parse::<ActivityEventRole>().unwrap(),
        ActivityEventRole::Agent
    );
    assert_eq!(
        "system".parse::<ActivityEventRole>().unwrap(),
        ActivityEventRole::System
    );
    assert_eq!(
        "user".parse::<ActivityEventRole>().unwrap(),
        ActivityEventRole::User
    );
    assert!("invalid".parse::<ActivityEventRole>().is_err());
}

#[test]
fn activity_event_role_default() {
    assert_eq!(ActivityEventRole::default(), ActivityEventRole::Agent);
}

#[test]
fn new_task_event_creates_correct_event() {
    let task_id = TaskId::new();
    let event =
        ActivityEvent::new_task_event(task_id.clone(), ActivityEventType::Thinking, "test content");

    assert_eq!(event.task_id, Some(task_id));
    assert_eq!(event.ideation_session_id, None);
    assert_eq!(event.event_type, ActivityEventType::Thinking);
    assert_eq!(event.role, ActivityEventRole::Agent);
    assert_eq!(event.content, "test content");
    assert_eq!(event.metadata, None);
    assert_eq!(event.internal_status, None);
}

#[test]
fn new_session_event_creates_correct_event() {
    let session_id = IdeationSessionId::new();
    let event = ActivityEvent::new_session_event(
        session_id.clone(),
        ActivityEventType::Text,
        "session content",
    );

    assert_eq!(event.task_id, None);
    assert_eq!(event.ideation_session_id, Some(session_id));
    assert_eq!(event.event_type, ActivityEventType::Text);
    assert_eq!(event.role, ActivityEventRole::Agent);
    assert_eq!(event.content, "session content");
}

#[test]
fn with_status_sets_status() {
    let task_id = TaskId::new();
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Text, "content")
        .with_status(InternalStatus::Executing);

    assert_eq!(event.internal_status, Some(InternalStatus::Executing));
}

#[test]
fn with_role_sets_role() {
    let task_id = TaskId::new();
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Error, "error")
        .with_role(ActivityEventRole::System);

    assert_eq!(event.role, ActivityEventRole::System);
}

#[test]
fn with_metadata_sets_metadata() {
    let task_id = TaskId::new();
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::ToolResult, "result")
        .with_metadata(r#"{"tool_use_id": "abc123"}"#);

    assert_eq!(
        event.metadata,
        Some(r#"{"tool_use_id": "abc123"}"#.to_string())
    );
}

#[test]
fn activity_event_serializes_to_json() {
    let task_id = TaskId::from_string("task-123".to_string());
    let event = ActivityEvent::new_task_event(task_id, ActivityEventType::Thinking, "test");

    let json = serde_json::to_string(&event).expect("Should serialize");
    assert!(json.contains("\"event_type\":\"thinking\""));
    assert!(json.contains("\"role\":\"agent\""));
}
