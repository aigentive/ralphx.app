use super::*;

#[test]
fn test_resolve_permission_args_deserialize() {
    let json = r#"{"request_id": "abc-123", "decision": "allow", "message": "User approved"}"#;
    let args: ResolvePermissionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.request_id, "abc-123");
    assert_eq!(args.decision, "allow");
    assert_eq!(args.message, Some("User approved".to_string()));
}

#[test]
fn test_resolve_permission_args_without_message() {
    let json = r#"{"request_id": "abc-123", "decision": "deny"}"#;
    let args: ResolvePermissionArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.request_id, "abc-123");
    assert_eq!(args.decision, "deny");
    assert!(args.message.is_none());
}

#[test]
fn test_resolve_permission_response_serialize() {
    let response = ResolvePermissionResponse {
        success: true,
        message: Some("Resolved".to_string()),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"message\":\"Resolved\""));
}
