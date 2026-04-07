use std::path::PathBuf;

use ralphx_lib::commands::diagnostic_commands::build_codex_cli_diagnostics_response;
use ralphx_lib::domain::agents::LogicalEffort;
use ralphx_lib::infrastructure::agents::{
    build_codex_exec_args, build_codex_exec_resume_args, build_spawnable_codex_exec_command,
    build_spawnable_codex_resume_command, parse_codex_cli_capabilities, parse_codex_version,
    CodexCliCapabilities, CodexExecCliConfig,
};

const ROOT_HELP: &str = r#"
Codex CLI

Commands:
  exec        Run Codex non-interactively [aliases: e]
  mcp         Manage external MCP servers for Codex
  resume      Resume a previous interactive session

Options:
  -c, --config <key=value>
  -m, --model <MODEL>
  -s, --sandbox <SANDBOX_MODE>
      --search
      --add-dir <DIR>
"#;

const EXEC_HELP: &str = r#"
Run Codex non-interactively

Usage: codex exec [OPTIONS] [PROMPT] [COMMAND]

Options:
  -c, --config <key=value>
  -m, --model <MODEL>
  -s, --sandbox <SANDBOX_MODE>
      --add-dir <DIR>
      --json
  -C, --cd <DIR>
      --skip-git-repo-check
"#;

#[test]
fn parse_codex_version_extracts_semver() {
    assert_eq!(
        parse_codex_version("codex-cli 0.116.0"),
        Some("0.116.0".to_string())
    );
    assert_eq!(parse_codex_version("codex 0.116.0"), None);
}

#[test]
fn parse_codex_cli_capabilities_detects_core_exec_surface() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, EXEC_HELP, Some("codex-cli 0.116.0"));

    assert_eq!(capabilities.version.as_deref(), Some("0.116.0"));
    assert!(capabilities.supports_exec_subcommand);
    assert!(capabilities.supports_json_output);
    assert!(capabilities.supports_model_flag);
    assert!(capabilities.supports_config_override);
    assert!(capabilities.supports_sandbox_flag);
    assert!(capabilities.supports_add_dir);
    assert!(capabilities.supports_search_flag);
    assert!(capabilities.supports_resume_subcommand);
    assert!(capabilities.supports_mcp_subcommand);
    assert!(capabilities.has_core_exec_support());
}

#[test]
fn parse_codex_cli_capabilities_reports_missing_json_feature() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, &EXEC_HELP.replace("--json", ""), None);

    assert!(!capabilities.supports_json_output);
    assert_eq!(capabilities.missing_core_exec_features(), vec!["json_output"]);
}

#[test]
fn build_codex_exec_args_maps_lane_settings_to_flags_and_overrides() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, EXEC_HELP, Some("codex-cli 0.116.0"));
    let config = CodexExecCliConfig {
        model: Some("gpt-5.4".to_string()),
        reasoning_effort: Some(LogicalEffort::XHigh),
        approval_policy: Some("on_request".to_string()),
        sandbox_mode: Some("workspace_write".to_string()),
        cwd: Some(PathBuf::from("/tmp/work")),
        add_dirs: vec![PathBuf::from("/tmp/extra")],
        skip_git_repo_check: true,
        json_output: true,
        search: true,
    };

    let args = build_codex_exec_args(&capabilities, &config).expect("args should build");

    assert_eq!(
        args,
        vec![
            "exec",
            "--json",
            "-m",
            "gpt-5.4",
            "-s",
            "workspace-write",
            "-C",
            "/tmp/work",
            "--add-dir",
            "/tmp/extra",
            "--skip-git-repo-check",
            "--search",
            "-c",
            "model_reasoning_effort=\"xhigh\"",
            "-c",
            "approval_policy=\"on-request\"",
        ]
    );
}

#[test]
fn build_codex_exec_args_rejects_missing_config_override_support() {
    let capabilities = parse_codex_cli_capabilities(
        &ROOT_HELP.replace("--config", ""),
        &EXEC_HELP.replace("--config", ""),
        Some("codex-cli 0.116.0"),
    );
    let config = CodexExecCliConfig {
        reasoning_effort: Some(LogicalEffort::Medium),
        ..Default::default()
    };

    let error = build_codex_exec_args(&capabilities, &config)
        .expect_err("missing config override support must fail");

    assert!(error.contains("config_override"));
}

