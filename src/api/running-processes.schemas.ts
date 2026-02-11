// Zod schemas for running processes API responses (snake_case from Rust backend)

import { z } from "zod";

/**
 * Step progress summary schema from Rust (snake_case)
 */
export const StepProgressSummarySchema = z.object({
  task_id: z.string(),
  total: z.number().int().nonnegative(),
  completed: z.number().int().nonnegative(),
  in_progress: z.number().int().nonnegative(),
  pending: z.number().int().nonnegative(),
  skipped: z.number().int().nonnegative(),
  failed: z.number().int().nonnegative(),
  current_step: z.any().nullable(), // TaskStep - nullable
  next_step: z.any().nullable(), // TaskStep - nullable
  percent_complete: z.number(),
});

/**
 * Running process schema from Rust (snake_case)
 */
export const RunningProcessSchema = z.object({
  task_id: z.string(),
  title: z.string(),
  internal_status: z.string(),
  step_progress: StepProgressSummarySchema.nullable(),
  elapsed_seconds: z.number().int().nullable(),
  trigger_origin: z.string().nullable(),
  task_branch: z.string().nullable(),
});

/**
 * Running processes response schema from Rust (snake_case)
 */
export const RunningProcessesResponseSchema = z.object({
  processes: z.array(RunningProcessSchema),
});
