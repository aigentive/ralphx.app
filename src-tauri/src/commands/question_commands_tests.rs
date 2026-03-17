use super::*;
use crate::application::{QuestionState, QuestionOption};

#[test]
fn test_resolve_question_args_deserialize() {
    let json = r#"{"requestId": "abc-123", "selectedOptions": ["opt1", "opt2"], "customResponse": "Custom answer"}"#;
    let args: ResolveQuestionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.request_id, "abc-123");
    assert_eq!(args.selected_options, vec!["opt1", "opt2"]);
    assert_eq!(args.custom_response, Some("Custom answer".to_string()));
}

#[test]
fn test_resolve_question_args_without_custom_response() {
    let json = r#"{"requestId": "abc-123", "selectedOptions": ["opt1"]}"#;
    let args: ResolveQuestionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.request_id, "abc-123");
    assert_eq!(args.selected_options, vec!["opt1"]);
    assert!(args.custom_response.is_none());
}

#[test]
fn test_resolve_question_response_serialize() {
    let response = ResolveQuestionResponse {
        success: true,
        message: Some("Resolved".to_string()),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"message\":\"Resolved\""));
}

/// Verify that resolve() returns (true, Some(session_id)) for a known question,
/// which is the condition that gates event emission in resolve_user_question.
#[tokio::test]
async fn test_resolve_returns_true_with_session_id_when_question_exists() {
    let state = QuestionState::new();
    state
        .register(
            "req-abc".to_string(),
            "session-xyz".to_string(),
            "Which option?".to_string(),
            None,
            vec![QuestionOption {
                value: "a".to_string(),
                label: "Option A".to_string(),
                description: None,
            }],
            false,
        )
        .await;

    let answer = QuestionAnswer {
        selected_options: vec!["a".to_string()],
        text: None,
    };
    let (resolved, session_id) = state.resolve("req-abc", answer).await;

    // emit path should be taken: resolved == true and session_id.is_some()
    assert!(resolved, "resolve should return true for a known request_id");
    assert_eq!(
        session_id,
        Some("session-xyz".to_string()),
        "session_id should match the registered session"
    );
}

/// Verify that resolve() returns (false, None) for an unknown question,
/// which means the event emission path is NOT taken.
#[tokio::test]
async fn test_resolve_returns_false_when_question_not_found() {
    let state = QuestionState::new();

    let answer = QuestionAnswer {
        selected_options: vec!["a".to_string()],
        text: None,
    };
    let (resolved, session_id) = state.resolve("nonexistent-req", answer).await;

    // emit path should NOT be taken: resolved == false
    assert!(!resolved, "resolve should return false for an unknown request_id");
    assert!(session_id.is_none(), "session_id should be None when not resolved");
}
