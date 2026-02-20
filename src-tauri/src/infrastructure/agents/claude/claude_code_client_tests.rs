use super::*;
use crate::domain::agents::AgentRole;

#[test]
fn test_claude_code_client_new() {
    let client = ClaudeCodeClient::new();
    // CLI might or might not exist, but client should be created
    assert_eq!(client.capabilities.client_type, ClientType::ClaudeCode);
}

#[test]
fn test_claude_code_client_with_cli_path() {
    let client = ClaudeCodeClient::new().with_cli_path("/custom/path/claude");
    assert_eq!(client.cli_path, PathBuf::from("/custom/path/claude"));
}

#[test]
fn test_capabilities_claude_code() {
    let client = ClaudeCodeClient::new();
    let caps = client.capabilities();
    assert_eq!(caps.client_type, ClientType::ClaudeCode);
    assert!(caps.supports_shell);
    assert!(caps.supports_filesystem);
    assert!(caps.supports_streaming);
    assert!(caps.supports_mcp);
    assert_eq!(caps.max_context_tokens, 200_000);
}

#[test]
fn test_capabilities_has_models() {
    let client = ClaudeCodeClient::new();
    let caps = client.capabilities();
    assert!(caps.has_model("claude-sonnet-4-5-20250929"));
    assert!(caps.has_model("claude-opus-4-5-20251101"));
    assert!(caps.has_model("claude-haiku-4-5-20251001"));
}

#[test]
fn test_cli_path_getter() {
    let client = ClaudeCodeClient::new().with_cli_path("/test/claude");
    assert_eq!(client.cli_path(), &PathBuf::from("/test/claude"));
}

#[test]
fn test_default_trait() {
    let client = ClaudeCodeClient::default();
    assert_eq!(client.capabilities().client_type, ClientType::ClaudeCode);
}

#[tokio::test]
async fn test_is_available_with_nonexistent_path() {
    let client =
        ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
    let available = client.is_available().await.unwrap();
    assert!(!available);
}

#[tokio::test]
async fn test_spawn_agent_blocked_in_tests() {
    let client =
        ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
    let config = AgentConfig::worker("test");

    let result = client.spawn_agent(config).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
}

#[tokio::test]
async fn test_stop_agent_nonexistent_handle() {
    let client = ClaudeCodeClient::new();
    let handle = AgentHandle::with_id("nonexistent", ClientType::ClaudeCode, AgentRole::Worker);

    // Should not error - just means already stopped
    let result = client.stop_agent(&handle).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_wait_for_completion_nonexistent_handle() {
    let client = ClaudeCodeClient::new();
    let handle = AgentHandle::with_id("nonexistent", ClientType::ClaudeCode, AgentRole::Worker);

    let result = client.wait_for_completion(&handle).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AgentError::NotFound(_))));
}

// ==================== Streaming Spawn Tests ====================

#[test]
fn test_build_cli_args_basic() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test prompt");

    let args = client.build_cli_args(&config, None);

    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&"Test prompt".to_string()));
    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"stream-json".to_string()));
    assert!(args.contains(&"--permission-prompt-tool".to_string()));
    assert!(args.contains(&"mcp__ralphx__permission_request".to_string()));
}

#[test]
fn test_build_cli_args_with_agent() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test").with_agent("worker");

    let args = client.build_cli_args(&config, None);

    assert!(args.contains(&"--agent".to_string()));
    assert!(args.contains(&"worker".to_string()));
}

#[test]
fn test_build_cli_args_with_resume() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test").with_agent("worker");

    let args = client.build_cli_args(&config, Some("session-123"));

    // When resuming, both --resume AND --agent should be present
    // to ensure tool restrictions (disallowedTools) are enforced
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"session-123".to_string()));
    // Agent MUST be present when resuming to enforce disallowedTools
    assert!(args.contains(&"--agent".to_string()));
    assert!(args.contains(&"worker".to_string()));
}

#[test]
fn test_build_cli_args_applies_tools_restriction() {
    let client = ClaudeCodeClient::new();
    // Use fully-qualified name as would be used in production
    let config = AgentConfig::worker("Test")
        .with_agent(crate::infrastructure::agents::claude::agent_names::AGENT_SESSION_NAMER);

    let args = client.build_cli_args(&config, None);

    // session-namer has allowed_tools = Some("") meaning no CLI tools
    // get_allowed_tools strips the ralphx: prefix for AGENT_CONFIGS lookup
    let tools_idx = args
        .iter()
        .position(|a| a == "--tools")
        .expect("--tools flag must be present");
    assert_eq!(
        args[tools_idx + 1],
        "",
        "session-namer should have empty tools (MCP only)"
    );
}

