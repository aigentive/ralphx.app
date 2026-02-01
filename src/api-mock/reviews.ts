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

  getByTaskId: async (taskId: string): Promise<ReviewResponse[]> => {
    // Return mock review for visual testing
    return [
      {
        id: `review-${taskId}`,
        task_id: taskId,
        outcome: "approved",
        comments: null,
        created_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(),
        updated_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(),
      },
    ];
  },

  getTaskStateHistory: async (taskId: string): Promise<ReviewNoteResponse[]> => {
    // Return mock review history for visual testing
    return [
      {
        id: `note-${taskId}-1`,
        task_id: taskId,
        reviewer: "ai",
        outcome: "approved",
        notes: "Code follows project patterns and passes all automated checks. No issues found.",
        created_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(), // 1 hour ago
      },
      {
        id: `note-${taskId}-2`,
        task_id: taskId,
        reviewer: "human",
        outcome: "changes_requested",
        notes: "Please add error handling for edge cases in the validation logic.",
        created_at: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(), // 2 hours ago
      },
    ];
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
