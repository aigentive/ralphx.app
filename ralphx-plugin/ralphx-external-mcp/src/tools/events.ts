/**
 * Event monitoring tool handlers — Flow 4 (Phase 6)
 *
 * 4 tools for real-time monitoring, attention items, and capacity checks.
 * All tools perform project scope validation before forwarding to backend.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Event Type Contracts — discriminated union interfaces for all pipeline events
// ─────────────────────────────────────────────────────────────────────────────

// Task events
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

// Review events
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

// Merge events
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

// Ideation events
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

// System events
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

// Union type covering all events
export type RalphXEvent =
  | TaskCreatedEvent
  | TaskStatusChangedEvent
  | TaskStepCompletedEvent
  | TaskExecutionStartedEvent
  | TaskExecutionCompletedEvent
  | ReviewReadyEvent
  | ReviewApprovedEvent
  | ReviewChangesRequestedEvent
  | ReviewEscalatedEvent
  | MergeReadyEvent
  | MergeCompletedEvent
  | MergeConflictEvent
  | IdeationSessionCreatedEvent
  | IdeationPlanCreatedEvent
  | IdeationVerifiedEvent
  | IdeationProposalsReadyEvent
  | IdeationAutoProposeSentEvent
  | IdeationAutoProposeFailedEvent
  | IdeationSessionAcceptedEvent
  | SystemWebhookUnhealthyEvent
  | SystemRateLimitWarningEvent;

// ─────────────────────────────────────────────────────────────────────────────

import { getBackendClient, BackendError } from "../backend-client.js";
import type { ApiKeyContext } from "../types.js";
import type { ExternalEvent } from "../events/types.js";

function handleError(err: unknown): string {
  if (err instanceof BackendError) {
    // Gracefully handle not-yet-implemented backend endpoints
    if (err.statusCode === 404 || err.statusCode === 501) {
      return JSON.stringify(
        {
          error: "endpoint_not_available",
          status: err.statusCode,
          message: err.message,
        },
        null,
        2
      );
    }
    return JSON.stringify(
      { error: "backend_error", status: err.statusCode, message: err.message },
      null,
      2
    );
  }
  return JSON.stringify(
    { error: "unexpected_error", message: String(err) },
    null,
    2
  );
}

function checkProjectScope(projectId: string, context: ApiKeyContext): string | null {
  if (context.projectIds.length > 0 && !context.projectIds.includes(projectId)) {
    return JSON.stringify(
      {
        error: "scope_violation",
        message: `Project '${projectId}' is not in this API key's scope.`,
      },
      null,
      2
    );
  }
  return null;
}

interface EventPollResponse {
  events: ExternalEvent[];
  next_cursor: number | null;
  has_more: boolean;
}

/**
 * v1_get_recent_events — cursor-based event retrieval from DB.
 * GET /api/external/events/poll?project_id={id}&cursor={cursor}&limit={limit}
 *
 * The cursor is the last event's `id` (integer autoincrement). Pass 0 or omit to
 * start from recent events.
 */
