// Transform functions for converting snake_case execution API responses to camelCase frontend types

import { z } from "zod";
import {
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
  ExecutionSettingsResponseSchema,
  GlobalExecutionSettingsResponseSchema,
} from "./execution.schemas";
import type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
  ExecutionSettingsResponse,
  UpdateExecutionSettingsInput,
  GlobalExecutionSettingsResponse,
  UpdateGlobalExecutionSettingsInput,
} from "./execution.types";

/**
 * Transform ExecutionStatusResponseSchema (snake_case) → ExecutionStatusResponse (camelCase)
 * Phase 82: Added globalMaxConcurrent transform
 */
export function transformExecutionStatus(
  raw: z.infer<typeof ExecutionStatusResponseSchema>
): ExecutionStatusResponse {
  return {
    isPaused: raw.is_paused,
    runningCount: raw.running_count,
    maxConcurrent: raw.max_concurrent,
    globalMaxConcurrent: raw.global_max_concurrent,
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

/**
 * Transform ExecutionSettingsResponseSchema (snake_case) → ExecutionSettingsResponse (camelCase)
 */
export function transformExecutionSettings(
  raw: z.infer<typeof ExecutionSettingsResponseSchema>
): ExecutionSettingsResponse {
  return {
    maxConcurrentTasks: raw.max_concurrent_tasks,
    autoCommit: raw.auto_commit,
    pauseOnFailure: raw.pause_on_failure,
  };
}

/**
 * Transform UpdateExecutionSettingsInput (camelCase) → snake_case for Tauri command
 */
export function transformExecutionSettingsInput(
  input: UpdateExecutionSettingsInput
): { max_concurrent_tasks: number; auto_commit: boolean; pause_on_failure: boolean } {
  return {
    max_concurrent_tasks: input.maxConcurrentTasks,
    auto_commit: input.autoCommit,
    pause_on_failure: input.pauseOnFailure,
  };
}

/**
 * Transform GlobalExecutionSettingsResponseSchema (snake_case) → GlobalExecutionSettingsResponse (camelCase)
 * Phase 82
 */
export function transformGlobalExecutionSettings(
  raw: z.infer<typeof GlobalExecutionSettingsResponseSchema>
): GlobalExecutionSettingsResponse {
  return {
    globalMaxConcurrent: raw.global_max_concurrent,
  };
}

/**
 * Transform UpdateGlobalExecutionSettingsInput (camelCase) → snake_case for Tauri command
 * Phase 82
 */
export function transformGlobalExecutionSettingsInput(
  input: UpdateGlobalExecutionSettingsInput
): { global_max_concurrent: number } {
  return {
    global_max_concurrent: input.globalMaxConcurrent,
  };
}
