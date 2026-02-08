// Agent Tool Configuration
//
// Single source of truth for which CLI tools each agent can access.
// MCP tools are separately controlled via RALPHX_AGENT_TYPE + server-side filtering
// in ralphx-mcp-server/src/tools.ts (TOOL_ALLOWLIST).
//
// ## How It Works
//
// When spawning an agent via CLI with `--agent <name> -p "prompt"`:
// - Frontmatter `tools`/`disallowedTools` fields are IGNORED (they only work for subagent spawning)
// - We must pass `--tools "Tool1,Tool2"` CLI flag to restrict built-in tools
// - MCP tools remain available based on RALPHX_AGENT_TYPE environment variable
//
// ## Adding a New Agent
//
// 1. Add entry to AGENT_CONFIGS below
// 2. Create agent definition in ralphx-plugin/agents/<name>.md
// 3. Add MCP tools to TOOL_ALLOWLIST in ralphx-mcp-server/src/tools.ts

/// Base CLI tools shared by all non-MCP-only agents.
/// Agents that need more (Write, Edit, Task, etc.) declare them in `extra_cli_tools`.
pub const BASE_CLI_TOOLS: &[&str] = &["Read", "Grep", "Glob", "Bash", "WebFetch", "WebSearch"];

/// Configuration for an agent's CLI tool access
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name (matches --agent flag and plugin filename without .md)
    pub name: &'static str,
    /// If true, agent gets no CLI tools (empty --tools ""), only MCP tools
    pub mcp_only: bool,
    /// Extra CLI tools beyond BASE_CLI_TOOLS (e.g. Write, Edit, Task)
    /// Ignored when mcp_only is true
    pub extra_cli_tools: &'static [&'static str],
    /// Allowed MCP tools (passed via --allowedTools flag)
    /// These are the tool names WITHOUT the mcp__ralphx__ prefix
    /// Empty slice = no MCP tools
    pub allowed_mcp_tools: &'static [&'static str],
    /// CLI tools to pre-approve (bypass permission prompts)
    /// These are added to --allowedTools alongside MCP tools
    /// Common values: "Write", "Edit", "Bash(npm:*)", etc.
    pub preapproved_cli_tools: &'static [&'static str],
}

/// All agent configurations
///
/// This is the single source of truth for agent tool restrictions.
/// Keep this in sync with:
/// - ralphx-plugin/agents/*.md (frontmatter for documentation)
/// - ralphx-mcp-server/src/tools.ts (TOOL_ALLOWLIST for MCP tools)
pub const AGENT_CONFIGS: &[AgentConfig] = &[
    // =========================================================================
    // IDEATION AGENTS
    // =========================================================================
    AgentConfig {
        name: "orchestrator-ideation",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[
            "create_task_proposal",
            "update_task_proposal",
            "delete_task_proposal",
            "list_session_proposals",
            "get_proposal",
            "analyze_session_dependencies",
            "create_plan_artifact",
            "update_plan_artifact",
            "get_plan_artifact",
            "link_proposals_to_plan",
            "get_session_plan",
            "ask_user_question",
        ],
        preapproved_cli_tools: &["Task"],
    },
    // Read-only variant for accepted plans - no mutation tools
    AgentConfig {
        name: "orchestrator-ideation-readonly",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[
            "list_session_proposals",
            "get_proposal",
            "get_plan_artifact",
            "get_session_plan",
        ],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "session-namer",
        mcp_only: true,
        extra_cli_tools: &[],
        allowed_mcp_tools: &["update_session_title"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "dependency-suggester",
        mcp_only: true,
        extra_cli_tools: &[],
        allowed_mcp_tools: &["apply_proposal_dependencies"],
        preapproved_cli_tools: &[],
    },
    // =========================================================================
    // CHAT AGENTS
    // =========================================================================
    AgentConfig {
        name: "chat-task",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &["update_task", "add_task_note", "get_task_details"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "chat-project",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &["suggest_task", "list_tasks"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "ralphx-review-chat",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[
            "approve_task",
            "request_task_changes",
            "get_review_notes",
            "get_task_context",
            "get_artifact",
            "get_artifact_version",
            "get_related_artifacts",
            "search_project_artifacts",
            "get_task_steps",
        ],
        preapproved_cli_tools: &[],
    },
    // =========================================================================
    // EXECUTION AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-worker",
        mcp_only: false,
        extra_cli_tools: &["Write", "Edit", "Task"],
        allowed_mcp_tools: &[
            "start_step",
            "complete_step",
            "skip_step",
            "fail_step",
            "add_step",
            "get_step_progress",
            "get_task_context",
            "get_artifact",
            "get_artifact_version",
            "get_related_artifacts",
            "search_project_artifacts",
            "get_review_notes",
            "get_task_steps",
            "get_task_issues",
            "mark_issue_in_progress",
            "mark_issue_addressed",
        ],
        preapproved_cli_tools: &["Write", "Edit", "Bash"],
    },
    AgentConfig {
        name: "ralphx-reviewer",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[
            "complete_review",
            "get_task_context",
            "get_artifact",
            "get_artifact_version",
            "get_related_artifacts",
            "search_project_artifacts",
            "get_review_notes",
            "get_task_steps",
            "get_task_issues",
            "get_step_progress",
            "get_issue_progress",
        ],
        preapproved_cli_tools: &["Bash"],
    },
    // =========================================================================
    // QA AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-qa-prep",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "ralphx-qa-executor",
        mcp_only: false,
        extra_cli_tools: &["Write", "Edit", "Task(Explore,Plan)"],
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "Edit", "Bash"],
    },
    // =========================================================================
    // COORDINATION AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-orchestrator",
        mcp_only: false,
        extra_cli_tools: &["Write", "Edit", "Task"],
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "Edit", "Bash", "Task"],
    },
    AgentConfig {
        name: "ralphx-supervisor",
        mcp_only: false,
        extra_cli_tools: &["Task(Explore,Plan)"],
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Bash"],
    },
    AgentConfig {
        name: "ralphx-deep-researcher",
        mcp_only: false,
        extra_cli_tools: &["Write", "Task(Explore,Plan)"],
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "WebFetch", "WebSearch"],
    },
    // =========================================================================
    // MERGE AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-merger",
        mcp_only: false,
        extra_cli_tools: &["Edit", "Task(Explore,Plan)"],
        allowed_mcp_tools: &[
            "complete_merge",
            "report_conflict",
            "report_incomplete",
            "get_merge_target",
            "get_task_context",
        ],
        preapproved_cli_tools: &["Read", "Edit", "Bash"],
    },
];

