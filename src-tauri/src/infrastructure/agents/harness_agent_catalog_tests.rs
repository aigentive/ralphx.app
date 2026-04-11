use super::{
    load_canonical_agent_definition, load_harness_agent_prompt, resolve_harness_agent_prompt_path,
    AgentPromptHarness,
};
use std::path::PathBuf;

const PILOT_AGENTS: &[(&str, &str, &str)] = &[
    (
        "orchestrator-ideation",
        "ideation_orchestrator",
        "orchestrator-ideation",
    ),
    ("ideation-team-lead", "ideation_team_lead", "ideation-team-lead"),
    ("session-namer", "session_namer", "session-namer"),
];

const CODEX_PILOT_AGENTS: &[&str] = &["orchestrator-ideation", "session-namer"];
const CLAUDE_ONLY_CANONICAL_AGENTS: &[(&str, &str, &str)] = &[
    ("plan-verifier", "plan_verifier", "plan-verifier"),
    (
        "plan-critic-completeness",
        "plan_critic_completeness",
        "plan-critic-completeness",
    ),
    (
        "plan-critic-implementation-feasibility",
        "plan_critic_implementation_feasibility",
        "plan-critic-implementation-feasibility",
    ),
    (
        "ideation-specialist-backend",
        "ideation_specialist_backend",
        "ideation-specialist-backend",
    ),
    (
        "ideation-specialist-frontend",
        "ideation_specialist_frontend",
        "ideation-specialist-frontend",
    ),
    (
        "ideation-specialist-infra",
        "ideation_specialist_infra",
        "ideation-specialist-infra",
    ),
    ("ideation-specialist-ux", "ideation_specialist_ux", "ideation-specialist-ux"),
    (
        "ideation-specialist-code-quality",
        "ideation_specialist_code_quality",
        "ideation-specialist-code-quality",
    ),
    (
        "ideation-specialist-prompt-quality",
        "ideation_specialist_prompt_quality",
        "ideation-specialist-prompt-quality",
    ),
    (
        "ideation-specialist-intent",
        "ideation_specialist_intent",
        "ideation-specialist-intent",
    ),
    (
        "ideation-specialist-state-machine",
        "ideation_specialist_state_machine",
        "ideation-specialist-state-machine",
    ),
    (
        "ideation-specialist-pipeline-safety",
        "ideation_specialist_pipeline_safety",
        "ideation-specialist-pipeline-safety",
    ),
    ("ideation-advocate", "ideation_advocate", "ideation-advocate"),
    ("ideation-critic", "ideation_critic", "ideation-critic"),
    ("ralphx-worker-team", "worker_team_lead", "worker-team"),
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

    for (agent_name, role, _) in PILOT_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CLAUDE_ONLY_CANONICAL_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }
}

#[test]
fn pilot_agent_prompt_paths_exist_for_both_harnesses() {
    let root = project_root();

    for agent_name in CODEX_PILOT_AGENTS {
        let claude_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude);
        assert!(
            claude_path.is_some(),
            "expected claude prompt path for {agent_name}"
        );
        let codex_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex);
        assert!(
            codex_path.is_some(),
            "expected codex prompt path for {agent_name}"
        );

        if agent_name == &"session-namer" {
            assert!(
                claude_path
                    .as_ref()
                    .is_some_and(|path| path.ends_with("agents/session-namer/shared/prompt.md")),
                "expected session-namer claude prompt to resolve through shared/prompt.md"
            );
            assert!(
                codex_path
                    .as_ref()
                    .is_some_and(|path| path.ends_with("agents/session-namer/shared/prompt.md")),
                "expected session-namer codex prompt to resolve through shared/prompt.md"
            );
        }
    }

    assert!(
        resolve_harness_agent_prompt_path(&root, "ideation-team-lead", AgentPromptHarness::Claude)
            .is_some(),
        "expected claude prompt path for ideation-team-lead"
    );
    assert!(
        resolve_harness_agent_prompt_path(&root, "ideation-team-lead", AgentPromptHarness::Codex)
            .is_none(),
        "ideation-team-lead should not have a codex prompt while Codex team mode is unsupported"
    );

    for (agent_name, _, _) in CLAUDE_ONLY_CANONICAL_AGENTS {
        assert!(
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude)
                .is_some(),
            "expected claude prompt path for {agent_name}"
        );
        assert!(
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex)
                .is_none(),
            "{agent_name} should remain claude-only until a codex prompt exists"
        );
    }
}

#[test]
fn canonical_claude_prompts_match_legacy_prompt_bodies_for_pilot_agents() {
    let root = project_root();

    for (agent_name, _, legacy_file_stem) in PILOT_AGENTS {
        let canonical = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Claude)
            .unwrap_or_else(|| panic!("missing canonical claude prompt for {agent_name}"));
        let legacy_raw = std::fs::read_to_string(
            root.join("plugins/app/agents")
                .join(format!("{legacy_file_stem}.md")),
        )
        .unwrap_or_else(|_| panic!("missing legacy prompt for {agent_name}"));

        assert_eq!(canonical, strip_frontmatter(&legacy_raw));
    }

    for (agent_name, _, legacy_file_stem) in CLAUDE_ONLY_CANONICAL_AGENTS {
        let canonical = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Claude)
            .unwrap_or_else(|| panic!("missing canonical claude prompt for {agent_name}"));
        let legacy_raw = std::fs::read_to_string(
            root.join("plugins/app/agents")
                .join(format!("{legacy_file_stem}.md")),
        )
        .unwrap_or_else(|_| panic!("missing legacy prompt for {agent_name}"));

        assert_eq!(canonical, strip_frontmatter(&legacy_raw));
    }
}

#[test]
fn codex_pilot_prompts_are_frontmatter_free() {
    let root = project_root();

    for agent_name in CODEX_PILOT_AGENTS {
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

#[test]
fn codex_pilot_prompts_avoid_claude_only_team_and_task_syntax() {
    let root = project_root();
    let banned_terms = [
        "Task(",
        "TeamCreate",
        "TaskCreate",
        "TaskUpdate",
        "TaskGet",
        "TaskList",
        "TaskOutput",
        "TaskStop",
        "SendMessage",
        "mcpServers",
        "CLAUDE_PLUGIN_ROOT",
        "--append-system-prompt",
    ];

    for agent_name in ["orchestrator-ideation", "session-namer"] {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }
}

#[test]
fn codex_ideation_pilot_prompts_declare_codex_native_delegation_contract() {
    let root = project_root();

    let orchestrator = load_harness_agent_prompt(
        &root,
        "orchestrator-ideation",
        AgentPromptHarness::Codex,
    )
    .expect("missing codex orchestrator prompt");
    assert!(
        orchestrator.contains("Codex-native delegation"),
        "orchestrator codex prompt should describe Codex-native delegation semantics"
    );
    assert!(
        load_harness_agent_prompt(&root, "ideation-team-lead", AgentPromptHarness::Codex).is_none(),
        "ideation-team-lead should not have a codex prompt while team mode is unsupported"
    );
}
