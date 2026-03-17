// Zod schemas for ideation API responses (snake_case from Rust backend)

import { z } from "zod";
import { VerificationGapSchema } from "../types/ideation";

/**
 * Ideation session response schema (snake_case from Rust)
 */
export const IdeationSessionResponseSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  title: z.string().nullable(),
  title_source: z.enum(["auto", "user"]).nullable().optional(),
  status: z.string(),
  plan_artifact_id: z.string().nullable(),
  seed_task_id: z.string().nullable().optional(),
  parent_session_id: z.string().nullable(),
  team_mode: z.enum(["solo", "research", "debate"]).nullable().optional(),
  team_config: z.object({
    max_teammates: z.number(),
    model_ceiling: z.string(),
    budget_limit: z.number().nullable().optional(),
    composition_mode: z.string().nullable().optional(),
  }).nullable().optional(),
  created_at: z.string(),
  updated_at: z.string(),
  archived_at: z.string().nullable(),
  converted_at: z.string().nullable(),
  verification_status: z.string().optional(),
  verification_in_progress: z.boolean().optional(),
  gap_score: z.number().int().nullable().optional(),
  source_project_id: z.string().nullable().optional(),
  source_session_id: z.string().nullable().optional(),
  inherited_plan_artifact_id: z.string().nullable().optional(),
  session_purpose: z.enum(["general", "verification"]).optional(),
});

/**
 * API gap schema (snake_case from HTTP server) — transforms to VerificationGap shape.
 * Reuses VerificationGapSchema as the output type (single source of truth).
 */
export const ApiVerificationGapSchema = z.object({
  severity: z.enum(["critical", "high", "medium", "low"]),
  category: z.string(),
  description: z.string(),
  why_it_matters: z.string().optional(),
}).transform((val): z.infer<typeof VerificationGapSchema> => ({
  severity: val.severity,
  category: val.category,
  description: val.description,
  ...(val.why_it_matters !== undefined && { whyItMatters: val.why_it_matters }),
}));

/**
 * API round summary schema (snake_case from HTTP server) — transforms to RoundSummary shape.
 */
export const ApiRoundSummarySchema = z.object({
  round: z.number(),
  gap_score: z.number(),
  gap_count: z.number(),
}).transform((val) => ({
  round: val.round,
  gapScore: val.gap_score,
  gapCount: val.gap_count,
}));

/**
 * Verification status response schema (snake_case from HTTP server)
 */
export const VerificationResponseSchema = z.object({
  session_id: z.string(),
  status: z.string(),
  in_progress: z.boolean(),
  current_round: z.number().int().optional(),
  max_rounds: z.number().int().optional(),
  gap_score: z.number().int().optional(),
  convergence_reason: z.string().optional(),
  best_round_index: z.number().int().optional(),
  current_gaps: z.array(ApiVerificationGapSchema).optional().default([]),
  rounds: z.array(ApiRoundSummarySchema).optional().default([]),
  plan_version: z.number().int().optional(),
  verification_generation: z.number().int(),
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
 * Dependency analysis summary (snake_case from Rust)
 */
export const DependencyAnalysisSummarySchema = z.object({
  total_proposals: z.number(),
  root_count: z.number(),
  leaf_count: z.number(),
  max_depth: z.number(),
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
  message: z.string().nullable().optional(),
  summary: DependencyAnalysisSummarySchema.nullable().optional(),
});

/**
 * Apply proposals result response (snake_case from Rust)
 */
export const ApplyProposalsResultResponseSchema = z.object({
  created_task_ids: z.array(z.string()),
  dependencies_created: z.number(),
  warnings: z.array(z.string()),
  session_converted: z.boolean(),
  execution_plan_id: z.string().nullable().optional(),
});

/**
 * Parent session context response (snake_case from Rust)
 */
export const ParentSessionContextResponseSchema = z.object({
  parent_session: z.object({
    id: z.string(),
    title: z.string().nullable(),
    status: z.string(),
  }),
  plan_content: z.string().nullable(),
  proposals: z.array(
    z.object({
      id: z.string(),
      title: z.string(),
      category: z.string(),
      priority: z.string().nullable(),
      status: z.string(),
      acceptance_criteria: z.array(z.string()),
    })
  ),
});

/**
 * Team mode and config schemas (snake_case from Rust)
 */
export const TeamModeResponseSchema = z.enum(["solo", "research", "debate"]);
export const CompositionModeResponseSchema = z.enum(["dynamic", "constrained"]);
export const TeamConfigResponseSchema = z.object({
  max_teammates: z.number(),
  model_ceiling: z.string(),
  budget_limit: z.number().nullable().optional(),
  composition_mode: z.string().nullable().optional(),
});

/**
 * Create child session response (snake_case from Rust)
 */
export const CreateChildSessionResponseSchema = z.object({
  session_id: z.string(),
  parent_session_id: z.string(),
  title: z.string().nullable(),
  status: z.string(),
  created_at: z.string(),
  generation: z.number().optional(),
  parent_context: ParentSessionContextResponseSchema.optional(),
});
