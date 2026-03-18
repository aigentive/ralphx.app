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
import { safeError } from "./redact.js";
import { getFilteredTools, isToolAllowed, getAllowedToolNames, parseAllowedToolsFromArgs, logAllTools, getToolsByAgent, setAgentType, } from "./tools.js";
import { permissionRequestTool, handlePermissionRequest, } from "./permission-handler.js";
import { handleAskUserQuestion } from "./question-handler.js";
import { handleRequestTeamPlan } from "./team-plan-handler.js";
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
    safeError(`[RalphX MCP] Agent type from CLI args: ${AGENT_TYPE}`);
}
else if (process.env.RALPHX_AGENT_TYPE) {
    safeError(`[RalphX MCP] Agent type from env: ${AGENT_TYPE}`);
}
else {
    safeError(`[RalphX MCP] Agent type unknown (no CLI arg or env var)`);
}
// Task ID from environment (for task-level scoping enforcement)
const RALPHX_TASK_ID = process.env.RALPHX_TASK_ID;
// Project ID from environment (for project-level scoping enforcement)
const RALPHX_PROJECT_ID = process.env.RALPHX_PROJECT_ID;
// Context type and ID from environment (set by chat_service_context for all agent spawns)
const RALPHX_CONTEXT_TYPE = process.env.RALPHX_CONTEXT_TYPE;
const RALPHX_CONTEXT_ID = process.env.RALPHX_CONTEXT_ID;
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
        "report_conflict",
        "report_incomplete",
        "complete_merge",
        "get_merge_target",
        // Issue tools (worker + reviewer agents)
        "get_task_issues",
        "get_issue_progress",
        // Execution complete (worker agent)
        "execution_complete",
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
        // Memory write tools (memory agents only)
        // Note: mark_memory_obsolete excluded - uses memory_id lookup for implicit project validation
        "upsert_memories",
        "refresh_memory_rule_index",
        "ingest_rule_file",
        "rebuild_archive_snapshots",
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
    // Parse once — reuse for logging and to avoid a redundant argv scan inside getAllowedToolNames()
    const cliToolsArg = parseAllowedToolsFromArgs();
    const tools = getFilteredTools();
    // Always include permission_request tool (not scoped by agent type)
    const allTools = [...tools, permissionRequestTool];
    // Log tool scoping for debugging
    if (cliToolsArg !== undefined) {
        safeError(`[RalphX MCP] Tools from --allowed-tools: ${cliToolsArg.length > 0 ? cliToolsArg.join(", ") : "none (explicit __NONE__)"}`);
    }
    const toolNames = tools.map((t) => t.name);
    safeError(`[RalphX MCP] Agent type: ${AGENT_TYPE}, Tools: ${toolNames.length > 0 ? toolNames.join(", ") : "none"} + permission_request`);
    return { tools: allTools };
});
/**
 * Execute tool calls (with authorization check)
 */