#[test]
fn test_build_cli_args_no_tools_for_unknown_agent() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test").with_agent("unknown-agent-xyz");

    let args = client.build_cli_args(&config, None);

    // Unknown agent should NOT have --tools restriction
    assert!(
        !args.contains(&"--tools".to_string()),
        "unknown agent should not have --tools flag"
    );
}

#[test]
fn test_build_cli_args_restricted_agent_tools() {
    let client = ClaudeCodeClient::new();
    // Use fully-qualified name as would be used in production
    let config = AgentConfig::worker("Test").with_agent(
        crate::infrastructure::agents::claude::agent_names::AGENT_ORCHESTRATOR_IDEATION,
    );

    let args = client.build_cli_args(&config, None);

    let tools_idx = args
        .iter()
        .position(|a| a == "--tools")
        .expect("--tools flag must be present");
    assert_eq!(
        args[tools_idx + 1],
        "Read,Grep,Glob,Bash,WebFetch,WebSearch,Skill,TaskCreate,TaskUpdate,TaskGet,TaskList,TaskOutput,KillShell,MCPSearch,Task",
        "orchestrator-ideation should have base tools + Task"
    );
}

#[test]
fn test_build_cli_args_with_model() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test").with_model("opus");

    let args = client.build_cli_args(&config, None);

    assert!(args.contains(&"--model".to_string()));
    assert!(args.contains(&"opus".to_string()));
}

#[test]
fn test_build_cli_args_uses_agent_model_when_not_overridden() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test")
        .with_agent(crate::infrastructure::agents::claude::agent_names::AGENT_MERGER);

    let args = client.build_cli_args(&config, None);
    let model_idx = args
        .iter()
        .position(|a| a == "--model")
        .expect("--model flag must be present");
    assert_eq!(args[model_idx + 1], "opus");
}

#[test]
fn test_build_cli_args_with_plugin_dir() {
    let client = ClaudeCodeClient::new();
    let config = AgentConfig::worker("Test").with_plugin_dir("/custom/plugin");

    let args = client.build_cli_args(&config, None);

    assert!(args.contains(&"--plugin-dir".to_string()));
    assert!(args.contains(&"/custom/plugin".to_string()));
}

#[tokio::test]
async fn test_spawn_agent_streaming_blocked_in_tests() {
    let client =
        ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
    let config = AgentConfig::worker("test");

    let result = client.spawn_agent_streaming(config, None).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
}

#[test]
fn test_cli_available_with_nonexistent_path() {
    let client =
        ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
    assert!(!client.cli_available());
}

// ==================== StreamEvent Tests ====================

#[test]
fn test_stream_event_text_chunk_serialization() {
    let event = StreamEvent::TextChunk {
        text: "Hello world".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("TextChunk"));
    assert!(json.contains("Hello world"));

    // Deserialize back
    let parsed: StreamEvent = serde_json::from_str(&json).unwrap();
    if let StreamEvent::TextChunk { text } = parsed {
        assert_eq!(text, "Hello world");
    } else {
        panic!("Expected TextChunk");
    }
}

#[test]
fn test_stream_event_tool_call_start_serialization() {
    let event = StreamEvent::ToolCallStart {
        tool_name: "Read".to_string(),
        tool_id: Some("tool-123".to_string()),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("ToolCallStart"));
    assert!(json.contains("Read"));
    assert!(json.contains("tool-123"));
}

#[test]
fn test_stream_event_tool_call_complete_serialization() {
    let event = StreamEvent::ToolCallComplete {
        tool_name: "Write".to_string(),
        tool_id: None,
        arguments: serde_json::json!({"path": "test.txt"}),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("ToolCallComplete"));
    assert!(json.contains("Write"));
    assert!(json.contains("path"));
}

#[test]
fn test_stream_event_completed_serialization() {
    let event = StreamEvent::Completed {
        session_id: Some("sess-456".to_string()),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("Completed"));
    assert!(json.contains("sess-456"));
}

#[test]
fn test_stream_event_error_serialization() {
    let event = StreamEvent::Error {
        message: "Something went wrong".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("Error"));
    assert!(json.contains("Something went wrong"));
}

#[test]
fn test_streaming_spawn_result_debug() {
    // StreamingSpawnResult is Debug
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<StreamingSpawnResult>();
}

// ==================== Teammate Interactive Spawn Tests ====================

