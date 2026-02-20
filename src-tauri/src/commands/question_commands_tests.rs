use super::*;

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
