/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */
import { safeError } from "./redact.js";
import { PLAN_TOOLS } from "./plan-tools.js";
import { WORKER_CONTEXT_TOOLS } from "./worker-context-tools.js";
import { STEP_TOOLS } from "./step-tools.js";
import { ISSUE_TOOLS } from "./issue-tools.js";
import { FILESYSTEM_TOOLS } from "./filesystem-tools.js";
import { IDEATION_TOOLS } from "./ideation-tools.js";
import { WORKFLOW_TOOLS } from "./workflow-tools.js";
import { getAllowedToolNames as resolveAllowedToolNames, getToolsByAgent as resolveToolsByAgent, parseAllowedToolsFromArgs as parseAllowedToolsFromKnownRegistry, } from "./tool-authorization.js";
export { TOOL_ALLOWLIST, setAgentType, getAgentType } from "./tool-authorization.js";
/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export const ALL_TOOLS = [
    ...FILESYSTEM_TOOLS,
    ...IDEATION_TOOLS,
    ...WORKFLOW_TOOLS,
    // ========================================================================
    // PLAN ARTIFACT TOOLS (ralphx-ideation agent)
    // ========================================================================
    ...PLAN_TOOLS,
    // ========================================================================
    // WORKER CONTEXT TOOLS (worker agent)
    // ========================================================================
    ...WORKER_CONTEXT_TOOLS,
    // ========================================================================
    // STEP TOOLS (worker agent)
    // ========================================================================
    ...STEP_TOOLS,
    // ========================================================================
    // ISSUE TOOLS (worker + reviewer agents)
    // ========================================================================
    ...ISSUE_TOOLS,
    // ========================================================================
    // MEMORY WRITE TOOLS (memory agents only - restricted via allowlist)
    // ========================================================================
    {
        name: "upsert_memories",
        description: "Batch upsert memory entries to SQLite canonical storage. " +
            "Performs content-hash deduplication to prevent duplicates. " +
            "WRITE-ONLY tool restricted to ralphx-memory-maintainer and ralphx-memory-capture agents.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (from RALPHX_PROJECT_ID env var)",
                },
                memories: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            bucket: {
                                type: "string",
                                enum: ["architecture_patterns", "implementation_discoveries", "operational_playbooks"],
                                description: "Memory bucket classification",
                            },
                            title: {
                                type: "string",
                                description: "Concise title for this memory (50-80 chars)",
                            },
                            summary: {
                                type: "string",
                                description: "Brief summary suitable for rule index files (1-3 sentences)",
                            },
                            details_markdown: {
                                type: "string",
                                description: "Full markdown details with examples, context, and rationale",
                            },
                            scope_paths: {
                                type: "array",
                                items: { type: "string" },
                                description: "Glob patterns for path scoping (e.g., ['src/domain/**', 'src-tauri/src/application/**'])",
                            },
                            source_context_type: {
                                type: "string",
                                description: "Optional: context type (e.g., 'task_execution', 'planning', 'review')",
                            },
                            source_context_id: {
                                type: "string",
                                description: "Optional: source context ID (e.g., task_id, session_id)",
                            },
                            source_conversation_id: {
                                type: "string",
                                description: "Optional: conversation ID for traceability",
                            },
                            quality_score: {
                                type: "number",
                                description: "Optional: quality score 0-1 (higher = more valuable)",
                            },
                        },
                        required: ["bucket", "title", "summary", "details_markdown", "scope_paths"],
                    },
                    description: "Array of memory entries to upsert",
                },
            },
            required: ["project_id", "memories"],
        },
    },
    {
        name: "mark_memory_obsolete",
        description: "Mark a memory entry as obsolete (soft delete). " +
            "The memory remains in DB but is excluded from index generation and searches. " +
            "WRITE-ONLY tool restricted to ralphx-memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                memory_id: {
                    type: "string",
                    description: "The memory entry ID to mark obsolete",
                },
            },
            required: ["memory_id"],
        },
    },
    {
        name: "refresh_memory_rule_index",
        description: "Regenerate .claude/rules/ index files from DB canonical state. " +
            "Reads memory entries for project, groups by scope_key, and writes index files with summaries + memory IDs. " +
            "WRITE-ONLY tool restricted to ralphx-memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                scope_key: {
                    type: "string",
                    description: "Optional: specific scope_key to refresh. If omitted, refreshes all rule indexes for project.",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "ingest_rule_file",
        description: "Ingest a .claude/rules/*.md file into canonical memory DB. " +
            "Parses content into chunks, classifies buckets, upserts to memory_entries, " +
            "rewrites file to index format, and enqueues archive jobs. " +
            "WRITE-ONLY tool restricted to ralphx-memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                rule_file_path: {
                    type: "string",
                    description: "Path to rule file relative to project root (e.g., '.claude/rules/task-state-machine.md')",
                },
            },
            required: ["project_id", "rule_file_path"],
        },
    },
    {
        name: "rebuild_archive_snapshots",
        description: "Enqueue full rebuild of archive snapshots from DB canonical state. " +
            "Generates .claude/memory-archive/ snapshots for disaster recovery. " +
            "WRITE-ONLY tool restricted to ralphx-memory-maintainer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "get_conversation_transcript",
        description: "Retrieve conversation messages for a given conversation ID, ordered chronologically. Used by ralphx-memory-capture for analysis.",
        inputSchema: {
            type: "object",
            properties: {
                conversation_id: {
                    type: "string",
                    description: "The conversation ID",
                },
            },
            required: ["conversation_id"],
        },
    },
    // ========================================================================
    // PROJECT ANALYSIS TOOLS (worker/reviewer/merger + ralphx-project-analyzer agents)
    // ========================================================================
    {
        name: "get_project_analysis",
        description: "Get project analysis data including build commands, validation commands, and worktree setup instructions. " +
            "Returns path-scoped entries with resolved template variables ({project_root}, {worktree_path}, {task_branch}). " +
            "If analysis hasn't been run yet, returns { status: 'analyzing', retry_after_secs: 30 }.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID (from RALPHX_PROJECT_ID env var)",
                },
                task_id: {
                    type: "string",
                    description: "Optional task ID for resolving {worktree_path} and {task_branch} template variables",
                },
            },
            required: ["project_id"],
        },
    },
    {
        name: "save_project_analysis",
        description: "Save auto-detected project analysis data. Updates detected_analysis and analyzed_at fields. " +
            "Never touches custom_analysis (user overrides). Only callable by the ralphx-project-analyzer agent.",
        inputSchema: {
            type: "object",
            properties: {
                project_id: {
                    type: "string",
                    description: "The project ID",
                },
                entries: {
                    type: "array",
                    items: {
                        type: "object",
                        properties: {
                            path: {
                                type: "string",
                                description: "Subpath relative to project root (e.g., '.', 'src-tauri/')",
                            },
                            label: {
                                type: "string",
                                description: "Human-readable label (e.g., 'Node.js root', 'Rust backend')",
                            },
                            install: {
                                type: "string",
                                description: "Install command (e.g., 'npm install'). Null if not needed.",
                            },
                            validate: {
                                type: "array",
                                items: { type: "string" },
                                description: "Validation commands (e.g., ['npm run typecheck', 'npm run lint'])",
                            },
                            worktree_setup: {
                                type: "array",
                                items: { type: "string" },
                                description: "Commands to run in worktree setup (e.g., ['ln -s {project_root}/node_modules {worktree_path}/node_modules'])",
                            },
                        },
                        required: ["path", "label"],
                    },
                    description: "Array of path-scoped analysis entries",
                },
            },
            required: ["project_id", "entries"],
        },
    },
    // ========================================================================
    // CROSS-PROJECT TOOLS (ralphx-ideation + ralphx-ideation-team-lead)
    // ========================================================================
    {
        name: "list_projects",
        description: "List all RalphX projects on this instance. Returns project id, name, working_directory, and status for each project. " +
            "Use to discover existing projects before creating cross-project sessions.",
        inputSchema: {
            type: "object",
            properties: {},
            required: [],
        },
    },
    {
        name: "create_cross_project_session",
        description: "Create a new ideation session in a target project with an inherited plan from the current project. " +
            "The backend resolves the target project by filesystem path (auto-creates a RalphX project if none exists at that path). " +
            "The inherited plan is set to 'imported_verified' status — no auto-verify triggered. " +
            "Use when exporting a verified plan to another project for execution.",
        inputSchema: {
            type: "object",
            properties: {
                target_project_path: {
                    type: "string",
                    description: "Absolute filesystem path to the target project root. Backend resolves or auto-creates the RalphX project.",
                },
                source_session_id: {
                    type: "string",
                    description: "ID of the source ideation session whose verified plan will be inherited.",
                },
                title: {
                    type: "string",
                    description: "Optional title for the new session. Defaults to 'Imported: {source session title}'.",
                },
            },
            required: ["target_project_path", "source_session_id"],
        },
    },
    {
        name: "cross_project_guide",
        description: "Analyze a plan for cross-project paths and return structured guidance for multi-project orchestration. " +
            "Detects file paths referencing different project roots, suggests how to split proposals across projects, " +
            "and provides step-by-step instructions for creating sessions in target projects. " +
            "When session_id is provided: calls the backend to set the cross-project gate, unlocking proposal creation (gate_status: 'set'). " +
            "Without session_id: returns analysis only, gate is not set (gate_status: 'no_session_id').",
        inputSchema: {
            type: "object",
            properties: {
                session_id: {
                    type: "string",
                    description: "Session ID to fetch the plan content from (uses get_session_plan internally). " +
                        "Provide either session_id or plan_content, not both.",
                },
                plan_content: {
                    type: "string",
                    description: "Raw plan text to analyze directly. " +
                        "Provide either plan_content or session_id, not both.",
                },
            },
            required: [],
        },
    },
    {
        name: "migrate_proposals",
        description: "Copy proposals from a source ideation session to a target ideation session. " +
            "Each proposal is cloned with a new UUID; migrated_from traceability fields are set automatically. " +
            "Dependencies between migrated proposals are remapped to the new IDs. " +
            "Dependencies to proposals outside the migration set are dropped with warnings in the response. " +
            "Use this to move cross-project proposals to the correct project session after using create_cross_project_session.",
        inputSchema: {
            type: "object",
            properties: {
                source_session_id: {
                    type: "string",
                    description: "ID of the source ideation session to copy proposals from.",
                },
                target_session_id: {
                    type: "string",
                    description: "ID of the target ideation session to copy proposals into.",
                },
                proposal_ids: {
                    type: "array",
                    items: { type: "string" },
                    description: "Optional list of proposal IDs to migrate. If omitted, all proposals from the source session are considered (subject to target_project_filter).",
                },
                target_project_filter: {
                    type: "string",
                    description: "Optional: only migrate proposals whose target_project field matches this string. " +
                        "Useful for migrating only the proposals intended for a specific project.",
                },
            },
            required: ["source_session_id", "target_session_id"],
        },
    },
    // ========================================================================
    // CHILD SESSION TOOLS (ralphx-ideation, ralphx-ideation-team-lead, ralphx-plan-verifier)
    // ========================================================================
    {
        name: "get_child_session_status",
        description: "Returns live status of a child session: session metadata, agent process state (idle/likely_generating/likely_waiting), " +
            "recent messages, and verification metadata if applicable. Use to check if a verification agent is stalled, " +
            "monitor child session progress, or verify agent completion. " +
            "When diagnosing a verification child, set include_recent_messages=true so you can inspect the last assistant/tool outputs instead of guessing what happened.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "verification-child-session-id",
                    include_recent_messages: true,
                    message_limit: 10,
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "The child session ID to check",
                },
                include_recent_messages: {
                    type: "boolean",
                    description: "Include recent conversation messages (default: false)",
                },
                message_limit: {
                    type: "number",
                    description: "Max messages to return (default: 5, max: 50). Only used when include_recent_messages=true.",
                },
            },
            required: ["session_id"],
        },
    },
    {
        name: "send_ideation_session_message",
        description: "Sends a message to any ideation session's agent conversation by session ID. " +
            "Zero directionality enforcement — any session can message any active session (parent→child, child→parent, or peer). " +
            "If agent is generating, message is queued. If agent is idle, a new agent run is spawned. " +
            "Returns delivery_status: 'sent' (written to active stdin), " +
            "'queued' (agent busy, will receive on next turn), or 'spawned' (new agent run started). " +
            "Use to nudge verification agents, inject context, send escalation payloads, or send stop signals. " +
            "When nudging critics/verifiers, repeat the full invariant context they need (for example SESSION_ID, ROUND, expected artifact prefix/schema) instead of sending a vague follow-up.",
        inputSchema: {
            type: "object",
            examples: [
                {
                    session_id: "verification-child-session-id",
                    message: "SESSION_ID: <parent-session-id>\nROUND: 2\nIf you are still running, publish your TeamResearch artifact now using the parent ideation session_id and the required JSON schema.",
                },
            ],
            properties: {
                session_id: {
                    type: "string",
                    description: "The target ideation session ID to message",
                },
                message: {
                    type: "string",
                    description: "The message content to send to the session's agent",
                },
            },
            required: ["session_id", "message"],
        },
    },
];
const ALL_TOOL_NAMES = ALL_TOOLS.map((tool) => tool.name);
export function parseAllowedToolsFromArgs() {
    return parseAllowedToolsFromKnownRegistry(ALL_TOOL_NAMES);
}
export function getAllowedToolNames() {
    return resolveAllowedToolNames(ALL_TOOL_NAMES);
}
/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export function getFilteredTools() {
    const allowedNames = getAllowedToolNames();
    return ALL_TOOLS.filter((tool) => allowedNames.includes(tool.name));
}
/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export function isToolAllowed(toolName) {
    const allowedNames = getAllowedToolNames();
    return allowedNames.includes(toolName);
}
/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export function getAllTools() {
    return ALL_TOOLS;
}
/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export function getToolsByAgent() {
    return resolveToolsByAgent(ALL_TOOL_NAMES);
}
function formatToolExamples(tool, limit = 1) {
    const examples = (tool.inputSchema?.examples ?? [])
        .slice(0, limit)
        .map((example) => {
        try {
            return JSON.stringify(example);
        }
        catch {
            return String(example);
        }
    })
        .filter((example) => example.length > 0);
    return examples;
}
/**
 * Return a compact repair hint for high-friction tools so weaker models can retry
 * with the expected payload shape instead of probing by trial and error.
 */
