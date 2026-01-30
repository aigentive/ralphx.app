// Tauri invoke wrappers for execution control with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
} from "./execution.schemas";
import {
  transformExecutionStatus,
  transformExecutionCommand,
} from "./execution.transforms";
import type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
} from "./execution.types";

// Re-export types for convenience
export type { ExecutionStatusResponse, ExecutionCommandResponse } from "./execution.types";

// Re-export schemas for consumers that need validation
export {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
} from "./execution.schemas";

// Re-export transforms for consumers that need manual transformation
export {
  transformExecutionStatus,
  transformExecutionCommand,
} from "./execution.transforms";

// ============================================================================
// Typed Invoke Helper
// ============================================================================

async function typedInvokeWithTransform<TRaw, TResult>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<TRaw>,
  transform: (raw: TRaw) => TResult
): Promise<TResult> {
  const result = await invoke(cmd, args);
  const validated = schema.parse(result);
  return transform(validated);
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Execution control API wrappers for Tauri commands
 */
export const executionApi = {
  /**
   * Get current execution status
   * @returns Execution status with pause state, running count, queued count
   */
  getStatus: (): Promise<ExecutionStatusResponse> =>
    typedInvokeWithTransform(
      "get_execution_status",
      {},
      ExecutionStatusResponseSchema,
      transformExecutionStatus
    ),

  /**
   * Pause execution (stops picking up new tasks)
   * @returns Command response with success and current status
   */
  pause: (): Promise<ExecutionCommandResponse> =>
    typedInvokeWithTransform(
      "pause_execution",
      {},
      ExecutionCommandResponseSchema,
      transformExecutionCommand
    ),

  /**
   * Resume execution (allows picking up new tasks)
   * @returns Command response with success and current status
   */
  resume: (): Promise<ExecutionCommandResponse> =>
    typedInvokeWithTransform(
      "resume_execution",
      {},
      ExecutionCommandResponseSchema,
      transformExecutionCommand
    ),

  /**
   * Stop execution (cancels current tasks and pauses)
   * @returns Command response with success and current status
   */
  stop: (): Promise<ExecutionCommandResponse> =>
    typedInvokeWithTransform(
      "stop_execution",
      {},
      ExecutionCommandResponseSchema,
      transformExecutionCommand
    ),
} as const;
