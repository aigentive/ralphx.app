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
});
