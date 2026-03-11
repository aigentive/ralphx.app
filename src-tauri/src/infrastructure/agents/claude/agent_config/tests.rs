use super::*;
use crate::infrastructure::agents::claude::agent_names::{
    SHORT_CHAT_PROJECT, SHORT_CHAT_TASK, SHORT_CODER, SHORT_DEEP_RESEARCHER,
    SHORT_DEPENDENCY_SUGGESTER, SHORT_IDEATION_TEAM_LEAD, SHORT_MEMORY_CAPTURE,
    SHORT_MEMORY_MAINTAINER, SHORT_MERGER, SHORT_ORCHESTRATOR, SHORT_ORCHESTRATOR_IDEATION,
    SHORT_ORCHESTRATOR_IDEATION_READONLY, SHORT_PROJECT_ANALYZER, SHORT_QA_EXECUTOR, SHORT_QA_PREP,
    SHORT_REVIEWER, SHORT_REVIEW_CHAT, SHORT_REVIEW_HISTORY, SHORT_SESSION_NAMER, SHORT_SUPERVISOR,
    SHORT_WORKER, SHORT_WORKER_TEAM,
};
use std::collections::HashSet;

#[test]
fn test_yaml_loaded_has_unique_names() {
    let mut names: Vec<String> = agent_configs().iter().map(|c| c.name.clone()).collect();
    let original_len = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), original_len);
}

#[test]
fn test_get_allowed_tools_worker_agent() {
    let tools = get_allowed_tools("ralphx-worker").unwrap();
    assert!(tools.contains("Read"));
    assert!(tools.contains("Write"));
    assert!(tools.contains("Edit"));
    assert!(tools.contains("Task"));
}

#[test]
fn test_get_allowed_tools_mcp_only_agent() {
    assert_eq!(get_allowed_tools("session-namer"), Some(String::new()));
}

#[test]
fn test_get_preapproved_tools_worker_contains_expected() {
    let tools = get_preapproved_tools("ralphx-worker").unwrap();
    assert!(tools.contains("mcp__ralphx__get_task_context"));
    assert!(tools.contains("mcp__ralphx__get_project_analysis"));
    assert!(tools.contains("Write"));
    assert!(tools.contains("Task(Explore)"));
    // Workers should NOT have memory skills - only dedicated memory agents
    assert!(!tools.contains("Skill(ralphx:rule-manager)"));
}

#[test]
fn test_default_base_tool_set_present_in_worker() {
    let tools = get_allowed_tools("ralphx-worker").unwrap();
    for t in DEFAULT_BASE_CLI_TOOLS {
        assert!(tools.contains(t), "worker missing base tool {}", t);
    }
}

#[test]
fn test_all_agent_names_are_known() {
    let known: HashSet<&str> = HashSet::from([
        SHORT_ORCHESTRATOR_IDEATION,
        SHORT_ORCHESTRATOR_IDEATION_READONLY,
        SHORT_SESSION_NAMER,
        SHORT_DEPENDENCY_SUGGESTER,
        SHORT_CHAT_TASK,
        SHORT_CHAT_PROJECT,
        SHORT_REVIEW_CHAT,
        SHORT_REVIEW_HISTORY,
        SHORT_WORKER,
        SHORT_CODER,
        SHORT_REVIEWER,
        SHORT_QA_PREP,
        SHORT_QA_EXECUTOR,
        SHORT_ORCHESTRATOR,
        SHORT_SUPERVISOR,
        SHORT_DEEP_RESEARCHER,
        SHORT_PROJECT_ANALYZER,
        SHORT_MERGER,
        SHORT_MEMORY_MAINTAINER,
        SHORT_MEMORY_CAPTURE,
        // Team lead variants
        SHORT_IDEATION_TEAM_LEAD,
        SHORT_WORKER_TEAM,
    ]);

    for agent in agent_configs() {
        assert!(
            known.contains(agent.name.as_str()),
            "Unknown agent name in ralphx.yaml: {}",
            agent.name
        );
    }
}

