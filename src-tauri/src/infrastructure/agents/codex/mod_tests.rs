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
