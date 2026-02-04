// Tauri invoke wrappers for execution control with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";
import {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
  ExecutionSettingsResponseSchema,
} from "./execution.schemas";
import {
  transformExecutionStatus,
  transformExecutionCommand,
  transformExecutionSettings,
  transformExecutionSettingsInput,
} from "./execution.transforms";
import type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
  ExecutionSettingsResponse,
  UpdateExecutionSettingsInput,
} from "./execution.types";

// Re-export types for convenience
export type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
  ExecutionSettingsResponse,
  UpdateExecutionSettingsInput,
} from "./execution.types";

// Re-export schemas for consumers that need validation
export {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
  ExecutionSettingsResponseSchema,
  UpdateExecutionSettingsInputSchema,
} from "./execution.schemas";

// Re-export transforms for consumers that need manual transformation
export {
  transformExecutionStatus,
  transformExecutionCommand,
  transformExecutionSettings,
  transformExecutionSettingsInput,
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

  /**
   * Get execution settings from database
   * @returns Execution settings with max concurrent tasks, auto-commit, pause on failure
   */
  getSettings: async (): Promise<ExecutionSettingsResponse> => {
    const result = await invoke("get_execution_settings", {});

    if (!result) {
      const defaults = DEFAULT_PROJECT_SETTINGS.execution;
      return transformExecutionSettings({
        max_concurrent_tasks: defaults.max_concurrent_tasks,
        auto_commit: defaults.auto_commit,
        pause_on_failure: defaults.pause_on_failure,
      });
    }

    const validated = ExecutionSettingsResponseSchema.parse(result);
    return transformExecutionSettings(validated);
  },

  /**
   * Update execution settings in database
   * Also syncs ExecutionState when max_concurrent_tasks changes
   * @param input - Settings to update
   * @returns Updated execution settings
   */
  updateSettings: (input: UpdateExecutionSettingsInput): Promise<ExecutionSettingsResponse> =>
    typedInvokeWithTransform(
      "update_execution_settings",
      { input: transformExecutionSettingsInput(input) },
      ExecutionSettingsResponseSchema,
      transformExecutionSettings
    ),
} as const;
