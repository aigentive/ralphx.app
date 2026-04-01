// QA API Response Schemas (snake_case from backend)
// These schemas validate Rust backend responses before transformation

import { z } from "zod";
import {
  AcceptanceCriteriaTypeSchema,
  QAStepStatusSchema,
  QAOverallStatusSchema,
} from "@/types";

// ============================================================================
// QA Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Acceptance criterion response from Rust
 * Note: `type` field is renamed to `criteria_type` in Rust response
 */
export const AcceptanceCriterionResponseSchema = z.object({
  id: z.string(),
  description: z.string(),
  testable: z.boolean(),
  criteria_type: AcceptanceCriteriaTypeSchema,
});

export type AcceptanceCriterionResponse = z.infer<typeof AcceptanceCriterionResponseSchema>;

/**
 * QA test step response from Rust
 */
export const QATestStepResponseSchema = z.object({
  id: z.string(),
  criteria_id: z.string(),
  description: z.string(),
  commands: z.array(z.string()),
  expected: z.string(),
});

export type QATestStepResponse = z.infer<typeof QATestStepResponseSchema>;

/**
 * QA step result response from Rust
 */
export const QAStepResultResponseSchema = z.object({
  step_id: z.string(),
  status: QAStepStatusSchema,
  screenshot: z.string().optional(),
  actual: z.string().optional(),
  expected: z.string().optional(),
  error: z.string().optional(),
});

export type QAStepResultResponse = z.infer<typeof QAStepResultResponseSchema>;

/**
 * QA results response from Rust
 */
export const QAResultsResponseSchema = z.object({
  task_id: z.string(),
  overall_status: QAOverallStatusSchema,
  total_steps: z.number().int().nonnegative(),
  passed_steps: z.number().int().nonnegative(),
  failed_steps: z.number().int().nonnegative(),
  steps: z.array(QAStepResultResponseSchema),
});

export type QAResultsResponse = z.infer<typeof QAResultsResponseSchema>;

/**
 * TaskQA response from Rust - full QA record for a task
 */
export const TaskQAResponseSchema = z.object({
  id: z.string(),
  task_id: z.string(),

  // Phase 1: QA Prep
  acceptance_criteria: z.array(AcceptanceCriterionResponseSchema).optional(),
  qa_test_steps: z.array(QATestStepResponseSchema).optional(),
  prep_agent_id: z.string().optional(),
  prep_started_at: z.string().optional(),
  prep_completed_at: z.string().optional(),

  // Phase 2: QA Refinement
  actual_implementation: z.string().optional(),
  refined_test_steps: z.array(QATestStepResponseSchema).optional(),
  refinement_agent_id: z.string().optional(),
  refinement_completed_at: z.string().optional(),

  // Phase 3: QA Testing
  test_results: QAResultsResponseSchema.optional(),
  screenshots: z.array(z.string()),
  test_agent_id: z.string().optional(),
  test_completed_at: z.string().optional(),

  created_at: z.string(),
});

export type TaskQAResponse = z.infer<typeof TaskQAResponseSchema>;
