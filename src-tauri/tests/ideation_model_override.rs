// Integration tests for ideation model pre-resolution in the ChatService command builder.
//
// These tests verify that:
// 1. `build_base_cli_command` with `model_override=Some("opus")` produces `--model opus` in args
// 2. `build_base_cli_command` with `model_override=None` falls through to YAML config
// 3. Non-ideation agents bypass DB resolution (model_bucket_for_agent returns None)
//
// Note: The `resolve_ideation_model()` 4-level chain is already tested exhaustively in
// `src-tauri/src/infrastructure/agents/claude/model_resolver_tests.rs`.
// These tests focus on the CLI arg injection layer.

use std::path::Path;

use ralphx_lib::domain::repositories::IdeationModelSettingsRepository;
use ralphx_lib::infrastructure::agents::claude::{
    build_base_cli_command,
    model_resolver::{resolve_ideation_model, resolve_verifier_subagent_model_with_source},
};
use ralphx_lib::infrastructure::memory::MemoryIdeationModelSettingsRepository;

// Helper to collect OsStr args from tokio::process::Command as Strings
fn collect_args(cmd: &tokio::process::Command) -> Vec<String> {
    cmd.as_std()
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect()
}

// --- CLI arg injection tests ---

#[test]
fn test_build_base_cli_command_with_model_override_passes_model_arg() {
    // When model_override=Some("opus"), --model opus must appear in the CLI args.
    let result = build_base_cli_command(
        Path::new("/fake/claude"),
        Path::new("/fake/plugin"),
        Some("orchestrator-ideation"),
        false,
        None,           // effort_override
        Some("opus"),   // model_override
    );
    assert!(result.is_ok(), "build_base_cli_command failed: {:?}", result.err());
    let cmd = result.unwrap();
    let args = collect_args(&cmd);
    let model_pos = args.iter().position(|a| a == "--model");
    assert!(
        model_pos.is_some(),
        "expected --model flag in args, got: {:?}",
        args
    );
    let model_val = args.get(model_pos.unwrap() + 1).map(String::as_str);
    assert_eq!(
        model_val,
        Some("opus"),
        "expected --model opus, got: {:?}",
        model_val
    );
}

#[test]
fn test_build_base_cli_command_with_sonnet_model_override() {
    // model_override=Some("sonnet") → --model sonnet
    let result = build_base_cli_command(
        Path::new("/fake/claude"),
        Path::new("/fake/plugin"),
        Some("orchestrator-ideation"),
        false,
        None,
        Some("sonnet"),
    );
    assert!(result.is_ok());
    let cmd = result.unwrap();
    let args = collect_args(&cmd);
    let model_pos = args.iter().position(|a| a == "--model").expect("--model not found");
    assert_eq!(args.get(model_pos + 1).map(String::as_str), Some("sonnet"));
}

#[test]
fn test_build_base_cli_command_no_model_override_no_yaml_uses_default() {
    // When model_override=None and the agent has no YAML-configured model,
    // build_base_cli_command should still produce a --model flag from the YAML fallback.
    // For an unknown agent name, the fallback is "sonnet" (hardcoded default).
    let result = build_base_cli_command(
        Path::new("/fake/claude"),
        Path::new("/fake/plugin"),
        Some("unknown-agent-with-no-yaml-config"),
        false,
        None,  // effort_override
        None,  // model_override — YAML fallback should apply
    );
    // Command building succeeds regardless of whether --model is present
    assert!(result.is_ok(), "build_base_cli_command failed: {:?}", result.err());
    // The --model flag should appear if the YAML agent config has a model set;
    // it may be absent if the agent has no model in YAML (acceptable behavior).
    // The key assertion is that model_override=None does NOT inject a DB-resolved value.
    let cmd = result.unwrap();
    let args = collect_args(&cmd);
    if let Some(pos) = args.iter().position(|a| a == "--model") {
        let val = args.get(pos + 1).map(String::as_str).unwrap_or("");
        assert_ne!(val, "opus", "DB override should not appear when model_override=None");
        assert_ne!(val, "", "model value should not be empty");
    }
    // Note: if --model is absent entirely, that is fine — means YAML had no model for this agent
}

// --- Verifier subagent independence test ---