#[test]
fn test_all_system_prompt_files_exist() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    for agent in agent_configs() {
        let prompt_path = project_root.join(&agent.system_prompt_file);
        assert!(
            prompt_path.exists(),
            "Missing system_prompt_file for {}: {}",
            agent.name,
            prompt_path.display()
        );
    }
}

#[test]
fn test_permission_prompt_tool_accepts_shorthand() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
tools:
  extends: base_tools
  include: [Write]
mcp_tools: [get_task_context]
preapproved_cli_tools: []
system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.permission_prompt_tool,
        "mcp__ralphx__permission_request"
    );
}

#[test]
fn test_settings_profile_selection_uses_default_profile_payload() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: default
  settings_profiles:
    default:
      sandbox:
        enabled: false
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config_no_env_overrides(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.settings,
        Some(serde_json::json!({
            "sandbox": { "enabled": false }
        }))
    );
}

#[test]
fn test_openrouter_settings_profile_supports_blank_api_key_and_timeout() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: openrouter
  settings_profiles:
    default:
      sandbox:
        enabled: false
    openrouter:
      extends: default
      env:
        ANTHROPIC_AUTH_TOKEN: your_openrouter_api_key
        ANTHROPIC_BASE_URL: https://openrouter.ai/api
        ANTHROPIC_API_KEY: ""
        API_TIMEOUT_MS: "3000000"
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config_no_env_overrides(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.settings,
        Some(serde_json::json!({
            "sandbox": { "enabled": false },
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "your_openrouter_api_key",
                "ANTHROPIC_BASE_URL": "https://openrouter.ai/api",
                "ANTHROPIC_API_KEY": "",
                "API_TIMEOUT_MS": "3000000"
            }
        }))
    );
}

#[test]
fn test_settings_profile_resolves_prefixed_env_overrides() {
    let mut settings = serde_json::json!({
        "env": {
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "glm-4.5-air",
            "ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "glm-5",
        }
    });

    apply_prefixed_env_overrides_with(&mut settings, &|name| match name {
        "RALPHX_ANTHROPIC_DEFAULT_HAIKU_MODEL" => Some("custom-haiku".to_string()),
        "RALPHX_ANTHROPIC_DEFAULT_SONNET_MODEL" => Some("custom-sonnet".to_string()),
        _ => None,
    });

    assert_eq!(
        settings
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_DEFAULT_HAIKU_MODEL"))
            .and_then(|v| v.as_str()),
        Some("custom-haiku")
    );
    assert_eq!(
        settings
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_DEFAULT_SONNET_MODEL"))
            .and_then(|v| v.as_str()),
        Some("custom-sonnet")
    );
    assert_eq!(
        settings
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_DEFAULT_OPUS_MODEL"))
            .and_then(|v| v.as_str()),
        Some("glm-5")
    );
}

#[test]
fn test_agent_settings_profile_overrides_global_profile() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
    default:
      sandbox:
        enabled: false
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    settings_profile: default
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
  - name: ralphx-coder
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/coder.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");

    assert!(
        parsed.claude.settings.is_some(),
        "global z_ai should be active"
    );

    let worker = parsed
        .agents
        .iter()
        .find(|a| a.name == "ralphx-worker")
        .expect("worker should exist");
    assert_eq!(
        worker.settings,
        Some(serde_json::json!({
            "sandbox": { "enabled": false }
        })),
        "worker should override to default profile"
    );

    let coder = parsed
        .agents
        .iter()
        .find(|a| a.name == "ralphx-coder")
        .expect("coder should exist");
    assert!(
        coder.settings.is_some(),
        "coder should inherit global z_ai profile"
    );
}

#[test]
fn test_unknown_agent_settings_profile_falls_back_to_global() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
z_ai:
  env:
    ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
settings_profile: missing_profile
tools:
  extends: base_tools
  include: [Write]
