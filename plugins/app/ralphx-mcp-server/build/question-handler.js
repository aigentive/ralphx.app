/**
 * Handler for ask_user_question MCP tool
 *
 * Mirrors the permission_request pattern:
 * 1. POST /api/question/request — registers question, emits Tauri event
 * 2. GET /api/question/await/:request_id — long-polls for user answer (about 5 min timeout)
 * 3. Returns answer to agent as tool result
 */
import { createHumanWaitAbortController, HUMAN_WAIT_CLIENT_TIMEOUT_MS, isHumanWaitTimeoutError, } from "./human-wait.js";
import { safeError } from "./redact.js";
import { buildTauriApiUrl } from "./tauri-client.js";
/**
 * Handle an ask_user_question tool call.
 *
 * Flow:
 * 1. POST to /api/question/request — registers the question, backend emits Tauri event
 * 2. GET /api/question/await/:request_id — blocks until user answers (about 5 min timeout)
 * 3. Return the answer JSON to the agent
 */
export async function handleAskUserQuestion(args) {
    safeError(`[RalphX MCP] ask_user_question for session: ${args.session_id}`);
    // 1. Register question with Tauri backend
    let request_id;
    try {
        const registerResponse = await fetch(buildTauriApiUrl("question/request"), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
                session_id: args.session_id,
                question: args.question,
                header: args.header,
                options: (args.options ?? []).map((o) => ({
                    value: o.value ?? o.label,
                    label: o.label,
                    description: o.description,
                })),
                multi_select: args.multi_select ?? false,
            }),
        });
        if (!registerResponse.ok) {
            const errorText = await registerResponse.text().catch(() => registerResponse.statusText);
            throw new Error(`Failed to register question: ${errorText}`);
        }
        const result = (await registerResponse.json());
        request_id = result.request_id;
        safeError(`[RalphX MCP] Question registered: ${request_id}`);
    }
    catch (error) {
        safeError(`[RalphX MCP] Failed to register question:`, error);
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify({
                        error: true,
                        message: `Failed to register question: ${error instanceof Error ? error.message : String(error)}`,
                    }),
                },
            ],
        };
    }
    // 2. Long-poll for user answer. Keep our timeout just below the effective
    // MCP tool ceiling so this path returns structured timeout JSON instead of
    // surfacing a raw transport error back to the agent.
    const { controller, timeoutId } = createHumanWaitAbortController();
    const waitStartedAt = Date.now();
    try {
        const answerResponse = await fetch(buildTauriApiUrl(`question/await/${encodeURIComponent(request_id)}`), {
            method: "GET",
            signal: controller.signal,
        });
        clearTimeout(timeoutId);
        if (!answerResponse.ok) {
            if (answerResponse.status === 408) {
                // Timeout from backend
                safeError(`[RalphX MCP] Question ${request_id} timed out (backend)`);
                return {
                    content: [
                        {
                            type: "text",
                            text: JSON.stringify({
                                error: true,
                                message: "Question timed out waiting for user response. The user may be away. You can continue without the answer or try asking again later.",
                            }),
                        },
                    ],
                };
            }
            const errorText = await answerResponse.text().catch(() => answerResponse.statusText);
            throw new Error(`Question await error: ${errorText}`);
        }
        const answer = (await answerResponse.json());
        safeError(`[RalphX MCP] Question ${request_id} answered`);
        return {
            content: [
                {
                    type: "text",
                    text: JSON.stringify(answer),
                },
            ],
        };
    }
    catch (error) {
        clearTimeout(timeoutId);
        const elapsedMs = Date.now() - waitStartedAt;
        if (isHumanWaitTimeoutError(error, elapsedMs, HUMAN_WAIT_CLIENT_TIMEOUT_MS)) {
            safeError(`[RalphX MCP] Question ${request_id} timed out (client/transport)`);
            return {
                content: [
                    {
                        type: "text",
                        text: JSON.stringify({
                            error: true,
                            message: "Question timed out waiting for user response. The user may be away. You can continue without the answer or try asking again later.",
                        }),
                    },
                ],
            };
        }
        safeError(`[RalphX MCP] Question await error:`, error);
        throw error;
    }
}
//# sourceMappingURL=question-handler.js.map