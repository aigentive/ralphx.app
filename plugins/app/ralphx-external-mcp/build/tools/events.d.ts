/**
 * Event monitoring tool handlers — Flow 4 (Phase 6)
 *
 * 4 tools for real-time monitoring, attention items, and capacity checks.
 * All tools perform project scope validation before forwarding to backend.
 */
export interface TaskCreatedEvent {
    event_type: 'task:created';
    task_id: string;
    project_id: string;
    title: string;
    timestamp: string;
}
export interface TaskStatusChangedEvent {
    event_type: 'task:status_changed';
    task_id: string;
    project_id: string;
    old_status: string;
    new_status: string;
    timestamp: string;
}
export interface TaskStepCompletedEvent {
    event_type: 'task:step_completed';
    task_id: string;
    project_id: string;
    step_id: string;
    step_title: string;
    timestamp: string;
}
export interface TaskExecutionStartedEvent {
    event_type: 'task:execution_started';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface TaskExecutionCompletedEvent {
    event_type: 'task:execution_completed';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface ReviewReadyEvent {
    event_type: 'review:ready';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface ReviewApprovedEvent {
    event_type: 'review:approved';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface ReviewChangesRequestedEvent {
    event_type: 'review:changes_requested';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface ReviewEscalatedEvent {
    event_type: 'review:escalated';
    task_id: string;
    project_id: string;
    reason?: string;
    timestamp: string;
}
export interface MergeReadyEvent {
    event_type: 'merge:ready';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface MergeCompletedEvent {
    event_type: 'merge:completed';
    task_id: string;
    project_id: string;
    timestamp: string;
}
export interface MergeConflictEvent {
    event_type: 'merge:conflict';
    task_id: string;
    project_id: string;
    source_branch: string;
    target_branch: string;
    conflict_files: string[];
    strategy: string;
    timestamp: string;
}
export interface IdeationSessionCreatedEvent {
    event_type: 'ideation:session_created';
    session_id: string;
    project_id: string;
    timestamp: string;
}
export interface IdeationPlanCreatedEvent {
    event_type: 'ideation:plan_created';
    session_id: string;
    project_id: string;
    timestamp: string;
}
export interface IdeationVerifiedEvent {
    event_type: 'ideation:verified';
    session_id: string;
    project_id: string;
    timestamp: string;
}
export interface IdeationProposalsReadyEvent {
    event_type: 'ideation:proposals_ready';
    session_id: string;
    project_id: string;
    proposal_count: number;
    timestamp: string;
}
export interface IdeationAutoProposeSentEvent {
    event_type: 'ideation:auto_propose_sent';
    session_id: string;
    project_id: string;
    timestamp: string;
}
export interface IdeationAutoProposeFailedEvent {
    event_type: 'ideation:auto_propose_failed';
    session_id: string;
    project_id: string;
    error: string;
    timestamp: string;
}
export interface IdeationSessionAcceptedEvent {
    event_type: 'ideation:session_accepted';
    session_id: string;
    project_id: string;
    timestamp: string;
}
export interface SystemWebhookUnhealthyEvent {
    event_type: 'system:webhook_unhealthy';
    webhook_id: string;
    project_id: string;
    failure_count: number;
    timestamp: string;
}
export interface SystemRateLimitWarningEvent {
    event_type: 'system:rate_limit_warning';
    project_id: string;
    api_key_id: string;
    timestamp: string;
}
export type RalphXEvent = TaskCreatedEvent | TaskStatusChangedEvent | TaskStepCompletedEvent | TaskExecutionStartedEvent | TaskExecutionCompletedEvent | ReviewReadyEvent | ReviewApprovedEvent | ReviewChangesRequestedEvent | ReviewEscalatedEvent | MergeReadyEvent | MergeCompletedEvent | MergeConflictEvent | IdeationSessionCreatedEvent | IdeationPlanCreatedEvent | IdeationVerifiedEvent | IdeationProposalsReadyEvent | IdeationAutoProposeSentEvent | IdeationAutoProposeFailedEvent | IdeationSessionAcceptedEvent | SystemWebhookUnhealthyEvent | SystemRateLimitWarningEvent;
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
/**
 * v1_register_webhook — register a webhook URL for real-time event delivery.
 * POST /api/external/webhooks/register
 *
 * Returns the webhook registration including the HMAC secret (shown ONCE — store it).
 * Idempotent: re-registering the same URL returns the existing registration.
 */
export declare function handleRegisterWebhook(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_unregister_webhook — remove a webhook registration.
 * DELETE /api/external/webhooks/:id
 */
export declare function handleUnregisterWebhook(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_list_webhooks — list all registered webhooks for this API key.
 * GET /api/external/webhooks
 */
export declare function handleListWebhooks(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_webhook_health — check delivery health for all registered webhooks.
 * GET /api/external/webhooks/health
 *
 * Returns per-webhook stats: active status, failure count, and last failure time.
 * Use this to detect broken webhooks before relying on event delivery.
 */
export declare function handleGetWebhookHealth(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
//# sourceMappingURL=events.d.ts.map