export function getToolRecoveryHint(toolName) {
    const tool = ALL_TOOLS.find((candidate) => candidate.name === toolName);
    if (!tool) {
        return null;
    }
    switch (toolName) {
        case "update_plan_verification": {
            const examples = formatToolExamples(tool, 2);
            return [
                "Use the PARENT ideation session_id as the canonical target. If a verification child session_id is passed, the backend remaps it automatically.",
                "If report_verification_round / complete_plan_verification are available, prefer those narrower helpers instead of this generic tool.",
                "Use status=reviewing with in_progress=true for mid-round updates; use verified or needs_revision with in_progress=false for terminal updates.",
                "Re-read get_plan_verification if generation/in_progress is unclear instead of guessing.",
                ...examples.map((example, index) => index === 0
                    ? `Example reviewing payload: ${example}`
                    : `Example terminal payload: ${example}`),
            ].join("\n");
        }
        case "report_verification_round": {
            const examples = formatToolExamples(tool);
            return [
                "Use this verifier-friendly helper for in-progress rounds on the PARENT ideation session.",
                "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                "You only provide round, gaps, and generation; status=reviewing and in_progress=true are filled in automatically.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "complete_plan_verification": {
            const examples = formatToolExamples(tool, 2);
            return [
                "Use this verifier-friendly helper for terminal verification updates on the PARENT ideation session.",
                "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                "You provide the terminal status and generation; in_progress=false is filled in automatically.",
                "External sessions cannot use status=skipped.",
                ...examples.map((example, index) => index === 0
                    ? `Example terminal payload: ${example}`
                    : `Example abort-cleanup payload: ${example}`),
            ].join("\n");
        }
        case "get_plan_verification": {
            const examples = formatToolExamples(tool);
            return [
                "Call this on the PARENT ideation session before retrying report_verification_round, complete_plan_verification, or update_plan_verification. If a verification child session_id is passed, the backend remaps it to the parent automatically.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "create_team_artifact": {
            const examples = formatToolExamples(tool);
            return [
                "Use the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
                "For verifier critics, keep the exact artifact prefix and publish partial results instead of exploring further.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_team_artifacts": {
            const examples = formatToolExamples(tool);
            return [
                "Read artifacts from the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
                "Verification flows should usually prefer get_verification_round_artifacts instead of manually sorting summaries and then loading full artifact ids.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_verification_round_artifacts": {
            const examples = formatToolExamples(tool);
            return [
                "Use this verifier helper instead of manually calling get_team_artifacts + get_artifact + client-side sorting for current-round artifacts.",
                "Provide the parent ideation session_id plus the title prefixes you expect; the MCP proxy filters by created_after and returns the latest match per prefix.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "get_child_session_status": {
            const examples = formatToolExamples(tool);
            return [
                "When debugging a verification child, set include_recent_messages=true so you can inspect the last assistant/tool outputs.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        case "send_ideation_session_message": {
            const examples = formatToolExamples(tool);
            return [
                "When nudging a verifier/critic, repeat full invariant context: SESSION_ID, ROUND, artifact prefix/schema, and explicit parent-session target.",
                ...examples.map((example) => `Example payload: ${example}`),
            ].join("\n");
        }
        default: {
            const examples = formatToolExamples(tool);
            if (examples.length === 0) {
                return null;
            }
            return examples.map((example) => `Example payload: ${example}`).join("\n");
        }
    }
}
/**
 * Format a backend error message with an optional tool-specific usage hint.
 */
export function formatToolErrorMessage(toolName, message, details) {
    const repairHint = getToolRecoveryHint(toolName);
    return (`ERROR: ${message}` +
        (details ? `\n\nDetails: ${details}` : "") +
        (repairHint ? `\n\nUsage hint for ${toolName}:\n${repairHint}` : ""));
}
/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export function logAllTools() {
    console.error("\n=== RalphX MCP Server - All Available Tools ===\n");
    for (const [agentType, tools] of Object.entries(getToolsByAgent())) {
        if (tools.length > 0) {
            safeError(`[${agentType}]`);
            tools.forEach((t) => safeError(`  - ${t}`));
            console.error("");
        }
    }
    console.error("=== End of Tools List ===\n");
}
//# sourceMappingURL=tools.js.map