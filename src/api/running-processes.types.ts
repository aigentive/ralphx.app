// Frontend types for running processes API (camelCase)

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
  currentStep: unknown | null; // TaskStep - nullable
  nextStep: unknown | null; // TaskStep - nullable
  percentComplete: number;
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
}

/**
 * Running processes response - frontend representation (camelCase)
 */
export interface RunningProcessesResponse {
  processes: RunningProcess[];
}
