// QA Types - Acceptance Criteria, Test Steps, and Results
// Mirrors Rust domain/qa types for frontend use

import { z } from "zod";

// ============================================================================
// Acceptance Criteria Type
// ============================================================================

export const AcceptanceCriteriaTypeSchema = z.enum([
  "visual",
  "behavior",
  "data",
  "accessibility",
]);

export type AcceptanceCriteriaType = z.infer<typeof AcceptanceCriteriaTypeSchema>;

export const ACCEPTANCE_CRITERIA_TYPE_VALUES = AcceptanceCriteriaTypeSchema.options;

// ============================================================================
// Acceptance Criterion
// ============================================================================

export const AcceptanceCriterionSchema = z.object({
  /** Unique identifier (e.g., "AC1", "AC2") */
  id: z.string(),
  /** Description of what needs to be verified */
  description: z.string(),
  /** Whether this criterion can be tested automatically */
  testable: z.boolean(),
  /** Type of criterion for categorization */
  type: AcceptanceCriteriaTypeSchema,
});

export type AcceptanceCriterion = z.infer<typeof AcceptanceCriterionSchema>;

// ============================================================================
// Acceptance Criteria Collection
// ============================================================================

export const AcceptanceCriteriaSchema = z.object({
  /** List of acceptance criteria */
  acceptance_criteria: z.array(AcceptanceCriterionSchema),
});

export type AcceptanceCriteria = z.infer<typeof AcceptanceCriteriaSchema>;

// ============================================================================
// QA Test Step
// ============================================================================

export const QATestStepSchema = z.object({
  /** Unique identifier (e.g., "QA1", "QA2") */
  id: z.string(),
  /** Reference to the acceptance criterion being tested */
  criteria_id: z.string(),
  /** Human-readable description of what this step verifies */
  description: z.string(),
  /** List of agent-browser commands to execute */
  commands: z.array(z.string()),
  /** Expected outcome description */
  expected: z.string(),
});

export type QATestStep = z.infer<typeof QATestStepSchema>;

// ============================================================================
// QA Test Steps Collection
// ============================================================================

export const QATestStepsSchema = z.object({
  /** List of test steps */
  qa_steps: z.array(QATestStepSchema),
});

export type QATestSteps = z.infer<typeof QATestStepsSchema>;

// ============================================================================
// QA Step Status
// ============================================================================

export const QAStepStatusSchema = z.enum([
  "pending",
  "running",
  "passed",
  "failed",
  "skipped",
]);

export type QAStepStatus = z.infer<typeof QAStepStatusSchema>;

export const QA_STEP_STATUS_VALUES = QAStepStatusSchema.options;

/** Check if step is in a terminal state (passed, failed, or skipped) */
export function isStepTerminal(status: QAStepStatus): boolean {
  return status === "passed" || status === "failed" || status === "skipped";
}

/** Check if step passed */
export function isStepPassed(status: QAStepStatus): boolean {
  return status === "passed";
}

/** Check if step failed */
export function isStepFailed(status: QAStepStatus): boolean {
  return status === "failed";
}

// ============================================================================
// QA Overall Status
// ============================================================================

export const QAOverallStatusSchema = z.enum([
  "pending",
  "running",
  "passed",
  "failed",
]);

export type QAOverallStatus = z.infer<typeof QAOverallStatusSchema>;

export const QA_OVERALL_STATUS_VALUES = QAOverallStatusSchema.options;

/** Check if overall testing is complete (passed or failed) */
export function isOverallComplete(status: QAOverallStatus): boolean {
  return status === "passed" || status === "failed";
}

// ============================================================================
// QA Step Result
// ============================================================================

export const QAStepResultSchema = z.object({
  /** Reference to the QA step ID */
  step_id: z.string(),
  /** Current status of this step */
  status: QAStepStatusSchema,
  /** Path to screenshot captured during this step (if any) */
  screenshot: z.string().nullish(),
  /** Actual observed value (for comparison failures) */
  actual: z.string().nullish(),
  /** Expected value (for comparison failures) */
  expected: z.string().nullish(),
  /** Error message if step failed */
  error: z.string().nullish(),
});

export type QAStepResult = z.infer<typeof QAStepResultSchema>;

// ============================================================================
// QA Results Totals
// ============================================================================

