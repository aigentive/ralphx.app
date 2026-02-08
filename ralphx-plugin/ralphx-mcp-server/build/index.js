#!/usr/bin/env node
/**
 * RalphX MCP Server
 *
 * A proxy MCP server that forwards tool calls to the RalphX Tauri backend via HTTP.
 * All business logic lives in Rust - this server is a thin transport layer.
 *
 * Tool scoping:
 * - Reads agent type from CLI args (--agent-type=<type>) or environment (RALPHX_AGENT_TYPE)
 * - CLI args take precedence (because Claude CLI doesn't pass env vars to MCP servers)
 * - Filters available tools based on agent type (hard enforcement)
 * - Each agent only sees tools appropriate for its role
 */
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { CallToolRequestSchema, ListToolsRequestSchema, } from "@modelcontextprotocol/sdk/types.js";
import { callTauri, callTauriGet, TauriClientError } from "./tauri-client.js";
import { getFilteredTools, isToolAllowed, getAllowedToolNames, logAllTools, getToolsByAgent, setAgentType, } from "./tools.js";
import { permissionRequestTool, handlePermissionRequest, } from "./permission-handler.js";
import { handleAskUserQuestion } from "./question-handler.js";
/**
 * Parse command line arguments for --agent-type
 * Returns the agent type if found, undefined otherwise
 */
function parseAgentTypeFromArgs() {
    for (const arg of process.argv) {
        if (arg.startsWith("--agent-type=")) {
            return arg.substring("--agent-type=".length);
        }
        if (arg === "--agent-type") {
            const idx = process.argv.indexOf(arg);
            if (idx >= 0 && idx + 1 < process.argv.length) {
                return process.argv[idx + 1];
            }
        }
    }
    return undefined;
}
// Agent type: prefer CLI args over environment (Claude CLI doesn't pass env to MCP servers)
const cliAgentType = parseAgentTypeFromArgs();
const AGENT_TYPE = cliAgentType || process.env.RALPHX_AGENT_TYPE || "unknown";
// Set the agent type in tools module for filtering
setAgentType(AGENT_TYPE);
// Log how agent type was determined
if (cliAgentType) {
    console.error(`[RalphX MCP] Agent type from CLI args: ${AGENT_TYPE}`);
}
else if (process.env.RALPHX_AGENT_TYPE) {
    console.error(`[RalphX MCP] Agent type from env: ${AGENT_TYPE}`);
}
else {
    console.error(`[RalphX MCP] Agent type unknown (no CLI arg or env var)`);
}
// Task ID from environment (for task-level scoping enforcement)
const RALPHX_TASK_ID = process.env.RALPHX_TASK_ID;
// Project ID from environment (for project-level scoping enforcement)
const RALPHX_PROJECT_ID = process.env.RALPHX_PROJECT_ID;
/**
 * Validate that a tool call's task_id parameter matches the assigned task
 * @param toolName - Name of the tool being called
 * @param args - Arguments passed to the tool
 * @returns Error message if validation fails, null if validation passes or not applicable
 *
 * Test Cases:
 * 1. Non-scoped tool (get_artifact) => returns null (no validation)
 * 2. Scoped tool, no RALPHX_TASK_ID set => returns null (backward compat)
 * 3. Scoped tool, matching task_id => returns null (validation passed)
 * 4. Scoped tool, mismatched task_id => returns error message
 */
function validateTaskScope(toolName, args) {
    // Only validate tools that have task_id parameter directly
    // Note: start_step, complete_step, skip_step, fail_step take step_id, not task_id
    // The backend validates step ownership - we can't do it here without a DB lookup
    const taskScopedTools = [
        "complete_review",
        "approve_task",
        "request_task_changes",
        "update_task",
        "add_task_note",
        "get_task_details",
        "get_task_context",
        "get_review_notes",
        "get_task_steps",
        "add_step",
        "get_step_progress",
        // Merge tools (merger agent)
        "complete_merge",
        "report_conflict",
        "report_incomplete",
        "get_merge_target",
        // Issue tools (worker + reviewer agents)
        "get_task_issues",
        "get_issue_progress",
    ];
    if (!taskScopedTools.includes(toolName)) {
        return null; // No validation needed
    }
    if (!RALPHX_TASK_ID) {
        return null; // No task scope set, allow (backward compatibility)
    }
    const providedTaskId = args.task_id;
    if (providedTaskId !== RALPHX_TASK_ID) {
        return `ERROR: Task scope violation.\n\nYou are assigned to task "${RALPHX_TASK_ID}" but attempted to modify task "${providedTaskId}".\n\nYour assigned task details:\n- Task ID: ${RALPHX_TASK_ID}\n- You should only call ${toolName} with this task_id.\n\nPlease correct your tool call and try again.`;
    }
    return null; // Validation passed
}
/**
 * Validate that a tool call's project_id parameter matches the assigned project
 * @param toolName - Name of the tool being called
 * @param args - Arguments passed to the tool
 * @returns Error message if validation fails, null if validation passes or not applicable
 */
