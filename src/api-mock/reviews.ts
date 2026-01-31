/**
 * Mock Reviews API
 *
 * Mirrors the interface of src/api/reviews-api.ts with mock implementations.
 */

import type {
  ReviewResponse,
  ReviewNoteResponse,
  FixTaskAttemptsResponse,
} from "@/api/reviews-api.schemas";
import type {
  ApproveReviewInput,
  RequestChangesInput,
  RejectReviewInput,
  ApproveFixTaskInput,
  RejectFixTaskInput,
  ApproveTaskInput,
  RequestTaskChangesInput,
} from "@/api/reviews-api";

// ============================================================================
// Mock Reviews API
// ============================================================================

export const mockReviewsApi = {
  getPending: async (_projectId: string): Promise<ReviewResponse[]> => {
    return [];
  },

  getById: async (_reviewId: string): Promise<ReviewResponse | null> => {
    return null;
  },

  getByTaskId: async (_taskId: string): Promise<ReviewResponse[]> => {
    return [];
  },

  getTaskStateHistory: async (_taskId: string): Promise<ReviewNoteResponse[]> => {
    return [];
  },

  approve: async (_input: ApproveReviewInput): Promise<void> => {
    // No-op in read-only mode
  },

  requestChanges: async (_input: RequestChangesInput): Promise<string | null> => {
    return null;
  },

  reject: async (_input: RejectReviewInput): Promise<void> => {
    // No-op in read-only mode
  },

  approveTask: async (_input: ApproveTaskInput): Promise<void> => {
    // No-op in read-only mode
  },

  requestTaskChanges: async (_input: RequestTaskChangesInput): Promise<void> => {
    // No-op in read-only mode
  },
} as const;

// ============================================================================
// Mock Fix Tasks API
// ============================================================================

export const mockFixTasksApi = {
  approve: async (_input: ApproveFixTaskInput): Promise<void> => {
    // No-op in read-only mode
  },

  reject: async (_input: RejectFixTaskInput): Promise<string | null> => {
    return null;
  },

  getAttempts: async (taskId: string): Promise<FixTaskAttemptsResponse> => {
    return {
      task_id: taskId,
      attempt_count: 0,
    };
  },
} as const;