fn test_teammate_config() -> TeammateSpawnConfig {
    TeammateSpawnConfig::new(
        "transport-researcher",
        "ideation-abc123",
        "lead-session-uuid",
        "You are a transport research specialist. Investigate WebSocket vs SSE.",
    )
    .with_model("sonnet")
    .with_tools(vec![
        "Read".to_string(),
        "Grep".to_string(),
        "Glob".to_string(),
    ])
    .with_mcp_tools(vec![
        "get_session_plan".to_string(),
        "list_session_proposals".to_string(),
    ])
    .with_color("blue")
    .with_working_dir("/tmp/test")
    .with_plugin_dir("/test/ralphx-plugin")
}

#[test]
fn test_teammate_spawn_config_new_defaults() {
    let config = TeammateSpawnConfig::new("researcher", "team-1", "session-1", "Do research");

    assert_eq!(config.name, "researcher");
    assert_eq!(config.team_name, "team-1");
    assert_eq!(config.parent_session_id, "session-1");
    assert_eq!(config.prompt, "Do research");
    assert_eq!(config.model, "sonnet");
    assert_eq!(config.color, "blue");
    assert_eq!(config.agent_type, "general-purpose");
    assert_eq!(config.mcp_agent_type, "ideation-team-member");
    assert!(config.tools.is_empty());
    assert!(config.mcp_tools.is_empty());
    assert!(config.env.is_empty());
}

#[test]
fn test_teammate_spawn_config_builder_chain() {
    let config = TeammateSpawnConfig::new("dev", "team-x", "sess-1", "Code stuff")
        .with_model("haiku")
        .with_tools(vec!["Read".to_string()])
        .with_mcp_tools(vec!["get_task_context".to_string()])
        .with_color("green")
        .with_working_dir("/work")
        .with_plugin_dir("/plugins")
        .with_agent_type("Bash")
        .with_mcp_agent_type("worker-team-member")
        .with_env("CUSTOM_VAR", "value");

    assert_eq!(config.model, "haiku");
    assert_eq!(config.tools, vec!["Read"]);
    assert_eq!(config.mcp_tools, vec!["get_task_context"]);
    assert_eq!(config.color, "green");
    assert_eq!(config.working_directory, PathBuf::from("/work"));
    assert_eq!(config.plugin_dir, Some(PathBuf::from("/plugins")));
    assert_eq!(config.agent_type, "Bash");
    assert_eq!(config.mcp_agent_type, "worker-team-member");
    assert_eq!(config.env.get("CUSTOM_VAR"), Some(&"value".to_string()));
}

#[test]
fn test_build_teammate_cli_args_no_print_flag() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    // CRITICAL: No -p flag — interactive mode
    assert!(
        !args.contains(&"-p".to_string()),
        "Teammate args must NOT contain -p flag (interactive mode)"
    );
}

#[test]
fn test_build_teammate_cli_args_has_output_format() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    assert!(args.contains(&"--output-format".to_string()));
    assert!(args.contains(&"stream-json".to_string()));
    assert!(args.contains(&"--verbose".to_string()));
}

#[test]
fn test_build_teammate_cli_args_has_team_flags() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    // --agent-id <name>@<team-name>
    let agent_id_idx = args
        .iter()
        .position(|a| a == "--agent-id")
        .expect("--agent-id flag must be present");
    assert_eq!(
        args[agent_id_idx + 1],
        "transport-researcher@ideation-abc123"
    );

    // --agent-name
    let agent_name_idx = args
        .iter()
        .position(|a| a == "--agent-name")
        .expect("--agent-name flag must be present");
    assert_eq!(args[agent_name_idx + 1], "transport-researcher");

    // --team-name
    let team_name_idx = args
        .iter()
        .position(|a| a == "--team-name")
        .expect("--team-name flag must be present");
    assert_eq!(args[team_name_idx + 1], "ideation-abc123");

    // --agent-color
    let color_idx = args
        .iter()
        .position(|a| a == "--agent-color")
        .expect("--agent-color flag must be present");
    assert_eq!(args[color_idx + 1], "blue");

    // --parent-session-id
    let parent_idx = args
        .iter()
        .position(|a| a == "--parent-session-id")
        .expect("--parent-session-id flag must be present");
    assert_eq!(args[parent_idx + 1], "lead-session-uuid");

    // --agent-type (Claude Code built-in tool set)
    let agent_type_idx = args
        .iter()
        .position(|a| a == "--agent-type")
        .expect("--agent-type flag must be present");
    assert_eq!(args[agent_type_idx + 1], "general-purpose");
}

#[test]
fn test_build_teammate_cli_args_has_model() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    let model_idx = args
        .iter()
        .position(|a| a == "--model")
        .expect("--model flag must be present");
    assert_eq!(args[model_idx + 1], "sonnet");
}