mcp_tools: [get_task_context]
preapproved_cli_tools: []
system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let worker = parsed
        .agents
        .iter()
        .find(|a| a.name == "ralphx-worker")
        .expect("worker should exist");
    assert_eq!(
        worker.settings, parsed.claude.settings,
        "unknown agent profile should inherit global settings"
    );
}

#[test]
fn test_runtime_settings_profile_override_reads_env_value() {
    let selection = runtime_settings_profile_override_with(&|name| match name {
        "RALPHX_CLAUDE_SETTINGS_PROFILE" => Some("z_ai".to_string()),
        _ => None,
    });
    assert_eq!(selection.as_deref(), Some("z_ai"));
}

#[test]
fn test_runtime_settings_profile_override_ignores_blank_value() {
    let selection = runtime_settings_profile_override_with(&|name| match name {
        "RALPHX_CLAUDE_SETTINGS_PROFILE" => Some("   ".to_string()),
        _ => None,
    });
    assert_eq!(selection, None);
}

#[test]
fn test_runtime_settings_profile_override_for_agent_uses_normalized_key() {
    let selection = runtime_settings_profile_override_for_agent_with(
        "orchestrator-ideation",
        &|name| match name {
            "RALPHX_CLAUDE_SETTINGS_PROFILE_ORCHESTRATOR_IDEATION" => Some("default".to_string()),
            _ => None,
        },
    );
    assert_eq!(selection.as_deref(), Some("default"));
}

#[test]
fn test_normalize_agent_name_for_env_replaces_symbols() {
    assert_eq!(
        normalize_agent_name_for_env("ralphx:session-namer"),
        "RALPHX_SESSION_NAMER"
    );
}

#[test]
fn test_settings_profile_defaults_apply_to_selected_profile() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profile_defaults:
    permissions:
      deny:
        - Read(./.env)
  settings_profiles:
    z_ai:
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.settings,
        Some(serde_json::json!({
            "permissions": { "deny": ["Read(./.env)"] },
            "env": { "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic" }
        }))
    );
}

#[test]
fn test_settings_profile_extends_supports_base_profile() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
  settings_profile: z_ai
  settings_profiles:
    locked_down:
      permissions:
        deny:
          - Read(./.env)
          - Edit(./.env)
    z_ai:
      extends: locked_down
      env:
        ANTHROPIC_BASE_URL: https://api.z.ai/api/anthropic
agents:
  - name: ralphx-worker
    tools:
      extends: base_tools
      include: [Write]
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.settings,
        Some(serde_json::json!({
            "permissions": {
                "deny": ["Read(./.env)", "Edit(./.env)"]
            },
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.z.ai/api/anthropic"
            }
        }))
    );
}

#[test]
fn test_permission_prompt_tool_keeps_fully_qualified_name() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: mcp__external__permission_prompt
agents:
  - name: ralphx-worker
tools:
  extends: base_tools
  include: [Write]
mcp_tools: [get_task_context]
preapproved_cli_tools: []
system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(
        parsed.claude.permission_prompt_tool,
        "mcp__external__permission_prompt"
    );
}

#[test]
fn test_mcp_server_name_changes_shorthand_prefix() {
    let yaml = r#"
claude:
  mcp_server_name: acme
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
tools:
  extends: base_tools
  include: [Write]
mcp_tools: [get_task_context]
preapproved_cli_tools: []
system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(parsed.claude.mcp_server_name, "acme");
    assert_eq!(
        parsed.claude.permission_prompt_tool,
        "mcp__acme__permission_request"
    );
}

#[test]
fn test_memory_maintainer_has_memory_skills() {
    let tools = get_preapproved_tools("ralphx:memory-maintainer").unwrap();
    assert!(tools.contains("Skill(ralphx:rule-manager)"));
    assert!(tools.contains("Skill(ralphx:knowledge-capture)"));
}

#[test]
fn test_memory_capture_has_memory_skills() {
    let tools = get_preapproved_tools("ralphx:memory-capture").unwrap();
    assert!(tools.contains("Skill(ralphx:rule-manager)"));
    assert!(tools.contains("Skill(ralphx:knowledge-capture)"));
}

