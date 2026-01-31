// Reviews API module
// Handles review operations and fix task management

import { z } from "zod";
import { typedInvoke } from "@/lib/tauri";
import {
  ReviewResponseSchema,
  ReviewListResponseSchema,
  ReviewNoteListResponseSchema,
  FixTaskAttemptsResponseSchema,
} from "./reviews-api.schemas";

// Re-export schemas and types for consumers
export {
  ReviewResponseSchema,
  ReviewActionResponseSchema,
  ReviewNoteResponseSchema,
  ReviewIssueSchema,
  FixTaskAttemptsResponseSchema,
  ReviewListResponseSchema,
  ReviewNoteListResponseSchema,
  type ReviewResponse,
  type ReviewActionResponse,
  type ReviewNoteResponse,
  type ReviewIssue,
  type FixTaskAttemptsResponse,
} from "./reviews-api.schemas";

// ============================================================================
// Input Types for Review Operations
// ============================================================================

/**
 * Input for approving a review
 */
export interface ApproveReviewInput {
  review_id: string;
  notes?: string;
}

/**
 * Input for requesting changes on a review
 */
export interface RequestChangesInput {
  review_id: string;
  notes: string;
  fix_description?: string;
}

/**
 * Input for rejecting a review
 */
export interface RejectReviewInput {
  review_id: string;
  notes: string;
}

/**
 * Input for approving a fix task
 */
export interface ApproveFixTaskInput {
  fix_task_id: string;
}

/**
 * Input for rejecting a fix task
 */
export interface RejectFixTaskInput {
  fix_task_id: string;
  feedback: string;
  original_task_id: string;
}

/**
 * Input for approving a task (task-based, used by human reviewers)
 */
export interface ApproveTaskInput {
  task_id: string;
  notes?: string;
}

/**
 * Input for requesting changes on a task (task-based, used by human reviewers)
 */
export interface RequestTaskChangesInput {
  task_id: string;
  feedback: string;
}

// ============================================================================
// Reviews API
// ============================================================================

/**
 * Reviews API object containing all review-related Tauri command wrappers
 */
export const reviewsApi = {
  /**
   * Get all pending reviews for a project
   * @param projectId The project ID
   * @returns Array of pending reviews
   */
  getPending: (projectId: string) =>
    typedInvoke("get_pending_reviews", { project_id: projectId }, ReviewListResponseSchema),

  /**
   * Get a single review by ID
   * @param reviewId The review ID
   * @returns The review or null if not found
   */
  getById: (reviewId: string) =>
    typedInvoke("get_review_by_id", { review_id: reviewId }, ReviewResponseSchema.nullable()),

  /**
   * Get all reviews for a task
   * @param taskId The task ID
   * @returns Array of reviews for the task
   */
  getByTaskId: (taskId: string) =>
    typedInvoke("get_reviews_by_task_id", { task_id: taskId }, ReviewListResponseSchema),

  /**
   * Get task state history (review notes)
   * @param taskId The task ID
   * @returns Array of review notes (state transitions)
   */
  getTaskStateHistory: (taskId: string) =>
    typedInvoke("get_task_state_history", { task_id: taskId }, ReviewNoteListResponseSchema),

  /**
   * Approve a pending review
   * @param input Approval input with review_id and optional notes
   * @returns void on success
   */
  approve: (input: ApproveReviewInput) =>
    typedInvoke("approve_review", { input }, z.void()),

  /**
   * Request changes on a pending review
   * @param input Request changes input with review_id, notes, and optional fix_description
   * @returns The created fix task ID if fix_description provided, otherwise null
   */
  requestChanges: (input: RequestChangesInput) =>
    typedInvoke("request_changes", { input }, z.string().nullable()),

  /**
   * Reject a pending review
   * @param input Rejection input with review_id and notes
   * @returns void on success
   */
  reject: (input: RejectReviewInput) =>
    typedInvoke("reject_review", { input }, z.void()),

  /**
   * Approve a task (task-based, for human reviewers)
   * Used when task is in review_passed or escalated state.
   * @param input Approval input with task_id and optional notes
   * @returns void on success
   */
  approveTask: (input: ApproveTaskInput) =>
    typedInvoke("approve_task_for_review", { input }, z.void()),

  /**
   * Request changes on a task (task-based, for human reviewers)
   * Used when task is in review_passed or escalated state.
   * @param input Request changes input with task_id and feedback
   * @returns void on success
   */
  requestTaskChanges: (input: RequestTaskChangesInput) =>
    typedInvoke("request_task_changes_for_review", { input }, z.void()),
} as const;

// ============================================================================
// Fix Tasks API
// ============================================================================

/**
 * Fix Tasks API object containing fix task management Tauri command wrappers
 */
export const fixTasksApi = {
  /**
   * Approve a fix task (allows it to be executed)
   * @param input Approval input with fix_task_id
   * @returns void on success
   */
  approve: (input: ApproveFixTaskInput) =>
    typedInvoke("approve_fix_task", { input }, z.void()),

  /**
   * Reject a fix task with feedback
   * @param input Rejection input with fix_task_id, feedback, and original_task_id
   * @returns The new fix task ID if under max attempts, otherwise null (moved to backlog)
   */
  reject: (input: RejectFixTaskInput) =>
    typedInvoke("reject_fix_task", { input }, z.string().nullable()),

  /**
   * Get the number of fix attempts for a task
   * @param taskId The task ID
   * @returns Fix task attempts response with task_id and attempt_count
   */
  getAttempts: (taskId: string) =>
    typedInvoke("get_fix_task_attempts", { task_id: taskId }, FixTaskAttemptsResponseSchema),
} as const;