#[test]
fn test_build_teammate_cli_args_has_tools() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    let tools_idx = args
        .iter()
        .position(|a| a == "--tools")
        .expect("--tools flag must be present");
    assert_eq!(args[tools_idx + 1], "Read,Grep,Glob");
}

#[test]
fn test_build_teammate_cli_args_no_tools_when_empty() {
    let client = ClaudeCodeClient::new();
    let config = TeammateSpawnConfig::new("r", "t", "s", "p");
    let args = client.build_teammate_cli_args(&config);

    assert!(
        !args.contains(&"--tools".to_string()),
        "Empty tools should not produce --tools flag"
    );
}

#[test]
fn test_build_teammate_cli_args_mcp_tools_prefixed() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    let allowed_idx = args
        .iter()
        .position(|a| a == "--allowedTools")
        .expect("--allowedTools flag must be present");
    let allowed_value = &args[allowed_idx + 1];

    // MCP tools should be prefixed with mcp__ralphx__
    assert!(
        allowed_value.contains("mcp__ralphx__get_session_plan"),
        "MCP tools must be prefixed: got {allowed_value}"
    );
    assert!(
        allowed_value.contains("mcp__ralphx__list_session_proposals"),
        "MCP tools must be prefixed: got {allowed_value}"
    );
}

#[test]
fn test_build_teammate_cli_args_no_allowed_tools_when_empty() {
    let client = ClaudeCodeClient::new();
    let config = TeammateSpawnConfig::new("r", "t", "s", "p");
    let args = client.build_teammate_cli_args(&config);

    assert!(
        !args.contains(&"--allowedTools".to_string()),
        "Empty MCP tools should not produce --allowedTools flag"
    );
}

#[test]
fn test_build_teammate_cli_args_has_system_prompt() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    let prompt_idx = args
        .iter()
        .position(|a| a == "--append-system-prompt")
        .expect("--append-system-prompt flag must be present");
    assert!(args[prompt_idx + 1].contains("transport research specialist"));
}

#[test]
fn test_build_teammate_cli_args_has_skip_permissions() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    assert!(
        args.contains(&"--dangerously-skip-permissions".to_string()),
        "Teammates must skip permissions"
    );
}

#[test]
fn test_build_teammate_cli_args_has_disable_slash_commands() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    assert!(args.contains(&"--disable-slash-commands".to_string()));
}

#[test]
fn test_build_teammate_cli_args_has_plugin_dir() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config();
    let args = client.build_teammate_cli_args(&config);

    assert!(args.contains(&"--plugin-dir".to_string()));
    assert!(args.contains(&"/test/ralphx-plugin".to_string()));
}

#[test]
fn test_build_teammate_cli_args_custom_agent_type() {
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config().with_agent_type("Bash");
    let args = client.build_teammate_cli_args(&config);

    let agent_type_idx = args
        .iter()
        .position(|a| a == "--agent-type")
        .expect("--agent-type flag must be present");
    assert_eq!(args[agent_type_idx + 1], "Bash");
}

#[test]
fn test_build_teammate_env_vars_has_team_flags() {
    let config = test_teammate_config();
    let env = ClaudeCodeClient::build_teammate_env_vars(&config);

    assert_eq!(env.get("CLAUDECODE"), Some(&"1".to_string()));
    assert_eq!(
        env.get("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"),
        Some(&"1".to_string())
    );
}

#[test]
fn test_build_teammate_env_vars_has_agent_type() {
    let config = test_teammate_config();
    let env = ClaudeCodeClient::build_teammate_env_vars(&config);

    assert_eq!(
        env.get("RALPHX_AGENT_TYPE"),
        Some(&"ideation-team-member".to_string())
    );
}

#[test]
fn test_build_teammate_env_vars_custom_mcp_agent_type() {
    let config = test_teammate_config().with_mcp_agent_type("worker-team-member");
    let env = ClaudeCodeClient::build_teammate_env_vars(&config);

    assert_eq!(
        env.get("RALPHX_AGENT_TYPE"),
        Some(&"worker-team-member".to_string())
    );
}

#[test]
fn test_build_teammate_env_vars_includes_custom_env() {
    let config = test_teammate_config()
        .with_env("RALPHX_PROJECT_ID", "proj-123")
        .with_env("RALPHX_SESSION_ID", "sess-456");
    let env = ClaudeCodeClient::build_teammate_env_vars(&config);

    assert_eq!(
        env.get("RALPHX_PROJECT_ID"),
        Some(&"proj-123".to_string())
    );
    assert_eq!(
        env.get("RALPHX_SESSION_ID"),
        Some(&"sess-456".to_string())
    );
    // Team flags still present
    assert_eq!(env.get("CLAUDECODE"), Some(&"1".to_string()));
}