function validateProjectScope(toolName, args) {
    const projectScopedTools = [
        "get_project_analysis",
        "save_project_analysis",
    ];
    if (!projectScopedTools.includes(toolName)) {
        return null;
    }
    if (!RALPHX_PROJECT_ID) {
        return null; // No project scope set, allow (backward compatibility)
    }
    const providedProjectId = args.project_id;
    if (providedProjectId !== RALPHX_PROJECT_ID) {
        return `ERROR: Project scope violation.\n\nYou are assigned to project "${RALPHX_PROJECT_ID}" but attempted to access project "${providedProjectId}".\n\nPlease correct your tool call and try again.`;
    }
    return null;
}
/**
 * Create and configure the MCP server
 */
const server = new Server({
    name: "ralphx",
    version: "1.0.0",
}, {
    capabilities: {
        tools: {},
    },
});
/**
 * List available tools (filtered by agent type)
 * Note: permission_request tool is always included (not scoped by agent type)
 */
server.setRequestHandler(ListToolsRequestSchema, async () => {
    const tools = getFilteredTools();
    // Always include permission_request tool (not scoped by agent type)
    const allTools = [...tools, permissionRequestTool];
    // Log tool scoping for debugging
    const allowedNames = getAllowedToolNames();
    console.error(`[RalphX MCP] Agent type: ${AGENT_TYPE}, Tools: ${allowedNames.length > 0 ? allowedNames.join(", ") : "none"} + permission_request`);
    return { tools: allTools };
});
/**
 * Execute tool calls (with authorization check)
 */