#[test]
fn test_non_memory_agents_lack_memory_skills() {
    let agents_to_test = vec![
        "ralphx-worker",
        "ralphx-reviewer",
        "ralphx-orchestrator",
        "ralphx-chat-task",
        "ralphx-chat-project",
    ];
    for agent_name in agents_to_test {
        if let Some(tools) = get_preapproved_tools(agent_name) {
            assert!(
                !tools.contains("Skill(ralphx:rule-manager)"),
                "Agent {} should not have rule-manager skill",
                agent_name
            );
            assert!(
                !tools.contains("Skill(ralphx:knowledge-capture)"),
                "Agent {} should not have knowledge-capture skill",
                agent_name
            );
        }
    }
}

#[test]
fn test_non_memory_agents_lack_memory_write_mcp_tools() {
    // Memory write tools per spec section 11.2
    let memory_write_tools = vec![
        "upsert_memories",
        "mark_memory_obsolete",
        "refresh_memory_rule_index",
        "ingest_rule_file",
        "rebuild_archive_snapshots",
    ];

    let agents_to_test = vec![
        "ralphx-worker",
        "ralphx-reviewer",
        "ralphx-orchestrator",
        "ralphx-chat-task",
        "ralphx-chat-project",
    ];

    for agent_name in agents_to_test {
        if let Some(config) = get_agent_config(agent_name) {
            for write_tool in &memory_write_tools {
                assert!(
                    !config.allowed_mcp_tools.contains(&write_tool.to_string()),
                    "Agent {} should not have write memory tool: {}",
                    agent_name,
                    write_tool
                );
            }
        }
    }
}

#[test]
fn test_memory_agents_have_write_mcp_tools() {
    // Memory maintainer should have write tools
    if let Some(config) = get_agent_config("memory-maintainer") {
        assert!(config
            .allowed_mcp_tools
            .contains(&"upsert_memories".to_string()));
        assert!(config
            .allowed_mcp_tools
            .contains(&"mark_memory_obsolete".to_string()));
        assert!(config
            .allowed_mcp_tools
            .contains(&"refresh_memory_rule_index".to_string()));
        assert!(config
            .allowed_mcp_tools
            .contains(&"ingest_rule_file".to_string()));
        assert!(config
            .allowed_mcp_tools
            .contains(&"rebuild_archive_snapshots".to_string()));
    }

    // Memory capture should have upsert_memories
    if let Some(config) = get_agent_config("memory-capture") {
        assert!(config
            .allowed_mcp_tools
            .contains(&"upsert_memories".to_string()));
    }
}

#[test]
#[ignore = "memory read tools not yet added to worker/reviewer/orchestrator configs"]
fn test_read_only_agents_have_read_memory_tools() {
    let read_memory_tools = vec!["search_memories", "get_memory", "get_memories_for_paths"];

    let agents_to_test = vec!["ralphx-worker", "ralphx-reviewer", "ralphx-orchestrator"];

    for agent_name in agents_to_test {
        if let Some(config) = get_agent_config(agent_name) {
            // Each of these should have at least one of the read memory tools
            let has_read_tool = read_memory_tools
                .iter()
                .any(|t| config.allowed_mcp_tools.contains(&t.to_string()));
            assert!(
                has_read_tool,
                "Agent {} should have at least one read memory tool",
                agent_name
            );
        }
    }
}

#[test]
fn test_memory_maintainer_has_cli_write_tools() {
    // Memory maintainer must have Write and Edit to update rule files and archives
    if let Some(config) = get_agent_config("memory-maintainer") {
        assert!(
            config.preapproved_cli_tools.contains(&"Write".to_string()),
            "memory-maintainer must have Write tool"
        );
        assert!(
            config.preapproved_cli_tools.contains(&"Edit".to_string()),
            "memory-maintainer must have Edit tool"
        );
        assert!(
            config.preapproved_cli_tools.contains(&"Bash".to_string()),
            "memory-maintainer must have Bash tool for file operations"
        );
    }

    // Verify it's not MCP-only
    if let Some(config) = get_agent_config("memory-maintainer") {
        assert!(!config.mcp_only, "memory-maintainer should have CLI tools");
    }
}

