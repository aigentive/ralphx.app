// Zod schemas for ideation API responses (snake_case from Rust backend)

import { z } from "zod";

/**
 * Ideation session response schema (snake_case from Rust)
 */
export const IdeationSessionResponseSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  title: z.string().nullable(),
  status: z.string(),
  plan_artifact_id: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
  archived_at: z.string().nullable(),
  converted_at: z.string().nullable(),
});

/**
 * Task proposal response schema (snake_case from Rust)
 */
export const TaskProposalResponseSchema = z.object({
  id: z.string(),
  session_id: z.string(),
  title: z.string(),
  description: z.string().nullable(),
  category: z.string(),
  steps: z.array(z.string()),
  acceptance_criteria: z.array(z.string()),
  suggested_priority: z.string(),
  priority_score: z.number(),
  priority_reason: z.string().nullable(),
  estimated_complexity: z.string(),
  user_priority: z.string().nullable(),
  user_modified: z.boolean(),
  status: z.string(),
  selected: z.boolean(),
  created_task_id: z.string().nullable(),
  plan_artifact_id: z.string().nullable(),
  plan_version_at_creation: z.number().nullable(),
  sort_order: z.number(),
  created_at: z.string(),
  updated_at: z.string(),
});

/**
 * Chat message response schema (snake_case from Rust)
 */
export const ChatMessageResponseSchema = z.object({
  id: z.string(),
  session_id: z.string().nullable(),
  project_id: z.string().nullable(),
  task_id: z.string().nullable(),
  role: z.string(),
  content: z.string(),
  metadata: z.string().nullable(),
  tool_calls: z.string().nullable(),
  parent_message_id: z.string().nullable(),
  created_at: z.string(),
});

/**
 * Session with proposals and messages (snake_case from Rust)
 */
export const SessionWithDataResponseSchema = z.object({
  session: IdeationSessionResponseSchema,
  proposals: z.array(TaskProposalResponseSchema),
  messages: z.array(ChatMessageResponseSchema),
});

/**
 * Priority assessment response (snake_case from Rust)
 */
export const PriorityAssessmentResponseSchema = z.object({
  proposal_id: z.string(),
  priority: z.string(),
  score: z.number(),
  reason: z.string(),
});

/**
 * Dependency graph node response (snake_case from Rust)
 */
export const DependencyGraphNodeResponseSchema = z.object({
  proposal_id: z.string(),
  title: z.string(),
  in_degree: z.number(),
  out_degree: z.number(),
});

/**
 * Dependency graph edge response (snake_case from Rust)
 */
export const DependencyGraphEdgeResponseSchema = z.object({
  from: z.string(),
  to: z.string(),
  reason: z.string().nullable().optional(),
});

/**
 * Dependency graph response (snake_case from Rust)
 */
export const DependencyGraphResponseSchema = z.object({
  nodes: z.array(DependencyGraphNodeResponseSchema),
  edges: z.array(DependencyGraphEdgeResponseSchema),
  critical_path: z.array(z.string()),
  has_cycles: z.boolean(),
  cycles: z.array(z.array(z.string())).nullable(),
});

/**
 * Apply proposals result response (snake_case from Rust)
 */
export const ApplyProposalsResultResponseSchema = z.object({
  created_task_ids: z.array(z.string()),
  dependencies_created: z.number(),
  warnings: z.array(z.string()),
  session_converted: z.boolean(),
});
