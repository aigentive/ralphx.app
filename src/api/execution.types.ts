// Frontend types for execution API (camelCase)

/**
 * Execution status response - frontend representation (camelCase)
 */
export interface ExecutionStatusResponse {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  queuedCount: number;
  canStartTask: boolean;
}

/**
 * Execution command response - frontend representation (camelCase)
 */
export interface ExecutionCommandResponse {
  success: boolean;
  status: ExecutionStatusResponse;
}

/**
 * Execution settings response - frontend representation (camelCase)
 * Contains persistence settings: max concurrent tasks, auto-commit, pause on failure
 */
export interface ExecutionSettingsResponse {
  maxConcurrentTasks: number;
  autoCommit: boolean;
  pauseOnFailure: boolean;
}

/**
 * Input for updating execution settings - frontend representation (camelCase)
 */
export interface UpdateExecutionSettingsInput {
  maxConcurrentTasks: number;
  autoCommit: boolean;
  pauseOnFailure: boolean;
}
