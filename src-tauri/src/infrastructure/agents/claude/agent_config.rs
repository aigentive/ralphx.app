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
    // Orchestrator-ideation: facilitates brainstorming, creates task proposals
    // Read-only access to codebase, uses MCP for proposal creation
    AgentConfig {
        name: "orchestrator-ideation",
        allowed_tools: Some("Read,Grep,Glob"),
    },
    // Session namer: generates titles for ideation sessions
    // No CLI tools needed, only uses MCP update_session_title
    AgentConfig {
        name: "session-namer",
        allowed_tools: Some(""),
    },
    // =========================================================================
    // CHAT AGENTS
    // =========================================================================
    // Chat-task: assists with task-specific questions
    // Read-only, uses MCP for task updates
    AgentConfig {
        name: "chat-task",
        allowed_tools: Some("Read,Grep,Glob"),
    },
    // Chat-project: assists with project-level questions
    // Read-only, uses MCP for task suggestions
    AgentConfig {
        name: "chat-project",
        allowed_tools: Some("Read,Grep,Glob"),
    },
    // Review-chat: discusses review findings with user
    // Read-only, uses MCP for approve/request changes
    AgentConfig {
        name: "ralphx-review-chat",
        allowed_tools: Some("Read,Grep,Glob"),
    },
    // =========================================================================
    // EXECUTION AGENTS
    // =========================================================================
    // Worker: implements tasks autonomously
    // Full access - needs Write, Edit, Bash for implementation
    AgentConfig {
        name: "ralphx-worker",
        allowed_tools: None, // All tools
    },
    // Reviewer: reviews code changes
    // Read + Bash for running tests, no file writes
    AgentConfig {
        name: "ralphx-reviewer",
        allowed_tools: Some("Read,Grep,Glob,Bash"),
    },
    // =========================================================================
    // QA AGENTS
    // =========================================================================
    // QA-prep: generates acceptance criteria and test steps
    // Read-only analysis
    AgentConfig {
        name: "ralphx-qa-prep",
        allowed_tools: Some("Read,Grep,Glob"),
    },
    // QA-executor: runs browser tests
    // Read + Bash for executing agent-browser commands
    AgentConfig {
        name: "ralphx-qa-executor",
        allowed_tools: Some("Read,Grep,Glob,Bash"),
    },
    // =========================================================================
    // COORDINATION AGENTS
    // =========================================================================
    // Orchestrator: plans and coordinates complex tasks
    // Full access for delegation and coordination
    AgentConfig {
        name: "ralphx-orchestrator",
        allowed_tools: None, // All tools
    },
    // Supervisor: monitors task execution
    // Read + Bash for checking status, no writes
    AgentConfig {
        name: "ralphx-supervisor",
        allowed_tools: Some("Read,Grep,Glob,Bash"),
    },
    // Deep-researcher: conducts thorough research
    // Full access including web tools
    AgentConfig {
        name: "ralphx-deep-researcher",
        allowed_tools: None, // All tools
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_allowed_tools_restricted_agent() {
        let tools = get_allowed_tools("orchestrator-ideation");
        assert_eq!(tools, Some("Read,Grep,Glob"));
    }

    #[test]
    fn test_get_allowed_tools_unrestricted_agent() {
        let tools = get_allowed_tools("ralphx-worker");
        assert_eq!(tools, None);
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
}
