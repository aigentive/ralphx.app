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
        name: "list_design_source_files",
        description: "List backend-validated source files selected for the current design system. Use this before reading or searching source content.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                project_id: {
                    type: "string",
                    description: "Optional selected source project id to filter files.",
                },
                max_files: {
                    type: "integer",
                    description: "Optional result cap. Defaults to 500.",
                },
            },
        },
    },
    {
        name: "read_design_source_file",
        description: "Read a file from the selected design source manifest. The backend rejects paths outside the chosen source scope.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                project_id: {
                    type: "string",
                    description: "Optional selected source project id. Required when a path appears in more than one source project.",
                },
                path: {
                    type: "string",
                    description: "Manifest-relative source path returned by list_design_source_files.",
                },
                start_line: {
                    type: "integer",
                    description: "Optional 1-based inclusive start line.",
                },
                end_line: {
                    type: "integer",
                    description: "Optional 1-based inclusive end line.",
                },
                max_bytes: {
                    type: "integer",
                    description: "Optional byte cap for the read. Defaults to 65536.",
                },
            },
            required: ["path"],
        },
    },
    {
        name: "search_design_source_files",
        description: "Search selected design source files by literal text. The backend searches only files stored in the design source manifest.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                project_id: {
                    type: "string",
                    description: "Optional selected source project id to filter files.",
                },
                pattern: {
                    type: "string",
                    description: "Literal text to search for.",
                },
                case_sensitive: {
                    type: "boolean",
                    description: "Whether the search is case-sensitive. Defaults to false.",
                },
                max_results: {
                    type: "integer",
                    description: "Optional match cap. Defaults to 100.",
                },
            },
            required: ["pattern"],
        },
    },
    {
        name: "publish_design_schema_version",
        description: "Publish a new RalphX-owned design schema/styleguide version from source-grounded styleguide rows. The backend owns version ids, run state, storage, previews, and event emission.",
        inputSchema: {
            type: "object",
            properties: {
                design_system_id: {
                    type: "string",
                    description: CURRENT_DESIGN_CONTEXT_DESCRIPTION,
                },
                version: {
                    type: "string",
                    description: "Optional semantic version label. The backend chooses the next version when omitted.",
                },
                items: {
                    type: "array",
                    description: "Human-reviewable styleguide rows grounded in selected source refs.",
                    items: {
                        type: "object",
                        properties: {
                            item_id: {
                                type: "string",
                                description: "Stable row id such as colors.primary_palette or components.buttons.",
                            },
                            group: {
                                type: "string",
                                enum: ["ui_kit", "type", "colors", "spacing", "components", "brand"],
                            },
                            label: { type: "string" },
                            summary: { type: "string" },
                            source_refs: {
                                type: "array",
                                items: {
                                    type: "object",
                                    properties: {
                                        project_id: { type: "string" },
                                        path: { type: "string" },
                                        line: { type: "integer" },
                                    },
                                    required: ["project_id", "path"],
                                },
                            },
                            confidence: {
                                type: "string",
                                enum: ["high", "medium", "low"],
                            },
                        },
                        required: ["item_id", "group", "label", "summary"],
                    },
                },
            },
            required: ["items"],
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