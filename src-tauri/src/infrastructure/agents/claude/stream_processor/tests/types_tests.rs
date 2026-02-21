use super::*;

#[test]
fn test_tool_call_serialization() {
    let tool_call = ToolCall {
        id: Some("toolu_01ABC".to_string()),
        name: "create_task_proposal".to_string(),
        arguments: serde_json::json!({"title": "Test task"}),
        result: None,
        diff_context: None,
    };

    let json = serde_json::to_string(&tool_call).unwrap();
    assert!(json.contains("toolu_01ABC"));
    assert!(json.contains("create_task_proposal"));
    // diff_context: None should be skipped via skip_serializing_if
    assert!(!json.contains("diff_context"));

    let parsed: ToolCall = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "create_task_proposal");
    assert!(parsed.diff_context.is_none());
}

#[test]
fn test_tool_call_with_diff_context_serialization() {
    let tool_call = ToolCall {
        id: Some("toolu_02DEF".to_string()),
        name: "Edit".to_string(),
        arguments: serde_json::json!({"file_path": "src/main.rs", "old_string": "old", "new_string": "new"}),
        result: None,
        diff_context: Some(DiffContext {
            old_content: Some("fn main() {\n    old\n}\n".to_string()),
            file_path: "/project/src/main.rs".to_string(),
        }),
    };

    let json = serde_json::to_string(&tool_call).unwrap();
    assert!(json.contains("diff_context"));
    assert!(json.contains("old_content"));
    assert!(json.contains("/project/src/main.rs"));

    let parsed: ToolCall = serde_json::from_str(&json).unwrap();
    assert!(parsed.diff_context.is_some());
    let ctx = parsed.diff_context.unwrap();
    assert_eq!(ctx.file_path, "/project/src/main.rs");
    assert!(ctx.old_content.is_some());
}

#[test]
fn test_tool_call_diff_context_new_file() {
    let tool_call = ToolCall {
        id: Some("toolu_03GHI".to_string()),
        name: "Write".to_string(),
        arguments: serde_json::json!({"file_path": "src/new.rs", "content": "fn new() {}"}),
        result: None,
        diff_context: Some(DiffContext {
            old_content: None,
            file_path: "/project/src/new.rs".to_string(),
        }),
    };

    let json = serde_json::to_string(&tool_call).unwrap();
    let parsed: ToolCall = serde_json::from_str(&json).unwrap();
    let ctx = parsed.diff_context.unwrap();
    assert!(ctx.old_content.is_none());
    assert_eq!(ctx.file_path, "/project/src/new.rs");
}

#[test]
fn test_parse_usage_text_basic() {
    let text = "Some output\nagentId: a7db0f4 (for resuming...)\n<usage>total_tokens: 12345\ntool_uses: 8\nduration_ms: 45000</usage>";
    let (agent_id, duration, tokens, tools) = parse_usage_text(text);

    assert_eq!(agent_id, Some("a7db0f4".to_string()));
    assert_eq!(duration, Some(45000));
    assert_eq!(tokens, Some(12345));
    assert_eq!(tools, Some(8));
}

#[test]
fn test_parse_usage_text_no_usage_block() {
    let text = "Just some plain text output\nagentId: abc123";
    let (agent_id, duration, tokens, tools) = parse_usage_text(text);

    assert_eq!(agent_id, Some("abc123".to_string()));
    assert_eq!(duration, None);
    assert_eq!(tokens, None);
    assert_eq!(tools, None);
}

#[test]
fn test_parse_usage_text_no_agent_id() {
    let text = "<usage>total_tokens: 500\ntool_uses: 2\nduration_ms: 3000</usage>";
    let (agent_id, duration, tokens, tools) = parse_usage_text(text);

    assert_eq!(agent_id, None);
    assert_eq!(duration, Some(3000));
    assert_eq!(tokens, Some(500));
    assert_eq!(tools, Some(2));
}

#[test]
fn test_value_to_text_string() {
    let val = serde_json::json!("plain text result");
    assert_eq!(value_to_text(&val), "plain text result");
}

#[test]
fn test_value_to_text_content_blocks() {
    let val = serde_json::json!([
        {"type": "text", "text": "output line 1"},
        {"type": "tool_use", "id": "t1", "name": "Read"},
        {"type": "text", "text": "agentId: abc\n<usage>total_tokens: 100\ntool_uses: 1\nduration_ms: 2000</usage>"}
    ]);
    let text = value_to_text(&val);
    assert!(text.contains("output line 1"));
    assert!(text.contains("<usage>"));
    assert!(text.contains("agentId: abc"));
}