server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    // Special handling for permission_request tool (always allowed, not scoped by agent type)
    if (name === "permission_request") {
        return handlePermissionRequest(args);
    }
    // Special handling for ask_user_question (register + long-poll, like permission_request)
    if (name === "ask_user_question") {
        // Still check authorization (must be in agent's allowlist)
        if (!isToolAllowed(name)) {
            return {
                content: [
                    {
                        type: "text",
                        text: `ERROR: Tool "${name}" is not available for agent type "${AGENT_TYPE}".`,
                    },
                ],
                isError: true,
            };
        }
        return handleAskUserQuestion(args);
    }
    // Authorization check (defense in depth)
    if (!isToolAllowed(name)) {
        const allowedNames = getAllowedToolNames();
        const errorMessage = allowedNames.length > 0
            ? `Tool "${name}" is not available for agent type "${AGENT_TYPE}". Allowed tools: ${allowedNames.join(", ")}`
            : `Agent type "${AGENT_TYPE}" has no MCP tools available. This agent should use filesystem tools (Read, Grep, Glob, Bash, Edit, Write) instead.`;
        console.error(`[RalphX MCP] Unauthorized tool call: ${name}`);
        return {
            content: [
                {
                    type: "text",
                    text: `ERROR: ${errorMessage}`,
                },
            ],
            isError: true,
        };
    }
    // Task scope validation
    const scopeError = validateTaskScope(name, args || {});
    if (scopeError) {
        console.error(`[RalphX MCP] Task scope violation: ${name}`);
        return {
            content: [
                {
                    type: "text",
                    text: scopeError,
                },
            ],
            isError: true,
        };
    }
    // Project scope validation
    const projectScopeError = validateProjectScope(name, args || {});
    if (projectScopeError) {
        console.error(`[RalphX MCP] Project scope violation: ${name}`);
        return {
            content: [
                {
                    type: "text",
                    text: projectScopeError,
                },
            ],
            isError: true,
        };
    }
    try {
        // Forward to Tauri backend
        console.error(`[RalphX MCP] Calling Tauri: ${name} with args:`, JSON.stringify(args));
        let result;
        // Special handling for GET endpoints with path parameters
        if (name === "get_task_context") {
            const { task_id } = args;
            result = await callTauriGet(`task_context/${task_id}`);
        }
        else if (name === "get_artifact") {
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}`);
        }
        else if (name === "get_artifact_version") {
            const { artifact_id, version } = args;
            result = await callTauriGet(`artifact/${artifact_id}/version/${version}`);
        }
        else if (name === "get_related_artifacts") {
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}/related`);
        }
        else if (name === "get_plan_artifact") {
            // Also handle get_plan_artifact as GET
            const { artifact_id } = args;
            result = await callTauriGet(`get_plan_artifact/${artifact_id}`);
        }
        else if (name === "get_session_plan") {
            // Also handle get_session_plan as GET
            const { session_id } = args;
            result = await callTauriGet(`get_session_plan/${session_id}`);
        }
        else if (name === "get_task_steps") {
            // GET /api/task_steps/:task_id
            const { task_id } = args;
            result = await callTauriGet(`task_steps/${task_id}`);
        }
        else if (name === "get_step_progress") {
            // GET /api/step_progress/:task_id
            const { task_id } = args;
            result = await callTauriGet(`step_progress/${task_id}`);
        }
        else if (name === "get_review_notes") {
            // GET /api/review_notes/:task_id
            const { task_id } = args;
            result = await callTauriGet(`review_notes/${task_id}`);
        }
        else if (name === "list_session_proposals") {
            // GET /api/list_session_proposals/:session_id
            const { session_id } = args;
            result = await callTauriGet(`list_session_proposals/${session_id}`);
        }
        else if (name === "get_proposal") {
            // GET /api/proposal/:proposal_id
            const { proposal_id } = args;
            result = await callTauriGet(`proposal/${proposal_id}`);
        }
        else if (name === "analyze_session_dependencies") {
            // GET /api/analyze_dependencies/:session_id
            const { session_id } = args;
            result = await callTauriGet(`analyze_dependencies/${session_id}`);
        }
        else if (name === "complete_merge") {
            // POST /api/git/tasks/:task_id/complete-merge
            const { task_id, commit_sha } = args;
            result = await callTauri(`git/tasks/${task_id}/complete-merge`, { commit_sha });
        }
        else if (name === "report_conflict") {
            // POST /api/git/tasks/:task_id/report-conflict
            const { task_id, conflict_files, reason } = args;
            result = await callTauri(`git/tasks/${task_id}/report-conflict`, { conflict_files, reason });
        }
        else if (name === "report_incomplete") {
            // POST /api/git/tasks/:task_id/report-incomplete
            const { task_id, reason, diagnostic_info } = args;
            result = await callTauri(`git/tasks/${task_id}/report-incomplete`, { reason, diagnostic_info });
        }
        else if (name === "get_merge_target") {
            const { task_id } = args;
            result = await callTauriGet(`git/tasks/${task_id}/merge-target`);
        }
        else if (name === "get_task_issues") {
            // GET /api/task_issues/:task_id?status=<filter>
            const { task_id, status_filter } = args;
            const query = status_filter ? `?status=${status_filter}` : "";
            result = await callTauriGet(`task_issues/${task_id}${query}`);
        }
        else if (name === "get_issue_progress") {
            // GET /api/issue_progress/:task_id
            const { task_id } = args;
            result = await callTauriGet(`issue_progress/${task_id}`);
        }
        else if (name === "mark_issue_in_progress") {
            // POST /api/mark_issue_in_progress
            const { issue_id } = args;
            result = await callTauri("mark_issue_in_progress", { issue_id });
        }
        else if (name === "mark_issue_addressed") {
            // POST /api/mark_issue_addressed
            const { issue_id, resolution_notes, attempt_number } = args;
            result = await callTauri("mark_issue_addressed", { issue_id, resolution_notes, attempt_number });
        }
        else if (name === "get_project_analysis") {
            // GET /api/projects/:project_id/analysis?task_id=
            const { project_id, task_id } = args;
            const query = task_id ? `?task_id=${task_id}` : "";
            result = await callTauriGet(`projects/${project_id}/analysis${query}`);
        }
        else if (name === "save_project_analysis") {
            // POST /api/projects/:project_id/analysis
            const { project_id, entries } = args;
            result = await callTauri(`projects/${project_id}/analysis`, { entries });
        }
        else {
            // Default: POST request
            result = await callTauri(name, args || {});
        }
        console.error(`[RalphX MCP] Success: ${name}`);
        // Return result as JSON text
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(result, null, 2),
                },
            ],
        };
    }
    catch (error) {
        console.error(`[RalphX MCP] Error calling ${name}:`, error);
        if (error instanceof TauriClientError) {
            return {
                content: [
                    {
                        type: "text",
                        text: `ERROR: ${error.message}${error.details ? `\n\nDetails: ${error.details}` : ""}`,
                    },
                ],
                isError: true,
            };
        }
        return {
            content: [
                {
                    type: "text",
                    text: `ERROR: Unexpected error: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
            isError: true,
        };
    }
});
/**
 * Start the server
 */
async function main() {
    console.error("[RalphX MCP] Starting server...");
    console.error(`[RalphX MCP] Agent type: ${AGENT_TYPE}`);
    if (RALPHX_TASK_ID) {
        console.error(`[RalphX MCP] Task scope: ${RALPHX_TASK_ID}`);
    }
    if (RALPHX_PROJECT_ID) {
        console.error(`[RalphX MCP] Project scope: ${RALPHX_PROJECT_ID}`);
    }
    console.error(`[RalphX MCP] Tauri API URL: ${process.env.TAURI_API_URL || "http://127.0.0.1:3847"}`);
    // Log all tools if in debug mode or if RALPHX_DEBUG_TOOLS is set
    if (AGENT_TYPE === "debug" || process.env.RALPHX_DEBUG_TOOLS === "1") {
        logAllTools();
    }
    // Always log available tools for this agent
    const toolsByAgent = getToolsByAgent();
    const agentTools = toolsByAgent[AGENT_TYPE] || [];
    console.error(`[RalphX MCP] Tools for ${AGENT_TYPE}: ${agentTools.length > 0 ? agentTools.join(", ") : "(none - using filesystem tools)"}`);
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("[RalphX MCP] Server running on stdio");
}
main().catch((error) => {
    console.error("[RalphX MCP] Fatal error:", error);
    process.exit(1);
});
//# sourceMappingURL=index.js.map