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
        description: "Create a new implementation plan artifact linked to the ideation session. Use this when the user describes a complex feature that needs architectural planning before breaking into tasks. The plan is stored as a Specification artifact and can be referenced by task proposals. " +
            "For child sessions that inherited a parent's plan: calling this creates a completely independent plan for the child session — it does NOT modify or copy from the parent's plan.",
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
        description: "Update an existing implementation plan's content. Creates a NEW version with a new artifact ID (immutable version chain). Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before updating. Linked proposals are automatically re-linked to the new version (plan_version_at_creation is preserved). The response includes `previous_artifact_id` and `session_id` for reference. You do NOT need to call get_session_plan between updates to refresh the ID.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to update. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
                content: {
                    type: "string",
                    description: "Updated plan content in markdown format. This will create a new version of the artifact with a new ID.",
                },
            },
            required: ["artifact_id", "content"],
        },
    },
    {
        name: "get_plan_artifact",
        description: "Retrieve a plan artifact's content by version ID. Returns the content of the specific version requested (not necessarily the latest). Use get_session_plan to find the current latest version for a session. Note: unlike update_plan_artifact and link_proposals_to_plan, this does NOT auto-resolve stale IDs.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the specific version to retrieve. Returns that version's content, not the latest.",
                },
            },
            required: ["artifact_id"],
        },
    },
    {
        name: "link_proposals_to_plan",
        description: "Link multiple task proposals to an implementation plan. Use after creating proposals to establish the connection between the plan and its derived tasks. Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before linking. This enables traceability and allows the system to suggest updates when the plan changes.",
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
                    description: "The plan artifact ID to link proposals to. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
            },
            required: ["proposal_ids", "artifact_id"],
        },
    },
    {
        name: "get_session_plan",
        description: "Get the implementation plan artifact for the current ideation session, if one exists. Use to check if a plan has already been created before suggesting a new one. " +
            "Response includes an `is_inherited` boolean: if true, the plan was inherited from a parent session and is read-only — call create_plan_artifact to create an independent plan for this session.",
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
    {
        name: "update_plan_verification",
        description: "Update verification state for an ideation session. Reports round results from critic analysis. Call after each adversarial review round to record gaps found, and when verification converges (status=verified or skipped).",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "Ideation session ID",
                },
                status: {
                    type: "string",
                    enum: ["reviewing", "needs_revision", "verified", "skipped"],
                    description: "New verification status",
                },
                in_progress: {
                    type: "boolean",
                    description: "Whether verification loop is active",
                },
                round: {
                    type: "integer",
                    description: "Current round number (1-based)",
                },
                gaps: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            severity: {
                                type: "string",
                                enum: ["critical", "high", "medium", "low"],
                            },
                            category: { type: "string" },
                            description: { type: "string" },
                            why_it_matters: { type: "string" },
                        },
                        required: ["severity", "category", "description"],
                    },
                    description: "Gaps identified in this round",
                },
                convergence_reason: {
                    type: "string",
                    enum: [
                        "zero_critical",
                        "jaccard_converged",
                        "max_rounds",
                        "critic_parse_failure",
                        "user_skipped",
                        "user_reverted",
                    ],
                    description: "Why verification converged (only when status=verified or skipped)",
                },
                generation: {
                    type: "integer",
                    description: "Generation counter for zombie protection. Pass in every call when in auto-verify mode. Server rejects in_progress=true if generation mismatches.",
                },
            },
            required: ["session_id", "status"],
        },
    },
    {
        name: "get_plan_verification",
        description: "Get the current verification status for an ideation session. Returns status, round number, gap list, and convergence reason if applicable.",
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
    {
        name: "revert_and_skip",
        description: "Atomically revert the plan to a previous version and skip verification. " +
            "Use when the user wants to discard recent plan changes and proceed without re-running adversarial review. " +
            "Restores the exact content of the specified plan version, creates a new artifact entry for auditability, " +
            "and sets verification status to skipped in a single atomic operation.",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "The ideation session ID",
                },
                plan_version_to_restore: {
                    type: "string",
                    description: "The artifact ID of the plan version to restore content from. " +
                        "Typically a previous version's artifact ID (e.g., from before edits were made).",
                },
            },
            required: ["session_id", "plan_version_to_restore"],
        },
    },
    {
        name: "edit_plan_artifact",
        description: "Apply anchor-based edit operations to an existing implementation plan. More token-efficient than update_plan_artifact for targeted changes — only send the text to find and replace, not the entire plan content. Each edit finds the first occurrence of old_text and replaces it with new_text. Stale artifact IDs are auto-resolved to the latest version. Edits are applied sequentially; if any edit fails (old_text not found or ambiguous), the entire operation is rejected with details of which edit failed.",
        inputSchema: {
            type: "object",
            properties: {
                artifact_id: {
                    type: "string",
                    description: "The artifact ID of the plan to edit. Can be any version ID — stale IDs are auto-resolved to the latest version.",
                },
                edits: {
                    type: "array",
                    minItems: 1,
                    maxItems: 20,
                    items: {
                        type: "object",
                        properties: {
                            old_text: {
                                type: "string",
                                minLength: 1,
                                description: "The exact text to find in the plan. Must be unique within the plan content to avoid ambiguous replacements.",
                            },
                            new_text: {
                                type: "string",
                                description: "The replacement text. Can be empty string to delete the matched text.",
                            },
                        },
                        required: ["old_text", "new_text"],
                    },
                    description: "List of edit operations to apply sequentially. Each operation finds old_text and replaces with new_text.",
                },
            },
            required: ["artifact_id", "edits"],
        },
    },
];
//# sourceMappingURL=plan-tools.js.map