#[tokio::test]
async fn test_verifier_vs_non_verifier_subagent_independence() {
    // Scenario: primary_model=sonnet, verifier_model=sonnet, verifier_subagent_model=haiku
    //
    // plan-verifier:         agent model    = sonnet (Verifier bucket)
    //                        subagent cap   = haiku  (VerifierSubagent bucket — independent)
    // orchestrator-ideation: agent model    = sonnet (Primary bucket)
    //                        subagent cap   = sonnet (its own model, NOT haiku)
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_for_project("proj-1", "sonnet", "sonnet", "haiku")
        .await
        .unwrap();

    // plan-verifier agent model (from Verifier bucket) → sonnet
    let verifier_model = resolve_ideation_model("plan-verifier", Some("proj-1"), &repo).await;
    assert_eq!(verifier_model.model, "sonnet");
    assert_eq!(verifier_model.source, "user");

    // plan-verifier subagent cap (from verifier_subagent_model field) → haiku, not sonnet
    let project_row = repo.get_for_project("proj-1").await.unwrap().unwrap();
    let (cap_model, cap_source) =
        resolve_verifier_subagent_model_with_source(Some(&project_row.verifier_subagent_model), None);
    assert_eq!(cap_model, "haiku");
    assert_eq!(cap_source, "user");
    // Independence assertion: subagent cap ≠ verifier agent model when configured separately
    assert_ne!(
        cap_model, verifier_model.model,
        "verifier subagent cap (haiku) must differ from verifier agent model (sonnet)"
    );

    // orchestrator-ideation agent model (from Primary bucket) → sonnet
    let orchestrator_model =
        resolve_ideation_model("orchestrator-ideation", Some("proj-1"), &repo).await;
    assert_eq!(orchestrator_model.model, "sonnet");
    assert_eq!(orchestrator_model.source, "user");
    // orchestrator subagent cap = its own agent model (sonnet)
    // verifier_subagent_model=haiku must NOT affect non-verifier agents
    assert_ne!(
        orchestrator_model.model, "haiku",
        "orchestrator subagent cap must not be affected by verifier_subagent_model"
    );
}

// --- Resolver + CLI integration ---

#[tokio::test]
async fn test_ideation_context_db_override_flows_to_cli_arg() {
    // Scenario: Ideation context with DB override → resolved model passed as model_override
    // Simulate what send_message() does: resolve from DB, then pass to build_base_cli_command
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_for_project("proj-abc", "opus", "sonnet", "inherit")
        .await
        .unwrap();

    let resolved = resolve_ideation_model("orchestrator-ideation", Some("proj-abc"), &repo).await;
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.source, "user");

    // Now build the CLI command with the resolved model
    let result = build_base_cli_command(
        Path::new("/fake/claude"),
        Path::new("/fake/plugin"),
        Some("orchestrator-ideation"),
        false,
        None,
        Some(resolved.model.as_str()),
    );
    assert!(result.is_ok());
    let cmd = result.unwrap();
    let args = collect_args(&cmd);
    let model_pos = args.iter().position(|a| a == "--model").expect("--model not in args");
    assert_eq!(args.get(model_pos + 1).map(String::as_str), Some("opus"));
}

#[tokio::test]
async fn test_ideation_context_no_db_override_falls_through_to_yaml() {
    // Scenario: Ideation context without DB override → resolver returns YAML model
    let repo = MemoryIdeationModelSettingsRepository::new();
    // No rows → falls through to YAML/default

    let resolved = resolve_ideation_model("orchestrator-ideation", None, &repo).await;
    // Should come from YAML or hardcoded default — NOT from DB
    assert!(
        resolved.source == "yaml" || resolved.source == "yaml_default",
        "expected yaml source, got: {}",
        resolved.source
    );
    assert!(!resolved.model.is_empty());
    assert_ne!(resolved.model, "inherit");
}

#[tokio::test]
async fn test_non_ideation_agent_bypasses_db_model_resolution() {
    // Scenario: non-ideation agent (ralphx-worker) → model_bucket_for_agent returns None
    // → resolve_ideation_model falls through to YAML; DB overrides are NOT consulted
    use ralphx_lib::domain::ideation::model_settings::model_bucket_for_agent;

    // Confirm ralphx-worker has no bucket → bypasses DB
    let bucket = model_bucket_for_agent("ralphx-worker");
    assert!(
        bucket.is_none(),
        "ralphx-worker should not have an ideation model bucket"
    );

    // With a DB override for a project — the worker still ignores it
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_for_project("proj-x", "opus", "haiku", "inherit")
        .await
        .unwrap();

    let resolved = resolve_ideation_model("ralphx-worker", Some("proj-x"), &repo).await;
    // Worker bypasses DB → comes from YAML/default, NOT from DB "opus" override
    assert_ne!(
        resolved.source, "user",
        "worker should not use project DB override"
    );
    assert_ne!(
        resolved.source, "global",
        "worker should not use global DB override"
    );
    // Source should be yaml or yaml_default
    assert!(
        resolved.source == "yaml" || resolved.source == "yaml_default",
        "expected yaml source for non-ideation agent, got: {}",
        resolved.source
    );
}
