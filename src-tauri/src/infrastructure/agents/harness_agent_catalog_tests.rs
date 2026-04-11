use super::{
    load_canonical_agent_definition, load_harness_agent_prompt, resolve_harness_agent_prompt_path,
    AgentPromptHarness,
};
use std::path::PathBuf;

const PILOT_AGENTS: &[(&str, &str)] = &[
    ("orchestrator-ideation", "ideation_orchestrator"),
    ("ideation-team-lead", "ideation_team_lead"),
    ("session-namer", "session_namer"),
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn strip_frontmatter(markdown: &str) -> String {
    if let Some(after_first) = markdown.strip_prefix("---") {
        if let Some(end_idx) = after_first.find("\n---") {
            let body = &after_first[end_idx + "\n---".len()..];
            return body.trim().to_string();
        }
    }

    markdown.trim().to_string()
}

#[test]
fn pilot_agent_definitions_load_from_canonical_tree() {
    let root = project_root();

    for (agent_name, role) in PILOT_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
        assert!(
            definition
                .claude_plugin_output
                .as_deref()
                .is_some_and(|path| path.ends_with(&format!("{agent_name}.md"))),
            "expected claude plugin output path for {agent_name}"
        );
    }
}

#[test]
fn pilot_agent_prompt_paths_exist_for_both_harnesses() {
    let root = project_root();

    for (agent_name, _) in PILOT_AGENTS {
        assert!(
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude)
                .is_some(),
            "expected claude prompt path for {agent_name}"
        );
        assert!(
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex)
                .is_some(),
            "expected codex prompt path for {agent_name}"
        );
    }
}

#[test]
fn canonical_claude_prompts_match_legacy_prompt_bodies_for_pilot_agents() {
    let root = project_root();

    for (agent_name, _) in PILOT_AGENTS {
        let canonical = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Claude)
            .unwrap_or_else(|| panic!("missing canonical claude prompt for {agent_name}"));
        let legacy_raw = std::fs::read_to_string(
            root.join("plugins/app/agents")
                .join(format!("{agent_name}.md")),
        )
        .unwrap_or_else(|_| panic!("missing legacy prompt for {agent_name}"));

        assert_eq!(canonical, strip_frontmatter(&legacy_raw));
    }
}

#[test]
fn codex_pilot_prompts_are_frontmatter_free() {
    let root = project_root();

    for (agent_name, _) in PILOT_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        assert!(
            !prompt.starts_with("---"),
            "codex prompt for {agent_name} must not carry Claude frontmatter"
        );
        assert!(
            !prompt.contains("\nmcpServers:"),
            "codex prompt for {agent_name} must not carry Claude mcpServers frontmatter"
        );
    }
}