#[test]
fn build_codex_exec_resume_args_maps_config_to_resume_surface() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, EXEC_HELP, Some("codex-cli 0.116.0"));
    let config = CodexExecCliConfig {
        model: Some("gpt-5.4".to_string()),
        reasoning_effort: Some(LogicalEffort::XHigh),
        approval_policy: Some("on_request".to_string()),
        sandbox_mode: Some("workspace_write".to_string()),
        cwd: Some(PathBuf::from("/tmp/work")),
        skip_git_repo_check: true,
        json_output: true,
        ..Default::default()
    };

    let args = build_codex_exec_resume_args(&capabilities, "session-123", &config)
        .expect("resume args should build");

    assert_eq!(
        args,
        vec![
            "exec",
            "resume",
            "session-123",
            "--json",
            "-m",
            "gpt-5.4",
            "--skip-git-repo-check",
            "-c",
            "model_reasoning_effort=\"xhigh\"",
            "-c",
            "approval_policy=\"on-request\"",
            "-c",
            "sandbox_mode=\"workspace-write\"",
        ]
    );
}

#[test]
fn build_spawnable_codex_exec_command_uses_prompt_arg_transport() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, EXEC_HELP, Some("codex-cli 0.116.0"));
    let config = CodexExecCliConfig {
        model: Some("gpt-5.4".to_string()),
        cwd: Some(PathBuf::from("/tmp/work")),
        ..Default::default()
    };

    let spawnable = build_spawnable_codex_exec_command(
        std::path::Path::new("/opt/homebrew/bin/codex"),
        "Plan the refactor",
        &capabilities,
        &config,
    )
    .expect("spawnable codex command should build");

    assert_eq!(
        spawnable.get_args_for_test(),
        vec!["exec", "--json", "-m", "gpt-5.4", "-C", "/tmp/work", "--", "Plan the refactor"]
    );
}

#[test]
fn build_spawnable_codex_resume_command_uses_resume_subcommand_and_prompt_arg() {
    let capabilities =
        parse_codex_cli_capabilities(ROOT_HELP, EXEC_HELP, Some("codex-cli 0.116.0"));
    let config = CodexExecCliConfig {
        json_output: true,
        ..Default::default()
    };

    let spawnable = build_spawnable_codex_resume_command(
        std::path::Path::new("/opt/homebrew/bin/codex"),
        "session-123",
        "Continue the plan",
        &capabilities,
        &config,
    )
    .expect("spawnable codex resume command should build");

    assert_eq!(
        spawnable.get_args_for_test(),
        vec!["exec", "resume", "session-123", "--json", "--", "Continue the plan"]
    );
}

#[test]
fn build_codex_cli_diagnostics_response_handles_missing_binary() {
    let diagnostics = build_codex_cli_diagnostics_response(None, None);

    assert!(!diagnostics.binary_found);
    assert!(!diagnostics.probe_succeeded);
    assert_eq!(diagnostics.error.as_deref(), Some("Codex CLI not found"));
}

#[test]
fn build_codex_cli_diagnostics_response_handles_successful_probe() {
    let capabilities = CodexCliCapabilities {
        version: Some("0.116.0".to_string()),
        supports_exec_subcommand: true,
        supports_json_output: true,
        supports_model_flag: true,
        supports_config_override: true,
        supports_sandbox_flag: true,
        supports_add_dir: true,
        supports_search_flag: true,
        supports_resume_subcommand: true,
        supports_mcp_subcommand: true,
    };

    let diagnostics = build_codex_cli_diagnostics_response(
        Some(std::path::Path::new("/opt/homebrew/bin/codex")),
        Some(Ok(capabilities)),
    );

    assert!(diagnostics.binary_found);
    assert!(diagnostics.probe_succeeded);
    assert!(diagnostics.has_core_exec_support);
    assert_eq!(diagnostics.version.as_deref(), Some("0.116.0"));
    assert_eq!(
        diagnostics.binary_path.as_deref(),
        Some("/opt/homebrew/bin/codex")
    );
}

#[test]
fn build_codex_cli_diagnostics_response_handles_probe_errors() {
    let diagnostics = build_codex_cli_diagnostics_response(
        Some(std::path::Path::new("/opt/homebrew/bin/codex")),
        Some(Err("help probe failed".to_string())),
    );

    assert!(diagnostics.binary_found);
    assert!(!diagnostics.probe_succeeded);
    assert_eq!(diagnostics.error.as_deref(), Some("help probe failed"));
}
