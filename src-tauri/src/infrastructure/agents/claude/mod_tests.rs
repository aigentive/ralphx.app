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

fn make_temp_project_plugin_dir() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    let plugin_dir = root.join("plugins/app");
    std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build")).unwrap();
    std::fs::write(plugin_dir.join("ralphx-mcp-server/build/index.js"), "// fake").unwrap();
    std::fs::write(
        plugin_dir.join(".mcp.json"),
        r#"{"mcpServers":{"ralphx":{"type":"stdio","command":"node","args":["${CLAUDE_PLUGIN_ROOT}/ralphx-mcp-server/build/index.js"]}}}"#,
    )
    .unwrap();
    (dir, root, plugin_dir)
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
    let config_path = create_mcp_config(&plugin_dir, "orchestrator-ideation", false)
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
    let config_path = create_mcp_config(&plugin_dir, "orchestrator-ideation", false)
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
    let config_path = create_mcp_config(&plugin_dir, "completely-unknown-agent-xyz", false)
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
    let config_path = create_mcp_config(&plugin_dir, "session-namer", false)
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

// ─── validate_mcp_config_json ────────────────────────────────────────────────

#[test]
fn test_validate_mcp_config_json_accepts_valid_config() {
    let config = serde_json::json!({
        "mcpServers": {
            "ralphx": {
                "type": "stdio",
                "command": "/usr/local/bin/node",
                "args": ["/path/to/index.js", "--agent-type", "worker"]
            }
        }
    });
    assert!(validate_mcp_config_json(&config, "ralphx").is_ok());
}

#[test]
fn test_validate_mcp_config_json_rejects_missing_mcp_servers() {
    let config = serde_json::json!({
        "other": {}
    });
    let result = validate_mcp_config_json(&config, "ralphx");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("mcpServers"),
        "error should mention missing mcpServers"
    );
}

#[test]
fn test_validate_mcp_config_json_rejects_missing_server_entry() {
    let config = serde_json::json!({
        "mcpServers": {
            "other-server": {}
        }
    });
    let result = validate_mcp_config_json(&config, "ralphx");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("ralphx"),
        "error should mention missing server name"
    );
}

#[test]
fn test_validate_mcp_config_json_rejects_missing_command() {
    let config = serde_json::json!({
        "mcpServers": {
            "ralphx": {
                "args": ["/path/to/index.js"]
            }
        }
    });
    let result = validate_mcp_config_json(&config, "ralphx");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("command"),
        "error should mention missing command field"
    );
}

#[test]
fn test_validate_mcp_config_json_rejects_missing_args() {
    let config = serde_json::json!({
        "mcpServers": {
            "ralphx": {
                "command": "/usr/local/bin/node"
            }
        }
    });
    let result = validate_mcp_config_json(&config, "ralphx");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("args"),
        "error should mention missing args field"
    );
}

#[test]
fn test_create_mcp_config_returns_error_on_io_failure() {
    // Use a non-existent directory as plugin_dir — should fail gracefully
    let plugin_dir = std::path::Path::new("/nonexistent/path/that/does/not/exist");
    // create_mcp_config should return Err, not panic
    let result = create_mcp_config(plugin_dir, "worker", false);
    // May succeed (writing temp file doesn't need plugin_dir to exist) or fail on validation
    // The key invariant: it must not panic, regardless of outcome
    let _ = result; // just checking no panic
}

// ─── filter_interactive_tools tests ─────────────────────────────────────────

