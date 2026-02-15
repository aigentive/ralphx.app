// Transform functions for converting snake_case running processes API responses to camelCase frontend types

import { z } from "zod";
import {
  StepProgressSummarySchema,
  RunningProcessSchema,
  RunningProcessesResponseSchema,
  TeammateSummarySchema,
} from "./running-processes.schemas";
import type {
  StepProgressSummary,
  RunningProcess,
  RunningProcessesResponse,
  TeammateSummary,
} from "./running-processes.types";
import { transformTaskStep } from "@/types/task-step";

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
    currentStep: raw.current_step ? transformTaskStep(raw.current_step) : null,
    nextStep: raw.next_step ? transformTaskStep(raw.next_step) : null,
    percentComplete: raw.percent_complete,
  };
}

/**
 * Transform TeammateSummarySchema (snake_case) → TeammateSummary (camelCase)
 */
export function transformTeammateSummary(
  raw: z.infer<typeof TeammateSummarySchema>
): TeammateSummary {
  return {
    name: raw.name,
    status: raw.status,
    ...(raw.step !== undefined && { step: raw.step }),
    ...(raw.model !== undefined && { model: raw.model }),
    ...(raw.color !== undefined && { color: raw.color }),
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
    ...(raw.team_name !== undefined && { teamName: raw.team_name }),
    ...(raw.teammates !== undefined && {
      teammates: raw.teammates.map(transformTeammateSummary),
    }),
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