export const QAResultsTotalsSchema = z.object({
  /** Total number of test steps */
  total_steps: z.number().int().nonnegative(),
  /** Number of passed steps */
  passed_steps: z.number().int().nonnegative(),
  /** Number of failed steps */
  failed_steps: z.number().int().nonnegative(),
  /** Number of skipped steps */
  skipped_steps: z.number().int().nonnegative(),
});

export type QAResultsTotals = z.infer<typeof QAResultsTotalsSchema>;

/** Calculate totals from step results */
export function calculateTotals(results: QAStepResult[]): QAResultsTotals {
  const totals: QAResultsTotals = {
    total_steps: results.length,
    passed_steps: 0,
    failed_steps: 0,
    skipped_steps: 0,
  };

  for (const result of results) {
    if (result.status === "passed") totals.passed_steps++;
    else if (result.status === "failed") totals.failed_steps++;
    else if (result.status === "skipped") totals.skipped_steps++;
  }

  return totals;
}

// ============================================================================
// QA Results
// ============================================================================

export const QAResultsSchema = z.object({
  /** Task ID these results belong to */
  task_id: z.string(),
  /** Overall test status */
  overall_status: QAOverallStatusSchema,
  /** Total number of steps */
  total_steps: z.number().int().nonnegative(),
  /** Number of passed steps */
  passed_steps: z.number().int().nonnegative(),
  /** Number of failed steps */
  failed_steps: z.number().int().nonnegative(),
  /** Individual step results */
  steps: z.array(QAStepResultSchema),
});

export type QAResults = z.infer<typeof QAResultsSchema>;

// ============================================================================
// TaskQA - Full QA record for a task
// ============================================================================

export const TaskQASchema = z.object({
  /** Unique identifier for this QA record */
  id: z.string(),
  /** The task this QA data belongs to */
  task_id: z.string(),

  // ----- Phase 1: QA Prep -----
  /** Acceptance criteria generated by QA Prep agent */
  acceptance_criteria: AcceptanceCriteriaSchema.optional(),
  /** Initial test steps generated by QA Prep agent */
  qa_test_steps: QATestStepsSchema.optional(),
  /** ID of the agent that performed QA prep */
  prep_agent_id: z.string().optional(),
  /** When QA prep started (ISO string) */
  prep_started_at: z.string().optional(),
  /** When QA prep completed (ISO string) */
  prep_completed_at: z.string().optional(),

  // ----- Phase 2: QA Refinement -----
  /** Summary of what was actually implemented (from git diff) */
  actual_implementation: z.string().optional(),
  /** Test steps updated based on actual implementation */
  refined_test_steps: QATestStepsSchema.optional(),
  /** ID of the agent that performed refinement */
  refinement_agent_id: z.string().optional(),
  /** When refinement completed (ISO string) */
  refinement_completed_at: z.string().optional(),

  // ----- Phase 3: QA Testing -----
  /** Test execution results */
  test_results: QAResultsSchema.optional(),
  /** Paths to captured screenshots */
  screenshots: z.array(z.string()).default([]),
  /** ID of the agent that executed tests */
  test_agent_id: z.string().optional(),
  /** When testing completed (ISO string) */
  test_completed_at: z.string().optional(),

  /** When this record was created (ISO string) */
  created_at: z.string(),
});

export type TaskQA = z.infer<typeof TaskQASchema>;

// ============================================================================
// Parsing Utilities
// ============================================================================

export function parseAcceptanceCriteria(data: unknown): AcceptanceCriteria {
  return AcceptanceCriteriaSchema.parse(data);
}

export function safeParseAcceptanceCriteria(data: unknown): AcceptanceCriteria | null {
  const result = AcceptanceCriteriaSchema.safeParse(data);
  return result.success ? result.data : null;
}

export function parseQATestSteps(data: unknown): QATestSteps {
  return QATestStepsSchema.parse(data);
}

export function safeParseQATestSteps(data: unknown): QATestSteps | null {
  const result = QATestStepsSchema.safeParse(data);
  return result.success ? result.data : null;
}

export function parseQAResults(data: unknown): QAResults {
  return QAResultsSchema.parse(data);
}

export function safeParseQAResults(data: unknown): QAResults | null {
  const result = QAResultsSchema.safeParse(data);
  return result.success ? result.data : null;
}

export function parseTaskQA(data: unknown): TaskQA {
  return TaskQASchema.parse(data);
}

export function safeParseTaskQA(data: unknown): TaskQA | null {
  const result = TaskQASchema.safeParse(data);
  return result.success ? result.data : null;
}
