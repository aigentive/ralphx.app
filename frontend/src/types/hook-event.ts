/**
 * Hook Event Types — discriminated union for agent:hook Tauri events
 *
 * Matches AgentHookPayload from chat_service_types.rs
 */

/** Shared fields present on all hook event payloads from the backend */
interface HookEventBase {
  conversationId: string;
  contextType: string;
  contextId: string;
  timestamp: number;
}

/** Hook started — agent is running a hook */
export interface HookStartedEvent extends HookEventBase {
  type: "started";
  hookName: string;
  hookEvent: string;
  hookId: string;
}

/** Hook completed — hook finished running */
export interface HookCompletedEvent extends HookEventBase {
  type: "completed";
  hookName: string;
  hookEvent: string;
  hookId: string;
  output: string | null;
  outcome: string | null;
  exitCode: number | null;
}

/** Hook block — a Stop hook blocked the agent */
export interface HookBlockEvent extends HookEventBase {
  type: "block";
  hookName: string | null;
  reason: string;
}

/** Discriminated union of all hook event types */
export type HookEvent = HookStartedEvent | HookCompletedEvent | HookBlockEvent;

/**
 * Raw payload shape from the backend (snake_case).
 * Transformed to HookEvent (camelCase) in the hook.
 */
export interface RawAgentHookPayload {
  type: "started" | "completed" | "block";
  hook_name?: string | null;
  hook_event?: string | null;
  hook_id?: string | null;
  output?: string | null;
  outcome?: string | null;
  exit_code?: number | null;
  reason?: string | null;
  conversation_id: string;
  context_type: string;
  context_id: string;
  timestamp: number;
}