/// Get allowed tools for an agent by name
///
/// Returns:
/// - Some(Some("tools")) = use these specific tools
/// - Some(None) = no restrictions (all tools allowed)
/// - None = agent not found in config (defaults to no restrictions)
pub fn get_agent_config(agent_name: &str) -> Option<&'static AgentConfig> {
    AGENT_CONFIGS.iter().find(|c| c.name == agent_name)
}

/// Get the --tools argument value for an agent
///
/// Returns:
/// - Some("") = pass --tools "" (MCP-only agent, no CLI tools)
/// - Some("Read,Grep,Glob,Bash,WebFetch,WebSearch,...") = pass --tools with this value
/// - None = agent not found (don't pass --tools)
pub fn get_allowed_tools(agent_name: &str) -> Option<String> {
    get_agent_config(agent_name).map(|c| {
        if c.mcp_only {
            String::new()
        } else {
            let mut tools: Vec<&str> = BASE_CLI_TOOLS.to_vec();
            tools.extend_from_slice(c.extra_cli_tools);
            tools.join(",")
        }
    })
}

/// Get the --allowedTools argument value for an agent
///
/// Returns formatted string combining:
/// - MCP tools with mcp__ralphx__ prefix
/// - Pre-approved CLI tools (Write, Edit, Bash, etc.)
///
/// Returns None if agent has no tools to pre-approve.
pub fn get_allowed_mcp_tools(agent_name: &str) -> Option<String> {
    get_agent_config(agent_name).and_then(|c| {
        let mut tools: Vec<String> = Vec::new();

        // Add MCP tools with prefix
        for t in c.allowed_mcp_tools {
            tools.push(format!("mcp__ralphx__{}", t));
        }

        // Add pre-approved CLI tools (no prefix needed)
        for t in c.preapproved_cli_tools {
            tools.push((*t).to_string());
        }

        if tools.is_empty() {
            None
        } else {
            Some(tools.join(","))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_allowed_tools_restricted_agent() {
        let tools = get_allowed_tools("orchestrator-ideation");
        assert_eq!(tools, Some("Read,Grep,Glob,Bash,WebFetch,WebSearch,Task(Explore,Plan)".to_string()));
    }

    #[test]
    fn test_get_allowed_tools_worker_agent() {
        let tools = get_allowed_tools("ralphx-worker");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        // Base tools
        for base in BASE_CLI_TOOLS {
            assert!(tools.contains(base), "worker missing base tool: {}", base);
        }
        // Extra tools
        assert!(tools.contains("Write"));
        assert!(tools.contains("Edit"));
        assert!(tools.contains("Task"));
    }

    #[test]
    fn test_get_allowed_tools_mcp_only_agent() {
        let tools = get_allowed_tools("session-namer");
        assert_eq!(tools, Some(String::new()));
    }

    #[test]
    fn test_get_allowed_tools_unknown_agent() {
        let tools = get_allowed_tools("unknown-agent");
        assert_eq!(tools, None);
    }

    #[test]
    fn test_all_agents_have_unique_names() {
        let mut names: Vec<_> = AGENT_CONFIGS.iter().map(|c| c.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "Duplicate agent names found");
    }

    #[test]
    fn test_get_allowed_mcp_tools_with_tools() {
        let tools = get_allowed_mcp_tools("orchestrator-ideation");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("mcp__ralphx__create_task_proposal"));
        assert!(tools.contains("mcp__ralphx__list_session_proposals"));
        assert!(tools.contains("mcp__ralphx__ask_user_question"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_ideation_ask_user_question() {
        let tools = get_allowed_mcp_tools("orchestrator-ideation");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("mcp__ralphx__ask_user_question"));
        // Task should be in preapproved CLI tools
        assert!(tools.contains("Task"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_no_tools() {
        // qa-prep has no MCP tools and no preapproved CLI tools
        let tools = get_allowed_mcp_tools("ralphx-qa-prep");
        assert!(tools.is_none());
    }

    #[test]
    fn test_get_allowed_mcp_tools_cli_only() {
        // supervisor has no MCP tools but has preapproved CLI tools
        let tools = get_allowed_mcp_tools("ralphx-supervisor");
        assert!(tools.is_some());
        assert!(tools.unwrap().contains("Bash"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_unknown_agent() {
        let tools = get_allowed_mcp_tools("unknown-agent");
        assert!(tools.is_none());
    }

    #[test]
    fn test_get_allowed_tools_merger_agent() {
        let tools = get_allowed_tools("ralphx-merger");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        // Base tools
        for base in BASE_CLI_TOOLS {
            assert!(tools.contains(base), "merger missing base tool: {}", base);
        }
        // Extra tools
        assert!(tools.contains("Edit"));
        assert!(tools.contains("Task(Explore,Plan)"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_merger_agent() {
        let tools = get_allowed_mcp_tools("ralphx-merger");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("mcp__ralphx__complete_merge"));
        assert!(tools.contains("mcp__ralphx__report_conflict"));
        assert!(tools.contains("mcp__ralphx__report_incomplete"));
        assert!(tools.contains("mcp__ralphx__get_merge_target"));
        assert!(tools.contains("mcp__ralphx__get_task_context"));
        // Also includes preapproved CLI tools
        assert!(tools.contains("Read"));
        assert!(tools.contains("Edit"));
        assert!(tools.contains("Bash"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_worker_review_issues() {
        let tools = get_allowed_mcp_tools("ralphx-worker");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("mcp__ralphx__get_task_issues"));
        assert!(tools.contains("mcp__ralphx__mark_issue_in_progress"));
        assert!(tools.contains("mcp__ralphx__mark_issue_addressed"));
    }

    #[test]
    fn test_get_allowed_mcp_tools_reviewer_review_issues() {
        let tools = get_allowed_mcp_tools("ralphx-reviewer");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("mcp__ralphx__get_task_issues"));
        assert!(tools.contains("mcp__ralphx__get_step_progress"));
        assert!(tools.contains("mcp__ralphx__get_issue_progress"));
    }

    #[test]
    fn test_all_non_mcp_only_agents_include_base_tools() {
        for config in AGENT_CONFIGS {
            if config.mcp_only {
                continue;
            }
            let tools = get_allowed_tools(config.name)
                .unwrap_or_else(|| panic!("Agent {} not found", config.name));
            for base in BASE_CLI_TOOLS {
                assert!(
                    tools.contains(base),
                    "Agent {} missing base CLI tool: {}",
                    config.name,
                    base
                );
            }
        }
    }

    #[test]
    fn test_mcp_only_agents_get_empty_tools() {
        for config in AGENT_CONFIGS {
            if !config.mcp_only {
                continue;
            }
            let tools = get_allowed_tools(config.name)
                .unwrap_or_else(|| panic!("Agent {} not found", config.name));
            assert!(
                tools.is_empty(),
                "MCP-only agent {} should have empty tools, got: {}",
                config.name,
                tools
            );
        }
    }
}
