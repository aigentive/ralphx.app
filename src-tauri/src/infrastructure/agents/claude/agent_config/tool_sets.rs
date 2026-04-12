use std::collections::HashMap;

pub const CANONICAL_BASE_TOOLS: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "Bash",
    "WebFetch",
    "WebSearch",
    "Skill",
    "TaskCreate",
    "TaskUpdate",
    "TaskGet",
    "TaskList",
    "TaskOutput",
    "KillShell",
    "MCPSearch",
];

pub const CANONICAL_CRITIC_TOOLS: &[&str] = &["Read", "Grep", "Glob"];

pub fn canonical_claude_tool_sets() -> HashMap<&'static str, &'static [&'static str]> {
    HashMap::from([
        ("base_tools", CANONICAL_BASE_TOOLS),
        ("critic_tools", CANONICAL_CRITIC_TOOLS),
    ])
}
