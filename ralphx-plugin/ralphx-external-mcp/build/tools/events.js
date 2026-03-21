/**
 * Event monitoring tool handlers — Flow 4 (Phase 6)
 *
 * 4 tools for real-time monitoring, attention items, and capacity checks.
 * All tools perform project scope validation before forwarding to backend.
 */
// ─────────────────────────────────────────────────────────────────────────────
import { getBackendClient, BackendError } from "../backend-client.js";
function handleError(err) {
    if (err instanceof BackendError) {
        // Gracefully handle not-yet-implemented backend endpoints
        if (err.statusCode === 404 || err.statusCode === 501) {
            return JSON.stringify({
                error: "endpoint_not_available",
                status: err.statusCode,
                message: err.message,
            }, null, 2);
        }
        return JSON.stringify({ error: "backend_error", status: err.statusCode, message: err.message }, null, 2);
    }
    return JSON.stringify({ error: "unexpected_error", message: String(err) }, null, 2);
}
function checkProjectScope(projectId, context) {
    if (context.projectIds.length > 0 && !context.projectIds.includes(projectId)) {
        return JSON.stringify({
            error: "scope_violation",
            message: `Project '${projectId}' is not in this API key's scope.`,
        }, null, 2);
    }
    return null;
}
/**
 * v1_get_recent_events — cursor-based event retrieval from DB.
 * GET /api/external/events/poll?project_id={id}&cursor={cursor}&limit={limit}
 *
 * The cursor is the last event's `id` (integer autoincrement). Pass 0 or omit to
 * start from recent events.
 */
export async function handleGetRecentEvents(args, context) {
    const projectId = args.project_id;
    const cursor = typeof args.cursor === "number" ? args.cursor : 0;
    const rawLimit = typeof args.limit === "number" ? args.limit : 50;
    const limit = Math.min(Math.max(1, rawLimit), 200);
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    const scopeError = checkProjectScope(projectId, context);
    if (scopeError)
        return scopeError;
    try {
        const params = {
            project_id: projectId,
            limit: String(limit),
        };
        if (cursor > 0) {
            params.cursor = String(cursor);
        }
        const response = await getBackendClient().get("/api/external/events/poll", context, params);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_subscribe_events — fetch recent events and return a polling hint.
 *
 * Real SSE streaming is not possible within a single MCP tool call. This tool
 * fetches the most recent batch of events and returns a subscription_hint
 * telling the caller how to poll with v1_get_recent_events.
 *
 * GET /api/external/events/poll?project_id={id}&limit=20
 */
export async function handleSubscribeEvents(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    const scopeError = checkProjectScope(projectId, context);
    if (scopeError)
        return scopeError;
    try {
        const response = await getBackendClient().get("/api/external/events/poll", context, { project_id: projectId, limit: "20" });
        const data = response.body;
        const events = data.events ?? [];
        const nextCursor = data.next_cursor ?? (events.length > 0 ? events[events.length - 1].id : 0);
        return JSON.stringify({
            events,
            next_cursor: nextCursor,
            subscription_hint: "MCP tools are request/response only — real-time streaming is not supported. " +
                "To poll for new events, call v1_get_recent_events with { project_id, cursor: <next_cursor> } " +
                "repeatedly. The cursor is the last event id you received; pass it back to get only newer events.",
        }, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_get_attention_items — get tasks needing human attention.
 * GET /api/external/attention/:project_id
 *
 * Returns escalated reviews, failed tasks, and merge conflicts.
 * Returns empty arrays gracefully if backend endpoint is not yet available.
 */
export async function handleGetAttentionItems(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    const scopeError = checkProjectScope(projectId, context);
    if (scopeError)
        return scopeError;
    try {
        const response = await getBackendClient().get(`/api/external/attention/${encodeURIComponent(projectId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        if (err instanceof BackendError && (err.statusCode === 404 || err.statusCode === 501)) {
            // Backend endpoint not yet available — return empty state with note
            return JSON.stringify({
                escalated_reviews: [],
                failed_tasks: [],
                merge_conflicts: [],
                note: "Attention items endpoint not yet available on this backend version.",
            }, null, 2);
        }
        return handleError(err);
    }
}
/**
 * v1_get_execution_capacity — check if new tasks can be started.
 * GET /api/external/execution_capacity/:project_id
 *
 * Returns capacity info. Returns default values gracefully if backend
 * endpoint is not yet available.
 */
export async function handleGetExecutionCapacity(args, context) {
    const projectId = args.project_id;
    if (!projectId) {
        return JSON.stringify({ error: "missing_argument", message: "project_id is required" }, null, 2);
    }
    const scopeError = checkProjectScope(projectId, context);
    if (scopeError)
        return scopeError;
    try {
        const response = await getBackendClient().get(`/api/external/execution_capacity/${encodeURIComponent(projectId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        if (err instanceof BackendError && (err.statusCode === 404 || err.statusCode === 501)) {
            // Backend endpoint not yet available — return default capacity with note
            return JSON.stringify({
                can_start: true,
                project_running: 0,
                project_queued: 0,
                note: "Execution capacity endpoint not yet available on this backend version.",
            }, null, 2);
        }
        return handleError(err);
    }
}
/**
 * v1_register_webhook — register a webhook URL for real-time event delivery.
 * POST /api/external/webhooks/register
 *
 * Returns the webhook registration including the HMAC secret (shown ONCE — store it).
 * Idempotent: re-registering the same URL returns the existing registration.
 */
export async function handleRegisterWebhook(args, context) {
    const url = args.url;
    if (!url) {
        return JSON.stringify({ error: "missing_argument", message: "url is required" }, null, 2);
    }
    const eventTypes = Array.isArray(args.event_types)
        ? args.event_types
        : undefined;
    const projectIds = Array.isArray(args.project_ids)
        ? args.project_ids
        : [];
    // Project scope validation for explicit project_ids
    if (projectIds.length > 0) {
        for (const pid of projectIds) {
            const scopeError = checkProjectScope(pid, context);
            if (scopeError)
                return scopeError;
        }
    }
    try {
        const response = await getBackendClient().post("/api/external/webhooks/register", context, { url, event_types: eventTypes, project_ids: projectIds });
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_unregister_webhook — remove a webhook registration.
 * DELETE /api/external/webhooks/:id
 */
export async function handleUnregisterWebhook(args, context) {
    const webhookId = args.webhook_id;
    if (!webhookId) {
        return JSON.stringify({ error: "missing_argument", message: "webhook_id is required" }, null, 2);
    }
    try {
        const response = await getBackendClient().delete(`/api/external/webhooks/${encodeURIComponent(webhookId)}`, context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_list_webhooks — list all registered webhooks for this API key.
 * GET /api/external/webhooks
 */
export async function handleListWebhooks(args, context) {
    try {
        const response = await getBackendClient().get("/api/external/webhooks", context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
/**
 * v1_get_webhook_health — check delivery health for all registered webhooks.
 * GET /api/external/webhooks/health
 *
 * Returns per-webhook stats: active status, failure count, and last failure time.
 * Use this to detect broken webhooks before relying on event delivery.
 */
export async function handleGetWebhookHealth(args, context) {
    try {
        const response = await getBackendClient().get("/api/external/webhooks/health", context);
        return JSON.stringify(response.body, null, 2);
    }
    catch (err) {
        return handleError(err);
    }
}
//# sourceMappingURL=events.js.map