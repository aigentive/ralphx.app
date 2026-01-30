// Zod schemas for execution API responses (snake_case from Rust backend)

import { z } from "zod";

/**
 * Execution status response schema from Rust (snake_case)
 * Backend outputs snake_case by default (no rename_all annotation)
 */
export const ExecutionStatusResponseSchema = z.object({
  is_paused: z.boolean(),
  running_count: z.number().int().nonnegative(),
  max_concurrent: z.number().int().nonnegative(),
  queued_count: z.number().int().nonnegative(),
  can_start_task: z.boolean(),
});

/**
 * Execution command response schema from Rust (for pause/resume/stop) (snake_case)
 */
export const ExecutionCommandResponseSchema = z.object({
  success: z.boolean(),
  status: ExecutionStatusResponseSchema,
});
