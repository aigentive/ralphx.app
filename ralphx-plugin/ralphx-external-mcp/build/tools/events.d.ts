/**
 * Event monitoring tool handlers — Flow 4 (Phase 6)
 *
 * 4 tools for real-time monitoring, attention items, and capacity checks.
 * All tools perform project scope validation before forwarding to backend.
 */
import type { ApiKeyContext } from "../types.js";
/**
 * v1_get_recent_events — cursor-based event retrieval from DB.
 * GET /api/external/events/poll?project_id={id}&cursor={cursor}&limit={limit}
 *
 * The cursor is the last event's `id` (integer autoincrement). Pass 0 or omit to
 * start from recent events.
 */
export declare function handleGetRecentEvents(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_subscribe_events — fetch recent events and return a polling hint.
 *
 * Real SSE streaming is not possible within a single MCP tool call. This tool
 * fetches the most recent batch of events and returns a subscription_hint
 * telling the caller how to poll with v1_get_recent_events.
 *
 * GET /api/external/events/poll?project_id={id}&limit=20
 */
export declare function handleSubscribeEvents(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_attention_items — get tasks needing human attention.
 * GET /api/external/attention/:project_id
 *
 * Returns escalated reviews, failed tasks, and merge conflicts.
 * Returns empty arrays gracefully if backend endpoint is not yet available.
 */
export declare function handleGetAttentionItems(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_execution_capacity — check if new tasks can be started.
 * GET /api/external/execution_capacity/:project_id
 *
 * Returns capacity info. Returns default values gracefully if backend
 * endpoint is not yet available.
 */
export declare function handleGetExecutionCapacity(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
//# sourceMappingURL=events.d.ts.map