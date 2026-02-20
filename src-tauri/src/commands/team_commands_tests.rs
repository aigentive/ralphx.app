use super::*;

#[test]
fn test_send_team_message_input_deserialize() {
    let json = r#"{"teamName":"my-team","content":"Hello"}"#;
    let input: SendTeamMessageInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.team_name, "my-team");
    assert_eq!(input.content, "Hello");
}

#[test]
fn test_send_teammate_message_input_deserialize() {
    let json =
        r#"{"teamName":"my-team","teammateName":"coder-1","content":"Hello teammate"}"#;
    let input: SendTeammateMessageInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.team_name, "my-team");
    assert_eq!(input.teammate_name, "coder-1");
    assert_eq!(input.content, "Hello teammate");
}

#[test]
fn test_create_team_input_deserialize() {
    let json = r#"{"teamName":"alpha","contextType":"ideation","contextId":"session-123"}"#;
    let input: CreateTeamInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.team_name, "alpha");
    assert_eq!(input.context_type, "ideation");
    assert_eq!(input.context_id, "session-123");
}
