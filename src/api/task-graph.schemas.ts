// Zod schemas for task graph API responses (snake_case from Rust backend)

import { z } from "zod";

/**
 * Node in the task dependency graph (snake_case from backend)
 */
export const TaskGraphNodeSchema = z.object({
  task_id: z.string(),
  title: z.string(),
  description: z.string().nullable(),
  category: z.string(),
  internal_status: z.string(),
  priority: z.number().int(),
  in_degree: z.number().int().nonnegative(),
  out_degree: z.number().int().nonnegative(),
  tier: z.number().int().nonnegative(),
  plan_artifact_id: z.string().nullable(),
  source_proposal_id: z.string().nullable(),
});

/**
 * Edge in the task dependency graph (snake_case from backend)
 */
export const TaskGraphEdgeSchema = z.object({
  source: z.string(),
  target: z.string(),
  is_critical_path: z.boolean(),
});

/**
 * Status summary for a plan group (snake_case from backend)
 */
export const StatusSummarySchema = z.object({
  backlog: z.number().int().nonnegative(),
  ready: z.number().int().nonnegative(),
  blocked: z.number().int().nonnegative(),
  executing: z.number().int().nonnegative(),
  qa: z.number().int().nonnegative(),
  review: z.number().int().nonnegative(),
  merge: z.number().int().nonnegative(),
  completed: z.number().int().nonnegative(),
  terminal: z.number().int().nonnegative(),
});

/**
 * Information about a plan group in the graph (snake_case from backend)
 */
export const PlanGroupInfoSchema = z.object({
  plan_artifact_id: z.string(),
  session_id: z.string(),
  session_title: z.string().nullable(),
  task_ids: z.array(z.string()),
  status_summary: StatusSummarySchema,
});

/**
 * Full task dependency graph response (snake_case from backend)
 */
export const TaskDependencyGraphResponseSchema = z.object({
  nodes: z.array(TaskGraphNodeSchema),
  edges: z.array(TaskGraphEdgeSchema),
  plan_groups: z.array(PlanGroupInfoSchema),
  critical_path: z.array(z.string()),
  has_cycles: z.boolean(),
});

// ============================================================================
// Timeline Event Schemas (Phase 67 - Task D.2)
// ============================================================================

/**
 * Event type enum for timeline entries (snake_case from backend)
 */
export const TimelineEventTypeSchema = z.enum([
  "status_change",
  "plan_accepted",
  "plan_completed",
]);

/**
 * Single event in the execution timeline (snake_case from backend)
 */
export const TimelineEventSchema = z.object({
  id: z.string(),
  timestamp: z.string(),
  task_id: z.string().nullable(),
  task_title: z.string().nullable(),
  event_type: TimelineEventTypeSchema,
  from_status: z.string().nullable(),
  to_status: z.string().nullable(),
  description: z.string(),
  trigger: z.string().nullable(),
  plan_artifact_id: z.string().nullable(),
  session_title: z.string().nullable(),
});

/**
 * Response for timeline events query (snake_case from backend)
 */
export const TimelineEventsResponseSchema = z.object({
  events: z.array(TimelineEventSchema),
  total: z.number().int().nonnegative(),
  has_more: z.boolean(),
});