#[tokio::test]
async fn test_spawn_teammate_interactive_blocked_in_tests() {
    let client =
        ClaudeCodeClient::new().with_cli_path("/nonexistent/path/to/claude_binary_12345");
    let config = test_teammate_config();

    let result = client.spawn_teammate_interactive(config).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AgentError::SpawnNotAllowed(_))));
}

#[test]
fn test_teammate_spawn_result_debug() {
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<TeammateSpawnResult>();
}

#[test]
fn test_teammate_spawn_config_debug_and_clone() {
    let config = test_teammate_config();
    let cloned = config.clone();
    assert_eq!(cloned.name, "transport-researcher");
    // Verify Debug is implemented (compile-time check)
    let _debug = format!("{:?}", cloned);
}

#[test]
fn test_build_teammate_cli_args_full_integration() {
    // Verify the complete arg list for a realistic teammate spawn
    let client = ClaudeCodeClient::new();
    let config = TeammateSpawnConfig::new(
        "react-state-sync-researcher",
        "ideation-session-789",
        "c43c3747-44d8-437b-9a25-911032eec2ea",
        "You are a React state management specialist. Analyze existing Zustand stores.",
    )
    .with_model("sonnet")
    .with_tools(vec![
        "Read".to_string(),
        "Grep".to_string(),
        "Glob".to_string(),
        "WebSearch".to_string(),
    ])
    .with_mcp_tools(vec![
        "get_session_plan".to_string(),
        "get_plan_artifact".to_string(),
    ])
    .with_color("green")
    .with_working_dir("/Users/test/project");

    let args = client.build_teammate_cli_args(&config);

    // Verify NO -p flag
    assert!(!args.contains(&"-p".to_string()));

    // Verify all required flags are present
    let required_flags = vec![
        "--output-format",
        "--verbose",
        "--disable-slash-commands",
        "--agent-id",
        "--agent-name",
        "--team-name",
        "--agent-color",
        "--parent-session-id",
        "--agent-type",
        "--model",
        "--tools",
        "--allowedTools",
        "--append-system-prompt",
        "--dangerously-skip-permissions",
    ];
    for flag in &required_flags {
        assert!(
            args.contains(&flag.to_string()),
            "Missing required flag: {flag}"
        );
    }

    // Verify agent-id format: name@team-name
    let agent_id_idx = args.iter().position(|a| a == "--agent-id").unwrap();
    assert_eq!(
        args[agent_id_idx + 1],
        "react-state-sync-researcher@ideation-session-789"
    );

    // Verify tools are comma-separated
    let tools_idx = args.iter().position(|a| a == "--tools").unwrap();
    assert_eq!(args[tools_idx + 1], "Read,Grep,Glob,WebSearch");

    // Verify MCP tools are prefixed and comma-separated
    let allowed_idx = args.iter().position(|a| a == "--allowedTools").unwrap();
    assert_eq!(
        args[allowed_idx + 1],
        "mcp__ralphx__get_session_plan,mcp__ralphx__get_plan_artifact"
    );
}

#[test]
fn test_build_teammate_cli_args_passes_settings_when_profile_exists() {
    // Verifies that --settings is passed when get_effective_settings returns a value.
    // The embedded ralphx.yaml configures a global `settings_profile: default`,
    // so any agent_type not registered in agents[] falls through to the global profile.
    // "unregistered-agent-type" triggers the global fallback path.
    let client = ClaudeCodeClient::new();
    let config = test_teammate_config().with_agent_type("unregistered-agent-type");

    // Confirm a settings value is available for this agent_type (global fallback)
    let has_settings = super::get_effective_settings(Some("unregistered-agent-type")).is_some();
    if !has_settings {
        // No settings profile configured in this environment — skip the positive assertion
        // and verify --settings is correctly absent
        let args = client.build_teammate_cli_args(&config);
        assert!(
            !args.contains(&"--settings".to_string()),
            "--settings must not appear when no profile is configured"
        );
        return;
    }

    let args = client.build_teammate_cli_args(&config);

    // --settings flag must be present
    let settings_idx = args
        .iter()
        .position(|a| a == "--settings")
        .expect("--settings flag must be present when a settings profile exists");

    // The value after --settings must be valid JSON
    let json_str = &args[settings_idx + 1];
    serde_json::from_str::<serde_json::Value>(json_str)
        .expect("--settings value must be valid JSON");
}
