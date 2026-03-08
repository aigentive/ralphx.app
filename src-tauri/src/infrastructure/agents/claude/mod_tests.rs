/// TDD tests for --allowed-tools CLI arg injection in create_mcp_config (Wave 1).
/// These tests FAIL until Wave 3 implementation is complete.
///
/// Covers:
/// - validate_mcp_tool_name(): rejects names with commas/spaces/uppercase/digits-first
/// - format_allowed_tools_arg_value(): None→None, Some([])→"__NONE__", Some([...])→"t1,t2"
/// - create_mcp_config(): injects --allowed-tools from agent's mcp_tools list
/// - create_mcp_config(): --agent-type still present alongside --allowed-tools
/// - create_mcp_config(): no --allowed-tools arg when agent has no mcp_tools config
use super::*;
use std::path::Path;

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Create a minimal plugin dir structure that create_mcp_config() can use.
fn make_temp_plugin_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let plugin_dir = dir.path().to_path_buf();
    std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build")).unwrap();
    std::fs::write(plugin_dir.join("ralphx-mcp-server/build/index.js"), "// fake").unwrap();
    (dir, plugin_dir)
}

/// Parse the JSON args array from a generated MCP config temp file.
fn get_json_args(config_path: &Path) -> Vec<String> {
    let content = std::fs::read_to_string(config_path).expect("read config file");
    let v: serde_json::Value = serde_json::from_str(&content).expect("parse JSON");
    v.get("mcpServers")
        .and_then(|s| s.as_object())
        .and_then(|m| m.values().next())
        .and_then(|server| server.get("args"))
        .and_then(|args| args.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

// ─── validate_mcp_tool_name ──────────────────────────────────────────────────

#[test]
fn test_validate_mcp_tool_name_accepts_lowercase_alphanumeric_underscore() {
    assert!(validate_mcp_tool_name("get_session_plan"));
    assert!(validate_mcp_tool_name("tool1"));
    assert!(validate_mcp_tool_name("a"));
    assert!(validate_mcp_tool_name("abc123_def"));
    assert!(validate_mcp_tool_name("start_step"));
}

#[test]
fn test_validate_mcp_tool_name_rejects_names_with_spaces() {
    assert!(!validate_mcp_tool_name("has space"));
    assert!(!validate_mcp_tool_name(" leading"));
    assert!(!validate_mcp_tool_name("trailing "));
}

#[test]
fn test_validate_mcp_tool_name_rejects_names_with_commas() {
    assert!(!validate_mcp_tool_name("has,comma"));
    assert!(!validate_mcp_tool_name(",starts_with_comma"));
}

#[test]
fn test_validate_mcp_tool_name_rejects_uppercase() {
    assert!(!validate_mcp_tool_name("UPPERCASE"));
    assert!(!validate_mcp_tool_name("Mixed_Case"));
    assert!(!validate_mcp_tool_name("camelCase"));
}

#[test]
fn test_validate_mcp_tool_name_rejects_starting_with_digit() {
    assert!(!validate_mcp_tool_name("1starts_digit"));
    assert!(!validate_mcp_tool_name("9tool"));
}

#[test]
fn test_validate_mcp_tool_name_rejects_special_characters() {
    assert!(!validate_mcp_tool_name("has-hyphen"));
    assert!(!validate_mcp_tool_name("has.dot"));
    assert!(!validate_mcp_tool_name("has@at"));
    assert!(!validate_mcp_tool_name(""));
}

// ─── format_allowed_tools_arg_value ─────────────────────────────────────────

#[test]
fn test_format_allowed_tools_arg_value_with_tools_list() {
    let tools = vec!["tool1".to_string(), "tool2".to_string()];
    let result = format_allowed_tools_arg_value(Some(&tools));
    assert_eq!(result, Some("tool1,tool2".to_string()));
}

#[test]
fn test_format_allowed_tools_arg_value_single_tool() {
    let tools = vec!["get_session_plan".to_string()];
    let result = format_allowed_tools_arg_value(Some(&tools));
    assert_eq!(result, Some("get_session_plan".to_string()));
}

#[test]
fn test_format_allowed_tools_arg_value_explicit_empty_returns_none_sentinel() {
    let result = format_allowed_tools_arg_value(Some(&[]));
    assert_eq!(result, Some("__NONE__".to_string()));
}

#[test]
fn test_format_allowed_tools_arg_value_absent_mcp_tools_returns_none() {
    let result = format_allowed_tools_arg_value(None);
    assert_eq!(result, None);
}

// ─── create_mcp_config integration ──────────────────────────────────────────

#[test]
fn test_create_mcp_config_injects_allowed_tools_for_agent_with_mcp_tools() {
    let (_dir, plugin_dir) = make_temp_plugin_dir();
    // orchestrator-ideation has a non-empty mcp_tools list in ralphx.yaml
    let config_path = create_mcp_config(&plugin_dir, "orchestrator-ideation")
        .expect("should create config file");
    let args = get_json_args(&config_path);

    let allowed_tools_arg = args.iter().find(|a| a.starts_with("--allowed-tools="));
    assert!(
        allowed_tools_arg.is_some(),
        "--allowed-tools should be present for agent with mcp_tools; got args: {args:?}"
    );
    let value = allowed_tools_arg
        .unwrap()
        .strip_prefix("--allowed-tools=")
        .unwrap();
    assert!(!value.is_empty(), "--allowed-tools value should not be empty");
    assert_ne!(
        value, "__NONE__",
        "--allowed-tools should contain real tools, not __NONE__"
    );
}

#[test]
fn test_create_mcp_config_injects_agent_type_alongside_allowed_tools() {
    let (_dir, plugin_dir) = make_temp_plugin_dir();
    let config_path = create_mcp_config(&plugin_dir, "orchestrator-ideation")
        .expect("should create config file");
    let args = get_json_args(&config_path);

    // Both --agent-type and --allowed-tools must be present
    assert!(
        args.contains(&"--agent-type".to_string()),
        "--agent-type should be present; got: {args:?}"
    );
    assert!(
        args.iter().any(|a| a.starts_with("--allowed-tools=")),
        "--allowed-tools should be present; got: {args:?}"
    );
}

#[test]
fn test_create_mcp_config_no_allowed_tools_arg_for_unknown_agent() {
    let (_dir, plugin_dir) = make_temp_plugin_dir();
    // Unknown agent has no config → mcp_tools absent → no --allowed-tools injected
    let config_path = create_mcp_config(&plugin_dir, "completely-unknown-agent-xyz")
        .expect("should create config file even for unknown agent");
    let args = get_json_args(&config_path);

    let has_allowed_tools = args.iter().any(|a| a.starts_with("--allowed-tools="));
    assert!(
        !has_allowed_tools,
        "--allowed-tools should NOT be present for agent with no mcp_tools config; got: {args:?}"
    );
    // --agent-type should still be present
    assert!(
        args.contains(&"--agent-type".to_string()),
        "--agent-type should still be present; got: {args:?}"
    );
}

#[test]
fn test_create_mcp_config_allowed_tools_value_matches_agent_mcp_tools() {
    let (_dir, plugin_dir) = make_temp_plugin_dir();
    // session-namer has a small mcp_tools list: [update_session_title]
    let config_path = create_mcp_config(&plugin_dir, "session-namer")
        .expect("should create config file");
    let args = get_json_args(&config_path);

    let allowed_arg = args
        .iter()
        .find(|a| a.starts_with("--allowed-tools="))
        .expect("--allowed-tools should be present for session-namer");
    let value = allowed_arg.strip_prefix("--allowed-tools=").unwrap();
    // session-namer has mcp_tools: [update_session_title]
    assert_eq!(
        value, "update_session_title",
        "session-namer should have exactly update_session_title"
    );
}
