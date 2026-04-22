use super::{build_codex_mcp_overrides, compose_codex_prompt, CodexMcpRuntimeContext};
use std::path::PathBuf;

fn create_plugin_dir(root: &std::path::Path) -> PathBuf {
    let plugin_dir = root.join("plugins/app");
    std::fs::create_dir_all(plugin_dir.join("agents")).expect("create plugin agents dir");
    plugin_dir
}

#[test]
fn compose_codex_prompt_prefers_canonical_codex_prompt_when_available() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);

    std::fs::create_dir_all(root.join("agents/ralphx-utility-session-namer/codex"))
        .expect("create canonical codex dir");
    std::fs::write(
        root.join("agents/ralphx-utility-session-namer/agent.yaml"),
        "name: ralphx-utility-session-namer\nrole: session_namer\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/ralphx-utility-session-namer/codex/prompt.md"),
        "Canonical Codex Prompt",
    )
    .expect("write canonical codex prompt");
    std::fs::write(
        plugin_dir.join("agents/ralphx-utility-session-namer.md"),
        "---\nname: ralphx-utility-session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("ralphx-utility-session-namer"));

    assert!(
        composed.contains("Canonical Codex Prompt"),
        "expected canonical codex prompt to be injected"
    );
    assert!(
        !composed.contains("Legacy Claude Prompt"),
        "expected legacy claude prompt to be ignored when canonical codex prompt exists"
    );
}

#[test]
fn compose_codex_prompt_falls_back_to_legacy_claude_prompt_when_canonical_prompt_missing() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);

    std::fs::write(
        plugin_dir.join("agents/ralphx-utility-session-namer.md"),
        "---\nname: ralphx-utility-session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("ralphx-utility-session-namer"));

    assert!(
        composed.contains("Legacy Claude Prompt"),
        "expected legacy claude prompt fallback when canonical codex prompt is absent"
    );
}

#[test]
fn compose_codex_prompt_uses_shared_prompt_when_harness_is_explicitly_allowed() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);

    std::fs::create_dir_all(root.join("agents/ralphx-utility-session-namer/shared"))
        .expect("create shared prompt dir");
    std::fs::write(
        root.join("agents/ralphx-utility-session-namer/agent.yaml"),
        "name: ralphx-utility-session-namer\nrole: session_namer\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/ralphx-utility-session-namer/shared/prompt.md"),
        "Shared Session Namer Prompt",
    )
    .expect("write shared prompt");
    std::fs::write(
        plugin_dir.join("agents/ralphx-utility-session-namer.md"),
        "---\nname: ralphx-utility-session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("ralphx-utility-session-namer"));

    assert!(
        composed.contains("Shared Session Namer Prompt"),
        "expected shared prompt to be injected for supported codex harnesses"
    );
    assert!(
        !composed.contains("Legacy Claude Prompt"),
        "expected shared canonical prompt to win over legacy prompt fallback"
    );
}

#[test]
fn compose_codex_prompt_does_not_fall_back_to_legacy_prompt_when_canonical_agent_lacks_codex_prompt() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);

    std::fs::create_dir_all(root.join("agents/ralphx-ideation-team-lead/claude"))
        .expect("create canonical claude dir");
    std::fs::write(
        root.join("agents/ralphx-ideation-team-lead/agent.yaml"),
        "name: ralphx-ideation-team-lead\nrole: ideation_team_lead\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/ralphx-ideation-team-lead/claude/prompt.md"),
        "Canonical Claude Prompt",
    )
    .expect("write canonical claude prompt");
    std::fs::write(
        plugin_dir.join("agents/ralphx-ideation-team-lead.md"),
        "---\nname: ralphx-ideation-team-lead\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("ralphx-ideation-team-lead"));

    assert_eq!(
        composed, "User prompt",
        "canonical agents without a codex prompt should not silently inherit the legacy claude prompt"
    );
}