#[test]
fn test_memory_capture_has_read_cli_tools() {
    // Memory capture needs read tools to analyze conversations and extract memory
    if let Some(config) = get_agent_config("memory-capture") {
        assert!(
            config.preapproved_cli_tools.contains(&"Read".to_string()),
            "memory-capture must have Read tool"
        );
        assert!(
            config.preapproved_cli_tools.contains(&"Grep".to_string()),
            "memory-capture must have Grep tool"
        );
    }

    // Verify it's not MCP-only
    if let Some(config) = get_agent_config("memory-capture") {
        assert!(!config.mcp_only, "memory-capture should have CLI tools");
    }
}

// ── Verification tool allowlist tests ───────────────────────────

#[test]
fn test_readonly_agent_has_get_plan_verification_not_update() {
    let config = get_agent_config("orchestrator-ideation-readonly")
        .expect("orchestrator-ideation-readonly should exist");
    assert!(
        config
            .allowed_mcp_tools
            .contains(&"get_plan_verification".to_string()),
        "readonly agent must include get_plan_verification"
    );
    assert!(
        !config
            .allowed_mcp_tools
            .contains(&"update_plan_verification".to_string()),
        "readonly agent must NOT include update_plan_verification"
    );
}

// ── Agent extends inheritance tests ─────────────────────────────

#[test]
fn test_extends_inherits_parent_tools() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: base-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write, Edit, Task] }
    mcp_tools: [start_step, complete_step]
    preapproved_cli_tools: [Write, Edit, Bash]
  - name: worker-team
    extends: base-worker
    system_prompt_file: ralphx-plugin/agents/worker-team.md
    model: opus
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let team = parsed
        .agents
        .iter()
        .find(|a| a.name == "worker-team")
        .expect("worker-team should exist");

    // model overridden by child
    assert_eq!(team.model.as_deref(), Some("opus"));
    // system_prompt_file overridden by child
    assert_eq!(
        team.system_prompt_file,
        "ralphx-plugin/agents/worker-team.md"
    );
    // tools inherited from parent (child didn't specify)
    assert!(team.resolved_cli_tools.contains(&"Write".to_string()));
    assert!(team.resolved_cli_tools.contains(&"Edit".to_string()));
    assert!(team.resolved_cli_tools.contains(&"Task".to_string()));
    // mcp_tools inherited from parent
    assert!(team.allowed_mcp_tools.contains(&"start_step".to_string()));
    assert!(team
        .allowed_mcp_tools
        .contains(&"complete_step".to_string()));
    // preapproved_cli_tools inherited from parent
    assert!(team.preapproved_cli_tools.contains(&"Write".to_string()));
}

#[test]
fn test_extends_child_overrides_mcp_tools() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: base-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [start_step, complete_step]
    preapproved_cli_tools: [Write]
  - name: custom-worker
    extends: base-worker
    mcp_tools: [get_task_context]
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let custom = parsed
        .agents
        .iter()
        .find(|a| a.name == "custom-worker")
        .expect("custom-worker should exist");

    // mcp_tools overridden by child
    assert_eq!(custom.allowed_mcp_tools, vec!["get_task_context"]);
    // model inherited
    assert_eq!(custom.model.as_deref(), Some("sonnet"));
    // system_prompt_file inherited
    assert_eq!(custom.system_prompt_file, "ralphx-plugin/agents/worker.md");
}

#[test]
fn test_extends_circular_detection() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: agent-a
    extends: agent-b
    system_prompt_file: ralphx-plugin/agents/worker.md
  - name: agent-b
    extends: agent-a
    system_prompt_file: ralphx-plugin/agents/worker.md
"#;
    // Should parse without panic (circular breaks with warning)
    let parsed = parse_config(yaml).expect("config should parse despite circular extends");
    assert_eq!(parsed.agents.len(), 2);
}

