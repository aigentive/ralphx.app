const CURRENT_DESIGN_CONTEXT_DESCRIPTION = "Optional design system id. Defaults to the current RalphX design chat context.";
export const DESIGN_TOOLS = [
    {
        name: "get_design_system",
        description: "Read the current design system summary, selected source projects, and linked design conversation.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
            },
        },
    },
    {
        name: "get_design_source_manifest",
        description: "Read the backend-validated source manifest for the current design system, including selected source scopes and recorded hashes.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
            },
        },
    },
    {
        name: "get_design_styleguide",
        description: "Read styleguide rows for the current design system and optional schema version.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                schema_version_id: {
                    type: "string",
                    description: "Optional schema version id. Defaults to the current schema.",
                },
            },
        },
    },
    {
        name: "update_design_styleguide_item",
        description: "Set the review status for one styleguide item in the current design system.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                item_id: {
                    type: "string",
                    description: "Stable styleguide item id such as colors-primary or components-button.",
                },
                approval_status: {
                    type: "string",
                    enum: ["needs_review", "approved", "needs_work"],
                    description: "New review status for the styleguide item.",
                },
            },
            required: ["item_id", "approval_status"],
        },
    },
    {
        name: "record_design_styleguide_feedback",
        description: "Record explicit user feedback for one styleguide item in Design state without adding another chat message.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                item_id: {
                    type: "string",
                    description: "Stable styleguide item id receiving feedback.",
                },
                feedback: {
                    type: "string",
                    description: "Concrete feedback to record for the item.",
                },
                conversation_id: {
                    type: "string",
                    description: "Optional design conversation id. Defaults to the active design conversation.",
                },
            },
            required: ["item_id", "feedback"],
        },
    },
    {
        name: "create_design_artifact",
        description: "Generate a RalphX-owned, reviewable screen or component design artifact from the current design schema without writing to source projects.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                artifact_kind: {
                    type: "string",
                    enum: ["screen", "component"],
                    description: "Design artifact kind to generate. Screen artifacts should use compact workspace preview patterns; component artifacts should use realistic state previews.",
                },
                name: {
                    type: "string",
                    description: "Human name for the generated screen or component artifact.",
                },
                brief: {
                    type: "string",
                    description: "Optional concise intent or constraints. Include the row pattern, expected preview states, and any source-backed caveats the renderer should preserve.",
                },
                source_item_id: {
                    type: "string",
                    description: "Optional styleguide item id to ground the artifact and preview pattern. Defaults to a matching screen or component row.",
                },
            },
            required: ["artifact_kind", "name"],
        },
    },
    {
        name: "list_design_artifacts",
        description: "List current schema, source-audit, styleguide, and run-output artifacts for the current design system without exposing storage paths.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                schema_version_id: {
                    type: "string",
                    description: "Optional schema version id. Defaults to the current schema.",
                },
            },
        },
    },
];
//# sourceMappingURL=design-tools.js.map