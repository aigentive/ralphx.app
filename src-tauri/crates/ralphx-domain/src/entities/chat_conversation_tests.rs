use super::*;

#[test]
fn test_conversation_id_creation() {
    let id1 = ChatConversationId::new();
    let id2 = ChatConversationId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_conversation_id_from_string() {
    let id = ChatConversationId::new();
    let str_id = id.to_string();
    let parsed_id: ChatConversationId = str_id.parse().unwrap();
    assert_eq!(id, parsed_id);
}

#[test]
fn test_context_type_serialization() {
    assert_eq!(ChatContextType::Ideation.to_string(), "ideation");
    assert_eq!(ChatContextType::Task.to_string(), "task");
    assert_eq!(ChatContextType::Project.to_string(), "project");
    assert_eq!(ChatContextType::TaskExecution.to_string(), "task_execution");
    assert_eq!(ChatContextType::Review.to_string(), "review");
}

#[test]
fn test_context_type_parsing() {
    assert_eq!(
        "ideation".parse::<ChatContextType>().unwrap(),
        ChatContextType::Ideation
    );
    assert_eq!(
        "task".parse::<ChatContextType>().unwrap(),
        ChatContextType::Task
    );
    assert_eq!(
        "project".parse::<ChatContextType>().unwrap(),
        ChatContextType::Project
    );
    assert_eq!(
        "task_execution".parse::<ChatContextType>().unwrap(),
        ChatContextType::TaskExecution
    );
    assert_eq!(
        "review".parse::<ChatContextType>().unwrap(),
        ChatContextType::Review
    );
    assert!("invalid".parse::<ChatContextType>().is_err());
}

#[test]
fn test_new_ideation_conversation() {
    let session_id = IdeationSessionId::new();
    let expected_context_id = session_id.as_str().to_string();
    let conv = ChatConversation::new_ideation(session_id);

    assert_eq!(conv.context_type, ChatContextType::Ideation);
    assert_eq!(conv.context_id, expected_context_id);
    assert_eq!(conv.claude_session_id, None);
    assert_eq!(conv.message_count, 0);
    assert!(!conv.has_claude_session());
}

#[test]
fn test_set_claude_session_id() {
    let session_id = IdeationSessionId::new();
    let mut conv = ChatConversation::new_ideation(session_id);

    conv.set_claude_session_id("550e8400-e29b-41d4-a716-446655440000");
    assert!(conv.has_claude_session());
    assert_eq!(
        conv.claude_session_id,
        Some("550e8400-e29b-41d4-a716-446655440000".to_string())
    );
}

#[test]
fn test_set_title() {
    let session_id = IdeationSessionId::new();
    let mut conv = ChatConversation::new_ideation(session_id);

    conv.set_title("Dark mode implementation");
    assert_eq!(conv.display_title(), "Dark mode implementation");
}

#[test]
fn test_display_title_default() {
    let session_id = IdeationSessionId::new();
    let conv = ChatConversation::new_ideation(session_id);
    assert_eq!(conv.display_title(), "Untitled conversation");
}

#[test]
fn test_new_task_execution_conversation() {
    let task_id = TaskId::new();
    let expected_context_id = task_id.as_str().to_string();
    let conv = ChatConversation::new_task_execution(task_id);

    assert_eq!(conv.context_type, ChatContextType::TaskExecution);
    assert_eq!(conv.context_id, expected_context_id);
    assert_eq!(conv.claude_session_id, None);
    assert_eq!(conv.message_count, 0);
    assert!(!conv.has_claude_session());
}

#[test]
fn test_new_review_conversation() {
    let task_id = TaskId::new();
    let expected_context_id = task_id.as_str().to_string();
    let conv = ChatConversation::new_review(task_id);

    assert_eq!(conv.context_type, ChatContextType::Review);
    assert_eq!(conv.context_id, expected_context_id);
    assert_eq!(conv.claude_session_id, None);
    assert_eq!(conv.message_count, 0);
    assert!(!conv.has_claude_session());
}
