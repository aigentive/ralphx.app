// Transform functions for converting snake_case running processes API responses to camelCase frontend types

import { z } from "zod";
import {
  StepProgressSummarySchema,
  RunningProcessSchema,
  RunningProcessesResponseSchema,
} from "./running-processes.schemas";
import type {
  StepProgressSummary,
  RunningProcess,
  RunningProcessesResponse,
} from "./running-processes.types";

/**
 * Transform StepProgressSummarySchema (snake_case) → StepProgressSummary (camelCase)
 */
export function transformStepProgressSummary(
  raw: z.infer<typeof StepProgressSummarySchema>
): StepProgressSummary {
  return {
    taskId: raw.task_id,
    total: raw.total,
    completed: raw.completed,
    inProgress: raw.in_progress,
    pending: raw.pending,
    skipped: raw.skipped,
    failed: raw.failed,
    currentStep: raw.current_step,
    nextStep: raw.next_step,
    percentComplete: raw.percent_complete,
  };
}

/**
 * Transform RunningProcessSchema (snake_case) → RunningProcess (camelCase)
 */
export function transformRunningProcess(
  raw: z.infer<typeof RunningProcessSchema>
): RunningProcess {
  return {
    taskId: raw.task_id,
    title: raw.title,
    internalStatus: raw.internal_status,
    stepProgress: raw.step_progress
      ? transformStepProgressSummary(raw.step_progress)
      : null,
    elapsedSeconds: raw.elapsed_seconds,
    triggerOrigin: raw.trigger_origin,
    taskBranch: raw.task_branch,
  };
}

/**
 * Transform RunningProcessesResponseSchema (snake_case) → RunningProcessesResponse (camelCase)
 */
export function transformRunningProcessesResponse(
  raw: z.infer<typeof RunningProcessesResponseSchema>
): RunningProcessesResponse {
  return {
    processes: raw.processes.map(transformRunningProcess),
  };
}
