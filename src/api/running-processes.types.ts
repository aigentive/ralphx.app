// Frontend types for running processes API (camelCase)

import type { TaskStep } from "@/types/task-step";

/**
 * Step progress summary - frontend representation (camelCase)
 */
export interface StepProgressSummary {
  taskId: string;
  total: number;
  completed: number;
  inProgress: number;
  pending: number;
  skipped: number;
  failed: number;
  currentStep: TaskStep | null;
  nextStep: TaskStep | null;
  percentComplete: number;
}

/**
 * Teammate info within a team process group
 */
export interface TeammateSummary {
  name: string;
  status: string;
  step?: string;
  model?: string;
  color?: string;
}

/**
 * Running process - frontend representation (camelCase)
 */
export interface RunningProcess {
  taskId: string;
  title: string;
  internalStatus: string;
  stepProgress: StepProgressSummary | null;
  elapsedSeconds: number | null;
  triggerOrigin: string | null;
  taskBranch: string | null;
  teamName?: string;
  teammates?: TeammateSummary[];
}

/**
 * Running processes response - frontend representation (camelCase)
 */
export interface RunningProcessesResponse {
  processes: RunningProcess[];
}
