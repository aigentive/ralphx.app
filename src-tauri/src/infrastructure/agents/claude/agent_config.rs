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

/// Configuration for an agent's CLI tool access
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name (matches --agent flag and plugin filename without .md)
    pub name: &'static str,
    /// Allowed CLI tools (passed via --tools flag)
    /// - Some("Read,Grep,Glob") = only these tools
    /// - Some("") = no CLI tools (MCP only)
    /// - None = inherit all tools (no restriction)
    pub allowed_tools: Option<&'static str>,
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
        allowed_tools: Some("Read,Grep,Glob"),
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
        ],
        preapproved_cli_tools: &[],
    },
    // Read-only variant for accepted plans - no mutation tools
    AgentConfig {
        name: "orchestrator-ideation-readonly",
        allowed_tools: Some("Read,Grep,Glob"),
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
        allowed_tools: Some(""),
        allowed_mcp_tools: &["update_session_title"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "dependency-suggester",
        allowed_tools: Some(""),
        allowed_mcp_tools: &["apply_proposal_dependencies"],
        preapproved_cli_tools: &[],
    },
    // =========================================================================
    // CHAT AGENTS
    // =========================================================================
    AgentConfig {
        name: "chat-task",
        allowed_tools: Some("Read,Grep,Glob"),
        allowed_mcp_tools: &["update_task", "add_task_note", "get_task_details"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "chat-project",
        allowed_tools: Some("Read,Grep,Glob"),
        allowed_mcp_tools: &["suggest_task", "list_tasks"],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "ralphx-review-chat",
        allowed_tools: Some("Read,Grep,Glob"),
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
        allowed_tools: Some("Read,Write,Edit,Bash,Grep,Glob,Task,WebFetch,WebSearch"),
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
        ],
        preapproved_cli_tools: &["Write", "Edit", "Bash"],
    },
    AgentConfig {
        name: "ralphx-reviewer",
        allowed_tools: Some("Read,Grep,Glob,Bash"),
        allowed_mcp_tools: &[
            "complete_review",
            "get_task_context",
            "get_artifact",
            "get_artifact_version",
            "get_related_artifacts",
            "search_project_artifacts",
            "get_review_notes",
            "get_task_steps",
        ],
        preapproved_cli_tools: &["Bash"],
    },
    // =========================================================================
    // QA AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-qa-prep",
        allowed_tools: Some("Read,Grep,Glob"),
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &[],
    },
    AgentConfig {
        name: "ralphx-qa-executor",
        allowed_tools: Some("Read,Write,Edit,Grep,Glob,Bash"),
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "Edit", "Bash"],
    },
    // =========================================================================
    // COORDINATION AGENTS
    // =========================================================================
    AgentConfig {
        name: "ralphx-orchestrator",
        allowed_tools: Some("Read,Write,Edit,Bash,Grep,Glob,Task,WebFetch,WebSearch"),
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "Edit", "Bash", "Task"],
    },
    AgentConfig {
        name: "ralphx-supervisor",
        allowed_tools: Some("Read,Grep,Glob,Bash"),
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Bash"],
    },
    AgentConfig {
        name: "ralphx-deep-researcher",
        allowed_tools: Some("Read,Write,Grep,Glob,WebFetch,WebSearch"),
        allowed_mcp_tools: &[],
        preapproved_cli_tools: &["Write", "WebFetch", "WebSearch"],
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
/// - Some("Read,Grep,Glob") = pass --tools with this value
/// - Some("") = pass --tools "" (no CLI tools)
/// - None = don't pass --tools (all tools allowed)
pub fn get_allowed_tools(agent_name: &str) -> Option<&'static str> {
    get_agent_config(agent_name).and_then(|c| c.allowed_tools)
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
        assert_eq!(tools, Some("Read,Grep,Glob"));
    }

    #[test]
    fn test_get_allowed_tools_worker_agent() {
        let tools = get_allowed_tools("ralphx-worker");
        assert!(tools.is_some());
        let tools = tools.unwrap();
        assert!(tools.contains("Write"));
        assert!(tools.contains("Edit"));
        assert!(tools.contains("Bash"));
    }

    #[test]
    fn test_get_allowed_tools_mcp_only_agent() {
        let tools = get_allowed_tools("session-namer");
        assert_eq!(tools, Some(""));
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
}
