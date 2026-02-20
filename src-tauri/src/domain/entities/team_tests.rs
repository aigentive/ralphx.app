use super::*;

#[test]
fn team_session_id_generates_unique() {
    let id1 = TeamSessionId::new();
    let id2 = TeamSessionId::new();
    assert_ne!(id1, id2);
}

#[test]
fn team_session_id_from_string() {
    let id = TeamSessionId::from_string("test-id");
    assert_eq!(id.as_str(), "test-id");
}

#[test]
fn team_message_id_generates_unique() {
    let id1 = TeamMessageId::new();
    let id2 = TeamMessageId::new();
    assert_ne!(id1, id2);
}

#[test]
fn team_session_new_defaults() {
    let session = TeamSession::new("my-team", "ctx-123", "task");
    assert_eq!(session.team_name, "my-team");
    assert_eq!(session.context_id, "ctx-123");
    assert_eq!(session.context_type, "task");
    assert_eq!(session.phase, "forming");
    assert!(session.teammates.is_empty());
    assert!(session.disbanded_at.is_none());
    assert!(session.lead_name.is_none());
}

#[test]
fn team_message_record_new_defaults() {
    let session_id = TeamSessionId::new();
    let msg = TeamMessageRecord::new(session_id.clone(), "worker-1", "hello");
    assert_eq!(msg.team_session_id, session_id);
    assert_eq!(msg.sender, "worker-1");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.message_type, "teammate_message");
    assert!(msg.recipient.is_none());
}

#[test]
fn teammate_snapshot_serializes() {
    let snap = TeammateSnapshot {
        name: "worker-1".to_string(),
        color: "#ff6b35".to_string(),
        model: "sonnet".to_string(),
        role: "coder".to_string(),
        status: "idle".to_string(),
        cost: TeammateCost {
            input_tokens: 1000,
            output_tokens: 500,
            cache_creation_tokens: 200,
            cache_read_tokens: 100,
            estimated_usd: 0.05,
        },
        spawned_at: "2024-01-01T00:00:00Z".to_string(),
        last_activity_at: "2024-01-01T00:01:00Z".to_string(),
    };
    let json = serde_json::to_string(&snap).unwrap();
    assert!(json.contains("worker-1"));

    let parsed: TeammateSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "worker-1");
    assert_eq!(parsed.color, "#ff6b35");
    assert_eq!(parsed.role, "coder");
    assert_eq!(parsed.cost.input_tokens, 1000);
}

#[test]
fn team_session_id_display() {
    let id = TeamSessionId::from_string("display-test");
    assert_eq!(format!("{}", id), "display-test");
}

#[test]
fn team_message_id_display() {
    let id = TeamMessageId::from_string("msg-display");
    assert_eq!(format!("{}", id), "msg-display");
}

#[test]
fn team_session_id_serializes() {
    let id = TeamSessionId::from_string("ser-test");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"ser-test\"");
}

#[test]
fn team_message_id_serializes() {
    let id = TeamMessageId::from_string("msg-ser");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "\"msg-ser\"");
}
