/**
 * Handler for ask_user_question MCP tool
 *
 * Mirrors the permission_request pattern:
 * 1. POST /api/question/request — registers question, emits Tauri event
 * 2. GET /api/question/await/:request_id — long-polls for user answer (5 min timeout)
 * 3. Returns answer to agent as tool result
 */
const TAURI_API_URL = process.env.TAURI_API_URL || "http://127.0.0.1:3847";
/** Timeout for long-polling (5 minutes) */
const QUESTION_TIMEOUT_MS = 5 * 60 * 1000;
/**
 * Handle an ask_user_question tool call.
 *
 * Flow:
 * 1. POST to /api/question/request — registers the question, backend emits Tauri event
 * 2. GET /api/question/await/:request_id — blocks until user answers (5 min timeout)
 * 3. Return the answer JSON to the agent
 */
export async function handleAskUserQuestion(args) {
    console.error(`[RalphX MCP] ask_user_question for session: ${args.session_id}`);
    // 1. Register question with Tauri backend
    let request_id;
    try {
        const registerResponse = await fetch(`${TAURI_API_URL}/api/question/request`, {
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
        console.error(`[RalphX MCP] Question registered: ${request_id}`);
    }
    catch (error) {
        console.error(`[RalphX MCP] Failed to register question:`, error);
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
    // 2. Long-poll for user answer (5 minute timeout)
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), QUESTION_TIMEOUT_MS);
    try {
        const answerResponse = await fetch(`${TAURI_API_URL}/api/question/await/${request_id}`, {
            method: "GET",
            signal: controller.signal,
        });
        clearTimeout(timeoutId);
        if (!answerResponse.ok) {
            if (answerResponse.status === 408) {
                // Timeout from backend
                console.error(`[RalphX MCP] Question ${request_id} timed out (backend)`);
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
        console.error(`[RalphX MCP] Question ${request_id} answered`);
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
        if (error instanceof Error && error.name === "AbortError") {
            console.error(`[RalphX MCP] Question ${request_id} timed out (client)`);
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
        console.error(`[RalphX MCP] Question await error:`, error);
        throw error;
    }
}
//# sourceMappingURL=question-handler.js.map