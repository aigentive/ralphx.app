// Zod schemas for tasks API responses (snake_case from Rust backend)

import { z } from "zod";
import { TaskSchema } from "@/types/task";

/**
 * Inject task response schema from Rust (snake_case)
 * Backend outputs snake_case (Rust default). Transform layer converts to camelCase for UI.
 */
export const InjectTaskResponseSchemaRaw = z.object({
  task: TaskSchema,
  target: z.enum(["backlog", "planned"]),
  priority: z.number().int(),
  make_next_applied: z.boolean(),
});

/**
 * Cleanup report response schema from Rust (snake_case)
 * Returned by cleanup_tasks_in_group command.
 */
export const CleanupReportResponseSchemaRaw = z.object({
  deleted_count: z.number().int(),
  failed_count: z.number().int(),
  stopped_agents: z.number().int(),
});

/**
 * State transition response schema from Rust (snake_case)
 * Used by StateTimelineNav for displaying task state history.
 */
export const StateTransitionResponseSchemaRaw = z.object({
  /** Status transitioned from (null for initial state) */
  from_status: z.string().nullable(),
  /** Status transitioned to */
  to_status: z.string(),
  /** What triggered this transition (e.g., "user", "agent", "system") */
  trigger: z.string(),
  /** When the transition occurred (RFC3339 format) */
  timestamp: z.string(),
  /** Conversation ID for states that spawn conversations (executing, re_executing, reviewing) */
  conversation_id: z.string().nullish(),
  /** Agent run ID for the specific execution within the conversation */
  agent_run_id: z.string().nullish(),
});
