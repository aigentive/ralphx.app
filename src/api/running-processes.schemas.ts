// Zod schemas for running processes API responses (snake_case from Rust backend)

import { z } from "zod";
import { TaskStepResponseSchema } from "@/types/task-step";

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
  current_step: TaskStepResponseSchema.nullable(),
  next_step: TaskStepResponseSchema.nullable(),
  percent_complete: z.number(),
});

/**
 * Teammate summary schema from Rust (snake_case)
 */
export const TeammateSummarySchema = z.object({
  name: z.string(),
  status: z.string(),
  step: z.string().optional(),
  model: z.string().optional(),
  color: z.string().optional(),
  steps_completed: z.number().int().nonnegative().optional(),
  steps_total: z.number().int().nonnegative().optional(),
  wave: z.number().int().nonnegative().optional(),
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
  team_name: z.string().optional(),
  teammates: z.array(TeammateSummarySchema).optional(),
  current_wave: z.number().int().nonnegative().optional(),
  total_waves: z.number().int().nonnegative().optional(),
});

/**
 * Running ideation session schema from Rust (snake_case)
 */
export const RunningIdeationSessionSchema = z.object({
  session_id: z.string(),
  title: z.string(),
  elapsed_seconds: z.number().int().nullable(),
  team_mode: z.string().nullable(),
});

/**
 * Running processes response schema from Rust (snake_case)
 */
export const RunningProcessesResponseSchema = z.object({
  processes: z.array(RunningProcessSchema),
  ideation_sessions: z.array(RunningIdeationSessionSchema),
});
