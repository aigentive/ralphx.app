/**
 * MCP tool definitions for plan artifact management
 * Used by orchestrator-ideation agent to create and manage implementation plans
 */
/**
 * Plan artifact tools for orchestrator-ideation agent
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export const PLAN_TOOLS = [
    {
        name: "create_plan_artifact",
        description: "Create a new implementation plan artifact linked to the ideation session. Use this when the user describes a complex feature that needs architectural planning before breaking into tasks. The plan is stored as a Specification artifact and can be referenced by task proposals.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID (provided in context)",
                },
                title: {
                    type: "string",
                    description: "Plan title (e.g., 'Real-time Collaboration Implementation Plan')",
                },
                content: {
                    type: "string",
                    description: "Plan content in markdown format. Should include architecture decisions, data flow, key implementation details, and considerations.",
                },
            },
            required: ["session_id", "title", "content"],
        },
    },
    {
        name: "update_plan_artifact",
        description: "Update an existing implementation plan's content. Use when the user provides feedback, clarifications, or decisions that need to be incorporated into the plan. Creates a new version of the artifact.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to update",
                },
                content: {
                    type: "string",
                    description: "Updated plan content in markdown format. This will create a new version of the artifact.",
                },
            },
            required: ["artifact_id", "content"],
        },
    },
    {
        name: "get_plan_artifact",
        description: "Retrieve a plan artifact's current content. Use when you need to reference the plan details during conversation or when creating proposals.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to retrieve",
                },
            },
            required: ["artifact_id"],
        },
    },
    {
        name: "link_proposals_to_plan",
        description: "Link multiple task proposals to an implementation plan. Use after creating proposals to establish the connection between the plan and its derived tasks. This enables traceability and allows the system to suggest updates when the plan changes.",
        inputSchema: {
            type: "object",
            properties: {
                proposal_ids: {
                    type: "array",
                    items: { type: "string" },
                    description: "Array of proposal IDs to link to the plan",
                },
                artifact_id: {
                    type: "string",
                    description: "The plan artifact ID to link proposals to",
                },
            },
            required: ["proposal_ids", "artifact_id"],
        },
    },
    {
        name: "get_session_plan",
        description: "Get the implementation plan artifact for the current ideation session, if one exists. Use to check if a plan has already been created before suggesting a new one.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
            },
            required: ["session_id"],
        },
    },
];
//# sourceMappingURL=plan-tools.js.map