export async function handleGetRecentEvents(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const projectId = args.project_id as string | undefined;
  const cursorRaw = typeof args.cursor === "number" ? args.cursor : typeof args.last_id === "number" ? args.last_id : 0;
  const cursor = cursorRaw;
  const rawLimit = typeof args.limit === "number" ? args.limit : 50;
  const limit = Math.min(Math.max(1, rawLimit), 200);
  const eventType = typeof args.event_type === "string" ? args.event_type : undefined;

  if (!projectId) {
    return JSON.stringify(
      { error: "missing_argument", message: "project_id is required" },
      null,
      2
    );
  }

  const scopeError = checkProjectScope(projectId, context);
  if (scopeError) return scopeError;

  try {
    const params: Record<string, string> = {
      project_id: projectId,
      limit: String(limit),
    };
    if (cursor > 0) {
      params.cursor = String(cursor);
    }
    if (eventType !== undefined) {
      params.event_type = eventType;
    }

    const response = await getBackendClient().get<EventPollResponse>(
      "/api/external/events/poll",
      context,
      params
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
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
export async function handleSubscribeEvents(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const projectId = args.project_id as string | undefined;

  if (!projectId) {
    return JSON.stringify(
      { error: "missing_argument", message: "project_id is required" },
      null,
      2
    );
  }

  const scopeError = checkProjectScope(projectId, context);
  if (scopeError) return scopeError;

  try {
    const response = await getBackendClient().get<EventPollResponse>(
      "/api/external/events/poll",
      context,
      { project_id: projectId, limit: "20" }
    );

    const data = response.body;
    const events: ExternalEvent[] = data.events ?? [];
    const nextCursor =
      data.next_cursor ?? (events.length > 0 ? events[events.length - 1]!.id : 0);

    return JSON.stringify(
      {
        events,
        next_cursor: nextCursor,
        subscription_hint:
          "MCP tools are request/response only — real-time streaming is not supported. " +
          "To poll for new events, call v1_get_recent_events with { project_id, cursor: <next_cursor> } " +
          "repeatedly. The cursor is the last event id you received; pass it back to get only newer events.",
      },
      null,
      2
    );
  } catch (err) {
    return handleError(err);
  }
}

interface AttentionItem {
  task_id: string;
  title: string;
  status: string;
  updated_at: string;
}

interface AttentionResponse {
  escalated_reviews: AttentionItem[];
  failed_tasks: AttentionItem[];
  merge_conflicts: AttentionItem[];
}

/**
 * v1_get_attention_items — get tasks needing human attention.
 * GET /api/external/attention/:project_id
 *
 * Returns escalated reviews, failed tasks, and merge conflicts.
 * Returns empty arrays gracefully if backend endpoint is not yet available.
 */
export async function handleGetAttentionItems(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const projectId = args.project_id as string | undefined;

  if (!projectId) {
    return JSON.stringify(
      { error: "missing_argument", message: "project_id is required" },
      null,
      2
    );
  }

  const scopeError = checkProjectScope(projectId, context);
  if (scopeError) return scopeError;

  try {
    const response = await getBackendClient().get<AttentionResponse>(
      `/api/external/attention/${encodeURIComponent(projectId)}`,
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    if (err instanceof BackendError && (err.statusCode === 404 || err.statusCode === 501)) {
      // Backend endpoint not yet available — return empty state with note
      return JSON.stringify(
        {
          escalated_reviews: [],
          failed_tasks: [],
          merge_conflicts: [],
          note: "Attention items endpoint not yet available on this backend version.",
        },
        null,
        2
      );
    }
    return handleError(err);
  }
}

interface CapacityResponse {
  can_start: boolean;
  project_running: number;
  project_queued: number;
}

/**
 * v1_get_execution_capacity — check if new tasks can be started.
 * GET /api/external/execution_capacity/:project_id
 *
 * Returns capacity info. Returns default values gracefully if backend
 * endpoint is not yet available.
 */
export async function handleGetExecutionCapacity(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const projectId = args.project_id as string | undefined;

  if (!projectId) {
    return JSON.stringify(
      { error: "missing_argument", message: "project_id is required" },
      null,
      2
    );
  }

  const scopeError = checkProjectScope(projectId, context);
  if (scopeError) return scopeError;

  try {
    const response = await getBackendClient().get<CapacityResponse>(
      `/api/external/execution_capacity/${encodeURIComponent(projectId)}`,
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    if (err instanceof BackendError && (err.statusCode === 404 || err.statusCode === 501)) {
      // Backend endpoint not yet available — return default capacity with note
      return JSON.stringify(
        {
          can_start: true,
          project_running: 0,
          project_queued: 0,
          note: "Execution capacity endpoint not yet available on this backend version.",
        },
        null,
        2
      );
    }
    return handleError(err);
  }
}

// ============================================================================
// Webhook Registration Tools
// ============================================================================

interface RegisterWebhookResponse {
  id: string;
  url: string;
  secret: string;
  event_types?: string[];
  project_ids: string[];
  active: boolean;
  created_at: string;
}

interface WebhookSummary {
  id: string;
  url: string;
  event_types?: string[];
  project_ids: string[];
  active: boolean;
  failure_count: number;
  created_at: string;
}

interface ListWebhooksResponse {
  webhooks: WebhookSummary[];
}

interface UnregisterWebhookResponse {
  success: boolean;
  id: string;
}

/**
 * v1_register_webhook — register a webhook URL for real-time event delivery.
 * POST /api/external/webhooks/register
 *
 * Returns the webhook registration including the HMAC secret (shown ONCE — store it).
 * Idempotent: re-registering the same URL returns the existing registration.
 */
export async function handleRegisterWebhook(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const url = args.url as string | undefined;
  if (!url) {
    return JSON.stringify(
      { error: "missing_argument", message: "url is required" },
      null,
      2
    );
  }

  const eventTypes = Array.isArray(args.event_types)
    ? (args.event_types as string[])
    : undefined;
  const projectIds = Array.isArray(args.project_ids)
    ? (args.project_ids as string[])
    : [];

  // Project scope validation for explicit project_ids
  if (projectIds.length > 0) {
    for (const pid of projectIds) {
      const scopeError = checkProjectScope(pid, context);
      if (scopeError) return scopeError;
    }
  }

  try {
    const response = await getBackendClient().post<RegisterWebhookResponse>(
      "/api/external/webhooks/register",
      context,
      { url, event_types: eventTypes, project_ids: projectIds }
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}

/**
 * v1_unregister_webhook — remove a webhook registration.
 * DELETE /api/external/webhooks/:id
 */
export async function handleUnregisterWebhook(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  const webhookId = args.webhook_id as string | undefined;
  if (!webhookId) {
    return JSON.stringify(
      { error: "missing_argument", message: "webhook_id is required" },
      null,
      2
    );
  }

  try {
    const response = await getBackendClient().delete<UnregisterWebhookResponse>(
      `/api/external/webhooks/${encodeURIComponent(webhookId)}`,
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}

/**
 * v1_list_webhooks — list all registered webhooks for this API key.
 * GET /api/external/webhooks
 */
export async function handleListWebhooks(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  try {
    const response = await getBackendClient().get<ListWebhooksResponse>(
      "/api/external/webhooks",
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}

interface WebhookHealthItem {
  id: string;
  url: string;
  active: boolean;
  failure_count: number;
  last_failure_at?: string;
}

interface WebhookHealthResponse {
  webhooks: WebhookHealthItem[];
}

/**
 * v1_get_webhook_health — check delivery health for all registered webhooks.
 * GET /api/external/webhooks/health
 *
 * Returns per-webhook stats: active status, failure count, and last failure time.
 * Use this to detect broken webhooks before relying on event delivery.
 */
export async function handleGetWebhookHealth(
  args: Record<string, unknown>,
  context: ApiKeyContext
): Promise<string> {
  try {
    const response = await getBackendClient().get<WebhookHealthResponse>(
      "/api/external/webhooks/health",
      context
    );
    return JSON.stringify(response.body, null, 2);
  } catch (err) {
    return handleError(err);
  }
}