#[test]
fn build_codex_mcp_overrides_includes_runtime_feature_flags_from_agent_metadata() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);
    std::fs::create_dir_all(root.join("agents/ralphx-plan-verifier/codex"))
        .expect("create canonical codex dir");
    std::fs::write(
        root.join("agents/ralphx-plan-verifier/agent.yaml"),
        "name: ralphx-plan-verifier\nrole: plan_verifier\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/ralphx-plan-verifier/codex/agent.yaml"),
        "runtime_features:\n  shell_tool: false\n",
    )
    .expect("write codex metadata");
    std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build"))
        .expect("create fake mcp build dir");
    std::fs::write(
        plugin_dir.join("ralphx-mcp-server/build/index.js"),
        "// fake mcp server",
    )
    .expect("write fake mcp server");

    let overrides =
        build_codex_mcp_overrides(&plugin_dir, "ralphx-plan-verifier", false, None)
            .expect("overrides");

    assert!(
        overrides.iter().any(|entry| entry == "features.shell_tool=false"),
        "Codex runtime feature flags should flow into config overrides: {overrides:?}"
    );
}

#[test]
fn build_codex_mcp_overrides_injects_backend_owned_trace_dir() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);
    std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build"))
        .expect("create fake mcp build dir");
    std::fs::write(
        plugin_dir.join("ralphx-mcp-server/build/index.js"),
        "// fake mcp server",
    )
    .expect("write fake mcp server");

    let overrides =
        build_codex_mcp_overrides(&plugin_dir, "ralphx-plan-verifier", false, None)
            .expect("overrides");
    let trace_dir = crate::utils::runtime_log_paths::mcp_proxy_trace_dir()
        .to_string_lossy()
        .into_owned();
    let encoded_trace_dir = serde_json::to_string(&trace_dir).expect("encode trace dir");
    let expected_trace_override =
        format!("mcp_servers.ralphx.env.RALPHX_MCP_TRACE_DIR={encoded_trace_dir}");

    assert!(
        overrides.iter().any(|entry| entry == &expected_trace_override),
        "Codex MCP overrides must pass the app-owned trace dir: {overrides:?}"
    );
    assert!(
        !std::path::Path::new(&trace_dir).starts_with(root),
        "trace dir must not be under the target/plugin project root: {trace_dir}"
    );
}

#[test]
fn build_codex_mcp_overrides_passes_runtime_context_over_cli_args() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path();
    let plugin_dir = create_plugin_dir(root);
    std::fs::create_dir_all(plugin_dir.join("ralphx-mcp-server/build"))
        .expect("create fake mcp build dir");
    std::fs::write(
        plugin_dir.join("ralphx-mcp-server/build/index.js"),
        "// fake mcp server",
    )
    .expect("write fake mcp server");

    let runtime_context = CodexMcpRuntimeContext {
        context_type: Some("ideation".to_string()),
        context_id: Some("session-123".to_string()),
        task_id: None,
        project_id: Some("project-456".to_string()),
        working_directory: Some(root.join("workspace")),
        lead_session_id: Some("lead-789".to_string()),
    };

    let overrides = build_codex_mcp_overrides(
        &plugin_dir,
        "ralphx-plan-verifier",
        false,
        Some(&runtime_context),
    )
    .expect("overrides");

    let args_override = overrides
        .iter()
        .find(|entry| entry.starts_with("mcp_servers.") && entry.contains(".args="))
        .expect("args override");

    assert!(
        args_override.contains("--context-type"),
        "expected context-type CLI arg in overrides: {args_override}"
    );
    assert!(
        args_override.contains("ideation"),
        "expected context-type value in overrides: {args_override}"
    );
    assert!(
        args_override.contains("--context-id"),
        "expected context-id CLI arg in overrides: {args_override}"
    );
    assert!(
        args_override.contains("session-123"),
        "expected context-id value in overrides: {args_override}"
    );
    assert!(
        args_override.contains("--project-id"),
        "expected project-id CLI arg in overrides: {args_override}"
    );
    assert!(
        args_override.contains("project-456"),
        "expected project-id value in overrides: {args_override}"
    );
    assert!(
        args_override.contains("--working-directory"),
        "expected working-directory CLI arg in overrides: {args_override}"
    );
    assert!(
        args_override.contains("--lead-session-id"),
        "expected lead-session-id CLI arg in overrides: {args_override}"
    );
}
