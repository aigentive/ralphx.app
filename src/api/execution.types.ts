// Frontend types for execution API (camelCase)

/**
 * Execution status response - frontend representation (camelCase)
 * Phase 82: Added globalMaxConcurrent for cross-project cap
 */
export interface ExecutionStatusResponse {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  globalMaxConcurrent: number;
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

/**
 * Global execution settings response - frontend representation (Phase 82)
 */
export interface GlobalExecutionSettingsResponse {
  globalMaxConcurrent: number;
}

/**
 * Input for updating global execution settings (Phase 82)
 */
export interface UpdateGlobalExecutionSettingsInput {
  globalMaxConcurrent: number;
}
