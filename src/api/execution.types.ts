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
