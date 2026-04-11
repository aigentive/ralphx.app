use super::compose_codex_prompt;
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

    std::fs::create_dir_all(root.join("agents/session-namer/codex"))
        .expect("create canonical codex dir");
    std::fs::write(
        root.join("agents/session-namer/shared.yaml"),
        "name: session-namer\nrole: session_namer\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/session-namer/codex/prompt.md"),
        "Canonical Codex Prompt",
    )
    .expect("write canonical codex prompt");
    std::fs::write(
        plugin_dir.join("agents/session-namer.md"),
        "---\nname: session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("session-namer"));

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
        plugin_dir.join("agents/session-namer.md"),
        "---\nname: session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("session-namer"));

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

    std::fs::create_dir_all(root.join("agents/session-namer/shared"))
        .expect("create shared prompt dir");
    std::fs::write(
        root.join("agents/session-namer/shared.yaml"),
        "name: session-namer\nrole: session_namer\nshared_prompt_harnesses:\n  - codex\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/session-namer/shared/prompt.md"),
        "Shared Session Namer Prompt",
    )
    .expect("write shared prompt");
    std::fs::write(
        plugin_dir.join("agents/session-namer.md"),
        "---\nname: session-namer\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("session-namer"));

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

    std::fs::create_dir_all(root.join("agents/ideation-team-lead/claude"))
        .expect("create canonical claude dir");
    std::fs::write(
        root.join("agents/ideation-team-lead/shared.yaml"),
        "name: ideation-team-lead\nrole: ideation_team_lead\n",
    )
    .expect("write shared definition");
    std::fs::write(
        root.join("agents/ideation-team-lead/claude/prompt.md"),
        "Canonical Claude Prompt",
    )
    .expect("write canonical claude prompt");
    std::fs::write(
        plugin_dir.join("agents/ideation-team-lead.md"),
        "---\nname: ideation-team-lead\n---\nLegacy Claude Prompt",
    )
    .expect("write legacy prompt");

    let composed = compose_codex_prompt("User prompt", Some(&plugin_dir), Some("ideation-team-lead"));

    assert_eq!(
        composed, "User prompt",
        "canonical agents without a codex prompt should not silently inherit the legacy claude prompt"
    );
}
