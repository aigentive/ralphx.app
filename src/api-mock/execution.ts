/**
 * Mock Execution API
 *
 * Mirrors the interface of src/api/execution.ts with mock implementations.
 */

import type {
  ExecutionStatusResponse,
  ExecutionCommandResponse,
} from "@/api/execution";

// ============================================================================
// Mock State
// ============================================================================

let mockExecutionState: ExecutionStatusResponse = {
  isPaused: false,
  runningCount: 0,
  maxConcurrent: 3,
  queuedCount: 0,
  canStartTask: true,
};

// ============================================================================
// Mock Execution API
// ============================================================================

export const mockExecutionApi = {
  getStatus: async (): Promise<ExecutionStatusResponse> => {
    return { ...mockExecutionState };
  },

  pause: async (): Promise<ExecutionCommandResponse> => {
    mockExecutionState = { ...mockExecutionState, isPaused: true, canStartTask: false };
    return {
      success: true,
      status: { ...mockExecutionState },
    };
  },

  resume: async (): Promise<ExecutionCommandResponse> => {
    mockExecutionState = { ...mockExecutionState, isPaused: false, canStartTask: true };
    return {
      success: true,
      status: { ...mockExecutionState },
    };
  },

  stop: async (): Promise<ExecutionCommandResponse> => {
    mockExecutionState = {
      isPaused: true,
      runningCount: 0,
      maxConcurrent: 3,
      queuedCount: 0,
      canStartTask: false,
    };
    return {
      success: true,
      status: { ...mockExecutionState },
    };
  },
} as const;
