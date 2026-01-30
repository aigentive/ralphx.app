// Transform functions for converting snake_case execution API responses to camelCase frontend types

import { z } from "zod";
import {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
} from "./execution.schemas";
import type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
} from "./execution.types";

/**
 * Transform ExecutionStatusResponseSchema (snake_case) → ExecutionStatusResponse (camelCase)
 */
export function transformExecutionStatus(
  raw: z.infer<typeof ExecutionStatusResponseSchema>
): ExecutionStatusResponse {
  return {
    isPaused: raw.is_paused,
    runningCount: raw.running_count,
    maxConcurrent: raw.max_concurrent,
    queuedCount: raw.queued_count,
    canStartTask: raw.can_start_task,
  };
}

/**
 * Transform ExecutionCommandResponseSchema → ExecutionCommandResponse
 */
export function transformExecutionCommand(
  raw: z.infer<typeof ExecutionCommandResponseSchema>
): ExecutionCommandResponse {
  return {
    success: raw.success,
    status: transformExecutionStatus(raw.status),
  };
}