#[test]
fn test_extends_unknown_parent_keeps_child_as_is() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: orphan-agent
    extends: nonexistent-parent
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: haiku
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let agent = parsed
        .agents
        .iter()
        .find(|a| a.name == "orphan-agent")
        .expect("orphan-agent should exist");
    assert_eq!(agent.model.as_deref(), Some("haiku"));
}

#[test]
fn test_extends_chained_inheritance() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: grandparent
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: haiku
    mcp_tools: [tool_a]
    preapproved_cli_tools: [Bash]
  - name: parent
    extends: grandparent
    model: sonnet
    mcp_tools: [tool_b]
  - name: child
    extends: parent
    model: opus
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let child = parsed
        .agents
        .iter()
        .find(|a| a.name == "child")
        .expect("child should exist");

    // model from child
    assert_eq!(child.model.as_deref(), Some("opus"));
    // mcp_tools from parent (overrides grandparent)
    assert_eq!(child.allowed_mcp_tools, vec!["tool_b"]);
    // system_prompt_file from grandparent (inherited through chain)
    assert_eq!(child.system_prompt_file, "ralphx-plugin/agents/worker.md");
    // preapproved_cli_tools from grandparent
    assert!(child.preapproved_cli_tools.contains(&"Bash".to_string()));
}

#[test]
fn test_no_extends_backward_compatible() {
    // Agents without extends should work exactly as before
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: standalone
    system_prompt_file: ralphx-plugin/agents/worker.md
    model: sonnet
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: [Write, Bash]
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let agent = parsed
        .agents
        .iter()
        .find(|a| a.name == "standalone")
        .expect("standalone should exist");
    assert_eq!(agent.model.as_deref(), Some("sonnet"));
    assert!(agent.resolved_cli_tools.contains(&"Write".to_string()));
}

// ── Process mapping + team constraints integration tests ────────

#[test]
fn test_process_mapping_parsed_from_full_config() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
process_mapping:
  execution:
    default: ralphx-worker
    team: ralphx-worker-team
  ideation:
    default: orchestrator-ideation
agents:
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert_eq!(parsed.process_mapping.slots.len(), 2);
    assert_eq!(
        parsed.process_mapping.slots["execution"].default,
        "ralphx-worker"
    );
    assert_eq!(
        parsed.process_mapping.slots["execution"]
            .variants
            .get("team")
            .unwrap(),
        "ralphx-worker-team"
    );
}

#[test]
fn test_team_constraints_parsed_from_full_config() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
team_constraints:
  _defaults:
    max_teammates: 5
    model_cap: sonnet
  execution:
    max_teammates: 3
    mode: dynamic
    timeout_minutes: 30
agents:
  - name: ralphx-worker
    system_prompt_file: ralphx-plugin/agents/worker.md
    tools: { extends: base_tools, include: [Write] }
    mcp_tools: [get_task_context]
    preapproved_cli_tools: []
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    let defaults = parsed.team_constraints.defaults.as_ref().unwrap();
    assert_eq!(defaults.max_teammates, 5);
    let exec = &parsed.team_constraints.processes["execution"];
    assert_eq!(exec.max_teammates, 3);
    assert_eq!(exec.timeout_minutes, 30);
}

#[test]
fn test_missing_process_mapping_uses_empty_default() {
    let yaml = r#"
claude:
  mcp_server_name: ralphx
  permission_mode: default
  dangerously_skip_permissions: false
  permission_prompt_tool: permission_request
agents:
  - name: ralphx-worker
system_prompt_file: ralphx-plugin/agents/worker.md
tools: { extends: base_tools, include: [Write] }
mcp_tools: [get_task_context]
preapproved_cli_tools: []
"#;
    let parsed = parse_config(yaml).expect("config should parse");
    assert!(parsed.process_mapping.slots.is_empty());
    assert!(parsed.team_constraints.processes.is_empty());
    assert!(parsed.team_constraints.defaults.is_none());
}