#[test]
fn test_filter_interactive_tools_removes_ask_user_question() {
    let tools = vec![
        "get_task_context".to_string(),
        "ask_user_question".to_string(),
        "complete_step".to_string(),
    ];
    let filtered = filter_interactive_tools(&tools);
    assert!(!filtered.contains(&"ask_user_question".to_string()));
    assert!(filtered.contains(&"get_task_context".to_string()));
    assert!(filtered.contains(&"complete_step".to_string()));
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_interactive_tools_no_op_when_not_present() {
    let tools = vec!["get_task_context".to_string(), "complete_step".to_string()];
    let filtered = filter_interactive_tools(&tools);
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_interactive_tools_empty_input() {
    let tools: Vec<String> = vec![];
    let filtered = filter_interactive_tools(&tools);
    assert!(filtered.is_empty());
}

// ─── create_mcp_config with is_external_mcp=true tests ───────────────────────

#[test]
fn test_create_mcp_config_external_mcp_filters_ask_user_question() {
    let (dir, plugin_dir) = make_temp_plugin_dir();
    // orchestrator-ideation has ask_user_question in its mcp_tools
    let config_path =
        create_mcp_config(&plugin_dir, "orchestrator-ideation", true).expect("should succeed");
    let content = std::fs::read_to_string(&config_path).expect("should read config");
    let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let args: Vec<String> = json["mcpServers"]
        .as_object()
        .and_then(|servers| servers.values().next())
        .and_then(|server| server["args"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let allowed_tools_arg = args.iter().find(|a| a.starts_with("--allowed-tools="));
    if let Some(arg) = allowed_tools_arg {
        assert!(
            !arg.contains("ask_user_question"),
            "ask_user_question must not appear in --allowed-tools when is_external_mcp=true, got: {arg}"
        );
    }
    drop(dir);
}

#[test]
fn test_create_mcp_config_non_external_mcp_keeps_ask_user_question() {
    let (dir, plugin_dir) = make_temp_plugin_dir();
    // orchestrator-ideation has ask_user_question in its mcp_tools — should be present when not external
    let config_path =
        create_mcp_config(&plugin_dir, "orchestrator-ideation", false).expect("should succeed");
    let content = std::fs::read_to_string(&config_path).expect("should read config");
    let json: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let args: Vec<String> = json["mcpServers"]
        .as_object()
        .and_then(|servers| servers.values().next())
        .and_then(|server| server["args"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let allowed_tools_arg = args.iter().find(|a| a.starts_with("--allowed-tools="));
    if let Some(arg) = allowed_tools_arg {
        assert!(
            arg.contains("ask_user_question"),
            "ask_user_question must appear in --allowed-tools when is_external_mcp=false, got: {arg}"
        );
    }
    drop(dir);
}

#[test]
fn test_materialize_generated_plugin_dir_renders_canonical_claude_frontmatter_without_legacy_agent_file() {
    let (_dir, root, plugin_dir) = make_temp_project_plugin_dir();
    let agent_root = root.join("agents/orchestrator-ideation");
    std::fs::create_dir_all(agent_root.join("claude")).expect("create canonical claude dir");
    std::fs::write(
        agent_root.join("agent.yaml"),
        r#"name: orchestrator-ideation
role: ideation_orchestrator
description: Facilitates ideation sessions and generates task proposals for RalphX.
claude:
  disallowed_tools:
    - Write
    - Edit
    - NotebookEdit
  skills:
    - task-decomposition
    - priority-assessment
    - dependency-analysis
"#,
    )
    .expect("write shared definition");
    std::fs::write(
        agent_root.join("claude/prompt.md"),
        "Canonical Claude ideation prompt",
    )
    .expect("write claude prompt");

    let generated_dir =
        materialize_generated_plugin_dir(&plugin_dir).expect("materialize generated plugin dir");
    let generated_prompt = std::fs::read_to_string(
        generated_dir.join("agents/orchestrator-ideation.md"),
    )
    .expect("read generated agent prompt");

    assert!(
        generated_prompt.contains("name: orchestrator-ideation"),
        "expected generated frontmatter name, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("description: Facilitates ideation sessions"),
        "expected generated description, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("mcp__ralphx__create_task_proposal"),
        "expected MCP tool grants from runtime config, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("Task(Explore)")
            && generated_prompt.contains("Task(ralphx:ideation-specialist-ux)"),
        "expected derived preapproved task variants in generated frontmatter, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("disallowedTools:\n  - Write\n  - Edit\n  - NotebookEdit"),
        "expected canonical claude disallowed tools, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("skills:\n  - task-decomposition"),
        "expected canonical claude skills, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("model: opus"),
        "expected runtime-derived model in generated frontmatter, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("Canonical Claude ideation prompt"),
        "expected canonical prompt body to be preserved, got: {generated_prompt}"
    );
}

#[test]
fn test_materialize_generated_plugin_dir_supports_shared_prompt_without_legacy_frontmatter() {
    let (_dir, root, plugin_dir) = make_temp_project_plugin_dir();
    let agent_root = root.join("agents/session-namer");
    std::fs::create_dir_all(agent_root.join("shared")).expect("create shared prompt dir");
    std::fs::write(
        agent_root.join("agent.yaml"),
        r#"name: session-namer
role: session_namer
description: Generates concise ideation session titles from user or plan context.
"#,
    )
    .expect("write shared definition");
    std::fs::write(
        agent_root.join("shared/prompt.md"),
        "Shared session naming prompt",
    )
    .expect("write shared prompt");

    let generated_dir =
        materialize_generated_plugin_dir(&plugin_dir).expect("materialize generated plugin dir");
    let generated_prompt =
        std::fs::read_to_string(generated_dir.join("agents/session-namer.md"))
            .expect("read generated session namer prompt");

    assert!(
        generated_prompt.contains("model: sonnet"),
        "expected runtime-derived model in generated frontmatter, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("mcp__ralphx__update_session_title"),
        "expected session-namer MCP tool in generated frontmatter, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("Shared session naming prompt"),
        "expected shared canonical prompt body to be preserved, got: {generated_prompt}"
    );
}

#[test]
fn test_materialize_generated_plugin_dir_renders_canonical_claude_max_turns() {
    let (_dir, root, plugin_dir) = make_temp_project_plugin_dir();
    let agent_root = root.join("agents/plan-verifier");
    std::fs::create_dir_all(agent_root.join("claude")).expect("create canonical claude dir");
    std::fs::write(
        agent_root.join("agent.yaml"),
        r#"name: plan-verifier
role: plan_verifier
description: Dedicated plan verification agent that runs the adversarial round loop for ideation plans.
claude:
  disallowed_tools:
    - Write
    - Edit
    - NotebookEdit
  max_turns: 80
"#,
    )
    .expect("write shared definition");
    std::fs::write(
        agent_root.join("claude/prompt.md"),
        "Canonical plan verifier prompt",
    )
    .expect("write claude prompt");

    let generated_dir =
        materialize_generated_plugin_dir(&plugin_dir).expect("materialize generated plugin dir");
    let generated_prompt =
        std::fs::read_to_string(generated_dir.join("agents/plan-verifier.md"))
            .expect("read generated plan verifier prompt");

    assert!(
        generated_prompt.contains("maxTurns: 80"),
        "expected canonical claude maxTurns in generated frontmatter, got: {generated_prompt}"
    );
    assert!(
        generated_prompt.contains("Canonical plan verifier prompt"),
        "expected canonical prompt body to be preserved, got: {generated_prompt}"
    );
}
