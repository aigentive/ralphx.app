/**
 * Agent workspace MCP tool definitions.
 *
 * These are intentionally separate from task-pipeline workflow tools.
 */
export const AGENT_WORKSPACE_TOOLS = [
    {
        name: "complete_agent_workspace_repair",
        description: "Signal that an agent workspace publish/update repair has been committed and is ready for backend verification. " +
            "Call this only after the workspace branch contains the current base, the repair is committed, and the worktree is clean.",
        inputSchema: {
            type: "object",
            properties: {
                conversation_id: {
                    type: "string",
                    description: "The agent workspace conversation ID from the repair prompt",
                },
                repair_commit_sha: {
                    type: "string",
                    description: "Full 40-character SHA of the current workspace HEAD (from `git rev-parse HEAD`)",
                },
                resolved_base_ref: {
                    type: "string",
                    description: "The base ref that was resolved into the workspace branch",
                },
                resolved_base_commit: {
                    type: "string",
                    description: "Full 40-character SHA of the resolved base ref",
                },
                summary: {
                    type: "string",
                    description: "Brief summary of the repair performed",
                },
            },
            required: [
                "conversation_id",
                "repair_commit_sha",
                "resolved_base_ref",
                "resolved_base_commit",
                "summary",
            ],
        },
    },
];
//# sourceMappingURL=agent-workspace-tools.js.map