server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    // Special handling for permission_request tool (always allowed, not scoped by agent type)
    if (name === "permission_request") {
        try {
            return await handlePermissionRequest(args);
        }
        catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: JSON.stringify({ behavior: "deny", message }) }],
            };
        }
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
        try {
            return await handleAskUserQuestion(args);
        }
        catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: `ERROR: Unexpected error: ${message}` }],
                isError: true,
            };
        }
    }
    // Special handling for request_team_plan (two-phase: register POST + long-poll GET)
    if (name === "request_team_plan") {
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
        const leadSessionId = globalThis.process.env.RALPHX_LEAD_SESSION_ID;
        try {
            return await handleRequestTeamPlan(args, RALPHX_CONTEXT_TYPE ?? "ideation", RALPHX_CONTEXT_ID ?? "", leadSessionId);
        }
        catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            return {
                content: [{ type: "text", text: `ERROR: Unexpected error: ${message}` }],
                isError: true,
            };
        }
    }
    // Authorization check (defense in depth)
    if (!isToolAllowed(name)) {
        const allowedNames = getAllowedToolNames();
        const errorMessage = allowedNames.length > 0
            ? `Tool "${name}" is not available for agent type "${AGENT_TYPE}". Allowed tools: ${allowedNames.join(", ")}`
            : `Agent type "${AGENT_TYPE}" has no MCP tools available. This agent should use filesystem tools (Read, Grep, Glob, Bash, Edit, Write) instead.`;
        safeError(`[RalphX MCP] Unauthorized tool call: ${name}`);
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
        safeError(`[RalphX MCP] Task scope violation: ${name}`);
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
        safeError(`[RalphX MCP] Project scope violation: ${name}`);
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
        safeError(`[RalphX MCP] Calling Tauri: ${name} with args:`, JSON.stringify(args));
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
            // DEPRECATED: alias for backward compat — routes to get_artifact handler
            const { artifact_id } = args;
            result = await callTauriGet(`artifact/${artifact_id}`);
        }
        else if (name === "get_session_plan") {
            // Also handle get_session_plan as GET
            const { session_id } = args;
            result = await callTauriGet(`get_session_plan/${session_id}`);
        }
        else if (name === "get_plan_verification") {
            // GET /api/ideation/sessions/:id/verification
            const { session_id } = args;
            result = await callTauriGet(`ideation/sessions/${session_id}/verification`);
        }
        else if (name === "update_plan_verification") {
            // POST /api/ideation/sessions/:id/verification
            const { session_id, ...body } = args;
            result = await callTauri(`ideation/sessions/${session_id}/verification`, body);
        }
        else if (name === "revert_and_skip") {
            // POST /api/ideation/sessions/:id/revert-and-skip
            const { session_id, plan_version_to_restore } = args;
            result = await callTauri(`ideation/sessions/${session_id}/revert-and-skip`, { plan_version_to_restore });
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
        else if (name === "get_step_context") {
            // GET /api/step_context/:step_id
            const { step_id } = args;
            result = await callTauriGet(`step_context/${step_id}`);
        }
        else if (name === "get_sub_steps") {
            // GET /api/sub_steps/:parent_step_id
            const { parent_step_id } = args;
            result = await callTauriGet(`sub_steps/${parent_step_id}`);
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
        else if (name === "create_child_session") {
            // POST /api/create_child_session
            const { parent_session_id, title, description, inherit_context, initial_prompt, team_mode, team_config, purpose } = args;
            result = await callTauri("create_child_session", { parent_session_id, title, description, inherit_context, initial_prompt, team_mode, team_config, purpose });
        }
        else if (name === "get_parent_session_context") {
            // GET /api/parent_session_context/:session_id
            const { session_id } = args;
            result = await callTauriGet(`parent_session_context/${session_id}`);
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
        else if (name === "request_teammate_spawn") {
            // POST /api/team/spawn
            const { role, prompt, model, tools, mcp_tools, preset } = args;
            result = await callTauri("team/spawn", { role, prompt, model, tools, mcp_tools, preset });
        }
        else if (name === "create_team_artifact") {
            // POST /api/team/artifact
            const { session_id, title, content, artifact_type, related_artifact_id } = args;
            result = await callTauri("team/artifact", {
                session_id,
                title,
                content,
                artifact_type,
                related_artifact_id,
            });
        }
        else if (name === "get_team_artifacts") {
            // GET /api/team/artifacts/:session_id
            const { session_id } = args;
            result = await callTauriGet(`team/artifacts/${session_id}`);
        }
        else if (name === "get_team_session_state") {
            // GET /api/team/session_state/:session_id
            const { session_id } = args;
            result = await callTauriGet(`team/session_state/${session_id}`);
        }
        else if (name === "save_team_session_state") {
            // POST /api/team/session_state
            const { session_id, team_composition, phase, artifact_ids } = args;
            result = await callTauri("team/session_state", {
                session_id,
                team_composition,
                phase,
                artifact_ids,
            });
        }
        else if (name === "execution_complete") {
            // POST /api/execution/tasks/:task_id/complete
            const { task_id, summary } = args;
            result = await callTauri(`execution/tasks/${task_id}/complete`, { summary: summary || "" });
        }
        else if (name === "list_projects") {
            // GET /api/internal/projects
            result = await callTauriGet("internal/projects");
        }
        else if (name === "create_cross_project_session") {
            // POST /api/internal/cross_project/create_session
            const { target_project_path, source_session_id, title } = args;
            result = await callTauri("internal/cross_project/create_session", {
                targetProjectPath: target_project_path,
                sourceSessionId: source_session_id,
                title,
            });
        }
        else if (name === "cross_project_guide") {
            // MCP-server-only: analyze plan for cross-project paths and return guidance
            const { session_id, plan_content } = args;
            let planText = plan_content ?? "";
            // If session_id provided, fetch plan content via get_session_plan
            if (session_id && !planText) {
                try {
                    const planData = await callTauriGet(`get_session_plan/${session_id}`);
                    planText = planData?.content ?? "";
                }
                catch (err) {
                    planText = "";
                    safeError(`[RalphX MCP] cross_project_guide: failed to fetch plan for session ${session_id}:`, err);
                }
            }
            // Heuristic: detect cross-project paths (absolute paths, ../relative paths, paths with project-like names)
            const crossProjectPatterns = [
                /(?:^|\s|["'`])(\/(home|Users|workspace|projects|srv|opt)\/[^\s"'`]+)/gm,
                /(?:^|\s|["'`])(\.\.\/?[^\s"'`]+)/gm,
                /(?:target[_-]?project[_-]?path|project[_-]?path|working[_-]?directory)[:\s]+["']?([^\s"'`,\n]+)/gim,
            ];
            const detectedPaths = [];
            for (const pattern of crossProjectPatterns) {
                const matches = [...planText.matchAll(pattern)];
                for (const m of matches) {
                    const p = (m[1] || m[0]).trim().replace(/^["'`]|["'`]$/g, "");
                    if (p && !detectedPaths.includes(p)) {
                        detectedPaths.push(p);
                    }
                }
            }
            const hasCrossProjectContent = detectedPaths.length > 0 ||
                /cross[- ]?project|multi[- ]?project|target project|another project|different project|project[_ ]?b\b/i.test(planText);
            result = {
                has_cross_project_paths: hasCrossProjectContent,
                detected_paths: detectedPaths,
                guidance: hasCrossProjectContent
                    ? {
                        summary: "This plan contains cross-project references. Follow these steps to orchestrate multi-project execution:",
                        steps: [
                            "1. Call list_projects to discover existing RalphX projects and their filesystem paths.",
                            "2. For each target project, call create_cross_project_session({ target_project_path, source_session_id }) to create a new session with the inherited plan.",
                            "3. In each target session, use create_task_proposal to create proposals specific to that project's scope.",
                            "4. Call accept_plan_and_schedule (or equivalent) in each target session to push tasks to kanban.",
                        ],
                        notes: [
                            "The target project is auto-created if no RalphX project exists at the given path.",
                            "The inherited plan is read-only in the target session. Call create_plan_artifact to create a writable copy if modifications are needed.",
                            "The inherited plan status is set to 'imported_verified' — no re-verification is triggered.",
                        ],
                        detected_paths: detectedPaths,
                    }
                    : {
                        summary: "No cross-project paths detected in this plan.",
                        steps: [],
                        notes: [
                            "If you believe there are cross-project references, try providing the plan_content directly or check the session_id.",
                        ],
                        detected_paths: [],
                    },
            };
        }
        else if (name === "get_child_session_status") {
            // GET /api/ideation/sessions/:id/child-status
            const { session_id, include_recent_messages, message_limit } = args;
            const params = new URLSearchParams();
            if (include_recent_messages)
                params.set("include_messages", "true");
            if (message_limit)
                params.set("message_limit", String(message_limit));
            const query = params.toString() ? `?${params}` : "";
            result = await callTauriGet(`ideation/sessions/${session_id}/child-status${query}`);
        }
        else if (name === "send_child_session_message") {
            // POST /api/ideation/sessions/:id/message
            const { session_id, message } = args;
            result = await callTauri(`ideation/sessions/${session_id}/message`, { message });
        }
        else if (name === "delete_task_proposal") {
            // Alias for archive_task_proposal — no /api/delete_task_proposal route exists in backend
            const { proposal_id } = args;
            result = await callTauri("archive_task_proposal", { proposal_id });
        }
        else {
            // Default: POST request
            result = await callTauri(name, args || {});
        }
        safeError(`[RalphX MCP] Success: ${name}`);
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
        safeError(`[RalphX MCP] Error calling ${name}:`, error);
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
    safeError(`[RalphX MCP] Agent type: ${AGENT_TYPE}`);
    if (RALPHX_TASK_ID) {
        safeError(`[RalphX MCP] Task scope: ${RALPHX_TASK_ID}`);
    }
    if (RALPHX_PROJECT_ID) {
        safeError(`[RalphX MCP] Project scope: ${RALPHX_PROJECT_ID}`);
    }
    safeError(`[RalphX MCP] Tauri API URL: ${process.env.TAURI_API_URL || "http://127.0.0.1:3847"}`);
    // Log all tools if in debug mode or if RALPHX_DEBUG_TOOLS is set
    if (AGENT_TYPE === "debug" || process.env.RALPHX_DEBUG_TOOLS === "1") {
        logAllTools();
    }
    // Always log available tools for this agent
    const toolsByAgent = getToolsByAgent();
    const agentTools = toolsByAgent[AGENT_TYPE] || [];
    safeError(`[RalphX MCP] Tools for ${AGENT_TYPE}: ${agentTools.length > 0 ? agentTools.join(", ") : "(none - using filesystem tools)"}`);
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error("[RalphX MCP] Server running on stdio");
}
// Global handler for unhandled promise rejections.
// Prevents secrets in HTTP error bodies or rejected promises from leaking via Node's default stderr handler.
process.on("unhandledRejection", (reason) => {
    safeError("[RalphX MCP] Unhandled rejection:", reason);
});
main().catch((error) => {
    safeError("[RalphX MCP] Fatal error:", error);
    process.exit(1);
});
//# sourceMappingURL=index.js.map