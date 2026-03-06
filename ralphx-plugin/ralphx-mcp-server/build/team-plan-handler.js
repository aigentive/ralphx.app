/**
 * Handler for request_team_plan MCP tool
 *
 * Two-phase flow mirroring question-handler.ts:
 * 1. POST /api/team/plan/request — validates team, stores plan, returns plan_id immediately
 * 2. GET /api/team/plan/await/:plan_id — long-polls for user approval (15 min timeout)
 * 3. Returns approval result to agent as tool result
 *
 * Timeout staggering: backend timeout = 840s (14 min), client AbortController = 900,000ms (15 min).
 * Backend always fires first, returning a structured 408 response.
 */
const TAURI_API_URL = process.env.TAURI_API_URL || "http://127.0.0.1:3847";
/** Timeout for long-polling (15 minutes — staggered 1 min above backend's 14 min) */
const TEAM_PLAN_TIMEOUT_MS = 15 * 60 * 1000;
/**
 * Handle a request_team_plan tool call.
 *
 * Flow:
 * 1. Validate team_name and team registry existence
 * 2. Resolve lead_session_id from env or team config
 * 3. POST to /api/team/plan/request — registers plan, backend emits Tauri event
 * 4. GET /api/team/plan/await/:plan_id — blocks until user approves/rejects (15 min timeout)
 * 5. Return approval result to agent
 */
export async function handleRequestTeamPlan(args, contextType, contextId, leadSessionId) {
    const teamName = args.team_name;
    // Validate team_name is present
    if (!teamName) {
        return {
            content: [{
                    type: "text",
                    text: `ERROR: team_name is required for request_team_plan. Pass the exact team name from your TeamCreate call.`,
                }],
            isError: true,
        };
    }
    // Validate team exists in Claude Code's registry
    const os = await import("os");
    const fs = await import("fs");
    const path = await import("path");
    const configPath = path.join(os.homedir(), ".claude", "teams", teamName, "config.json");
    if (!fs.existsSync(configPath)) {
        return {
            content: [{
                    type: "text",
                    text: `ERROR: Team '${teamName}' not found in Claude Code registry at ${configPath}. Make sure you call TeamCreate with this exact team name before calling request_team_plan.`,
                }],
            isError: true,
        };
    }
    // Resolve lead_session_id: env var first, then team config fallback
    let resolvedLeadSessionId = leadSessionId;
    if (!resolvedLeadSessionId) {
        try {
            const configContent = JSON.parse(fs.readFileSync(configPath, "utf-8"));
            if (configContent.leadSessionId) {
                resolvedLeadSessionId = configContent.leadSessionId;
                console.error(`[RalphX MCP] lead_session_id resolved from team config: ${resolvedLeadSessionId}`);
            }
        }
        catch (e) {
            console.error(`[RalphX MCP] Warning: could not read team config for lead_session_id fallback: ${e}`);
        }
    }
    console.error(`[RalphX MCP] request_team_plan: lead_session_id=${resolvedLeadSessionId ?? "NULL"}, env_var=${leadSessionId ?? "NOT_SET"}, team=${teamName}, context_id=${contextId || "EMPTY"}`);
    // Phase 1: Register plan with Tauri backend
    let plan_id;
    try {
        const registerResponse = await fetch(`${TAURI_API_URL}/api/team/plan/request`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
                context_type: contextType || "ideation",
                context_id: contextId || "",
                process: args.process,
                teammates: args.teammates,
                team_name: teamName,
                lead_session_id: resolvedLeadSessionId ?? null,
            }),
        });
        if (!registerResponse.ok) {
            const errorText = await registerResponse.text().catch(() => registerResponse.statusText);
            throw new Error(`Failed to register team plan: ${errorText}`);
        }
        const result = (await registerResponse.json());
        plan_id = result.plan_id;
        console.error(`[RalphX MCP] Team plan registered: ${plan_id}`);
    }
    catch (error) {
        console.error(`[RalphX MCP] Failed to register team plan:`, error);
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify({
                        error: true,
                        message: `Failed to register team plan: ${error instanceof Error ? error.message : String(error)}`,
                    }),
                },
            ],
            isError: true,
        };
    }
    // Phase 2: Long-poll for user approval (15 min timeout)
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), TEAM_PLAN_TIMEOUT_MS);
    try {
        const awaitResponse = await fetch(`${TAURI_API_URL}/api/team/plan/await/${plan_id}`, {
            method: "GET",
            signal: controller.signal,
        });
        clearTimeout(timeoutId);
        if (!awaitResponse.ok) {
            if (awaitResponse.status === 408) {
                // Timeout from backend — structured response, not an error
                console.error(`[RalphX MCP] Team plan ${plan_id} timed out (backend)`);
                return {
                    content: [
                        {
                            type: "text",
                            text: JSON.stringify({
                                success: false,
                                reason: "timeout",
                                plan_id,
                                message: "Team plan approval timed out after 14 minutes. The user may be away. You can continue without approval or retry later.",
                            }),
                        },
                    ],
                };
            }
            const errorText = await awaitResponse.text().catch(() => awaitResponse.statusText);
            throw new Error(`Team plan await error: ${errorText}`);
        }
        const approvalResult = await awaitResponse.json();
        console.error(`[RalphX MCP] Team plan ${plan_id} resolved`);
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(approvalResult),
                },
            ],
        };
    }
    catch (error) {
        clearTimeout(timeoutId);
        if (error instanceof Error && error.name === "AbortError") {
            // Client-side timeout (safety net — backend should fire first)
            console.error(`[RalphX MCP] Team plan ${plan_id} timed out (client)`);
            return {
                content: [
                    {
                        type: "text",
                        text: JSON.stringify({
                            success: false,
                            reason: "timeout",
                            plan_id,
                            message: "Team plan approval timed out after 15 minutes. The user may be away. You can continue without approval or retry later.",
                        }),
                    },
                ],
            };
        }
        console.error(`[RalphX MCP] Team plan await error:`, error);
        throw error;
    }
}
//# sourceMappingURL=team-plan-handler.js.map