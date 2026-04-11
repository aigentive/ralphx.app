use super::{
    load_canonical_agent_definition, load_harness_agent_prompt, resolve_harness_agent_prompt_path,
    try_load_canonical_claude_metadata, AgentPromptHarness,
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

const CROSS_HARNESS_EXECUTION_AGENTS: &[(&str, &str, &str)] = &[
    ("ralphx-reviewer", "reviewer", "reviewer"),
    ("ralphx-merger", "merger", "merger"),
];

const CROSS_HARNESS_WORKFLOW_AGENTS: &[(&str, &str, &str)] = &[
    ("ralphx-worker", "worker", "worker"),
    ("ralphx-coder", "worker", "coder"),
    ("ralphx-review-chat", "review_chat", "review-chat"),
];

const CROSS_HARNESS_CHAT_AGENTS: &[(&str, &str, &str)] = &[
    ("chat-task", "task_chat", "chat-task"),
    ("chat-project", "project_chat", "chat-project"),
];

const CROSS_HARNESS_SUPPORT_AGENTS: &[(&str, &str, &str)] = &[
    ("ralphx-review-history", "review_history", "review-history"),
    ("project-analyzer", "project_analyzer", "project-analyzer"),
    ("memory-capture", "memory_capture", "memory-capture"),
    ("memory-maintainer", "memory_maintainer", "memory-maintainer"),
];

const CROSS_HARNESS_GENERAL_AGENTS: &[(&str, &str, &str)] = &[
    ("ralphx-deep-researcher", "researcher", "deep-researcher"),
    ("ralphx-orchestrator", "orchestrator", "orchestrator"),
    ("ralphx-supervisor", "supervisor", "supervisor"),
    ("ralphx-qa-prep", "qa_prep", "qa-prep"),
    ("ralphx-qa-executor", "qa_executor", "qa-executor"),
];

const CROSS_HARNESS_READONLY_IDEATION_AGENTS: &[(&str, &str, &str)] = &[(
    "orchestrator-ideation-readonly",
    "ideation_orchestrator_readonly",
    "orchestrator-ideation-readonly",
)];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn canonical_agent_names(root: &std::path::Path) -> Vec<String> {
    let mut names = std::fs::read_dir(root.join("agents"))
        .expect("canonical agents dir should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;
            if file_type.is_dir() {
                Some(entry.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn live_runtime_agents() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut agents = Vec::new();
    agents.extend_from_slice(PILOT_AGENTS);
    agents.extend_from_slice(CLAUDE_ONLY_CANONICAL_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_EXECUTION_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_WORKFLOW_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_CHAT_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_SUPPORT_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_GENERAL_AGENTS);
    agents.extend_from_slice(CROSS_HARNESS_READONLY_IDEATION_AGENTS);
    agents
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

    for (agent_name, role, _) in CROSS_HARNESS_EXECUTION_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CROSS_HARNESS_WORKFLOW_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CROSS_HARNESS_CHAT_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CROSS_HARNESS_SUPPORT_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CROSS_HARNESS_GENERAL_AGENTS {
        let definition = load_canonical_agent_definition(&root, agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, *agent_name);
        assert_eq!(definition.role, *role);
    }

    for (agent_name, role, _) in CROSS_HARNESS_READONLY_IDEATION_AGENTS {
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

    for (agent_name, _, _) in CROSS_HARNESS_EXECUTION_AGENTS {
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

    for (agent_name, _, _) in CROSS_HARNESS_WORKFLOW_AGENTS {
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

    for (agent_name, _, _) in CROSS_HARNESS_CHAT_AGENTS {
        let claude_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude);
        let codex_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex);
        assert!(
            claude_path.is_some(),
            "expected claude prompt path for {agent_name}"
        );
        assert!(
            codex_path.is_some(),
            "expected codex prompt path for {agent_name}"
        );
        assert!(
            claude_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} claude prompt to resolve through shared/prompt.md"
        );
        assert!(
            codex_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} codex prompt to resolve through shared/prompt.md"
        );
    }

    for (agent_name, _, _) in CROSS_HARNESS_SUPPORT_AGENTS {
        let claude_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude);
        let codex_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex);
        assert!(
            claude_path.is_some(),
            "expected claude prompt path for {agent_name}"
        );
        assert!(
            codex_path.is_some(),
            "expected codex prompt path for {agent_name}"
        );
        assert!(
            claude_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} claude prompt to resolve through shared/prompt.md"
        );
        assert!(
            codex_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} codex prompt to resolve through shared/prompt.md"
        );
    }

    for (agent_name, _, _) in CROSS_HARNESS_GENERAL_AGENTS {
        let claude_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Claude);
        let codex_path =
            resolve_harness_agent_prompt_path(&root, agent_name, AgentPromptHarness::Codex);
        assert!(
            claude_path.is_some(),
            "expected claude prompt path for {agent_name}"
        );
        assert!(
            codex_path.is_some(),
            "expected codex prompt path for {agent_name}"
        );
        assert!(
            claude_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} claude prompt to resolve through shared/prompt.md"
        );
        assert!(
            codex_path
                .as_ref()
                .is_some_and(|path| path.ends_with(format!("agents/{agent_name}/shared/prompt.md"))),
            "expected {agent_name} codex prompt to resolve through shared/prompt.md"
        );
    }

    for (agent_name, _, _) in CROSS_HARNESS_READONLY_IDEATION_AGENTS {
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
fn live_runtime_agents_resolve_through_canonical_claude_prompts() {
    let root = project_root();

    for (agent_name, _, _) in live_runtime_agents() {
        assert!(
            load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Claude).is_some(),
            "missing canonical Claude prompt for {agent_name}"
        );
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

#[test]
fn codex_execution_prompts_avoid_claude_only_team_and_task_syntax() {
    let root = project_root();
    let banned_terms = [
        "Task(",
        "TaskCreate",
        "TaskUpdate",
        "TaskGet",
        "TaskList",
        "TaskOutput",
        "TaskStop",
        "TeamCreate",
        "TeamDelete",
        "SendMessage",
        "mcpServers",
        "CLAUDE_PLUGIN_ROOT",
        "--append-system-prompt",
    ];

    for (agent_name, _, _) in CROSS_HARNESS_EXECUTION_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }

    for (agent_name, _, _) in CROSS_HARNESS_WORKFLOW_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }

    for (agent_name, _, _) in CROSS_HARNESS_CHAT_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }

    for (agent_name, _, _) in CROSS_HARNESS_SUPPORT_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }

    for (agent_name, _, _) in CROSS_HARNESS_GENERAL_AGENTS {
        let prompt = load_harness_agent_prompt(&root, agent_name, AgentPromptHarness::Codex)
            .unwrap_or_else(|| panic!("missing codex prompt for {agent_name}"));
        for banned in banned_terms {
            assert!(
                !prompt.contains(banned),
                "codex prompt for {agent_name} must not contain Claude-only syntax `{banned}`"
            );
        }
    }

    for (agent_name, _, _) in CROSS_HARNESS_READONLY_IDEATION_AGENTS {
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
fn canonical_agent_tree_is_schema_valid_and_loadable() {
    let root = project_root();

    for agent_name in canonical_agent_names(&root) {
        let definition = load_canonical_agent_definition(&root, &agent_name)
            .unwrap_or_else(|| panic!("expected canonical definition for {agent_name}"));
        assert_eq!(definition.name, agent_name);

        try_load_canonical_claude_metadata(&root, &agent_name)
            .unwrap_or_else(|error| panic!("invalid claude metadata for {agent_name}: {error}"));

        let shared_prompt_path = root
            .join("agents")
            .join(&agent_name)
            .join("shared/prompt.md");
        let claude_prompt_path = root
            .join("agents")
            .join(&agent_name)
            .join("claude/prompt.md");
        let codex_prompt_path = root
            .join("agents")
            .join(&agent_name)
            .join("codex/prompt.md");

        assert!(
            shared_prompt_path.exists() || claude_prompt_path.exists() || codex_prompt_path.exists(),
            "canonical agent {agent_name} must define at least one prompt"
        );
    }
}
