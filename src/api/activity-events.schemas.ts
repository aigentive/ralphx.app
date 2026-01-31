// Zod schemas for activity events API responses (snake_case from Rust backend)

import { z } from "zod";

/**
 * Activity event type enum values
 */
export const ActivityEventTypeValues = [
  "thinking",
  "tool_call",
  "tool_result",
  "text",
  "error",
] as const;

/**
 * Activity event role enum values
 */
export const ActivityEventRoleValues = ["agent", "system", "user"] as const;

/**
 * Activity event response schema (snake_case from Rust)
 */
export const ActivityEventResponseSchema = z.object({
  id: z.string(),
  task_id: z.string().nullable(),
  ideation_session_id: z.string().nullable(),
  internal_status: z.string().nullable(),
  event_type: z.string(),
  role: z.string(),
  content: z.string(),
  metadata: z.string().nullable(),
  created_at: z.string(),
});

/**
 * Paginated response for activity events (snake_case from Rust)
 */
export const ActivityEventPageResponseSchema = z.object({
  events: z.array(ActivityEventResponseSchema),
  cursor: z.string().nullable(),
  has_more: z.boolean(),
});

/**
 * Filter input for activity event queries (snake_case for Rust)
 */
export const ActivityEventFilterInputSchema = z.object({
  event_types: z.array(z.string()).optional(),
  roles: z.array(z.string()).optional(),
  statuses: z.array(z.string()).optional(),
});
