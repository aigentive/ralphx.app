// Review Issues API module
// Handles review issue tracking and lifecycle management

import { typedInvokeWithTransform } from "@/lib/tauri";
import {
  ReviewIssueResponseSchema,
  ReviewIssueListResponseSchema,
  IssueProgressSummaryResponseSchema,
  transformReviewIssue,
  transformReviewIssueList,
  transformIssueProgressSummary,
  type ReviewIssue,
  type IssueProgressSummary,
} from "@/types/review-issue";

// Re-export types for consumers
export type { ReviewIssue, IssueProgressSummary } from "@/types/review-issue";
export {
  type IssueStatus,
  type IssueSeverity,
  type IssueCategory,
  isIssueOpen,
  isIssueInProgress,
  isIssueAddressed,
  isIssueVerified,
  isIssueWontFix,
  isIssueTerminal,
  isIssueResolved,
  isIssueNeedsWork,
  getSeverityPriority,
  isSeverityBlocking,
  sortBySeverity,
  isCodeIssue,
  isRequirementsIssue,
} from "@/types/review-issue";

// ============================================================================
// Input Types for Review Issue Operations
// ============================================================================

/**
 * Input for verifying an issue
 */
export interface VerifyIssueInput {
  issue_id: string;
  review_note_id: string;
}

/**
 * Input for reopening an issue
 */
export interface ReopenIssueInput {
  issue_id: string;
  reason?: string;
}

/**
 * Input for marking an issue as in progress
 */
export interface MarkIssueInProgressInput {
  issue_id: string;
}

/**
 * Input for marking an issue as addressed
 */
export interface MarkIssueAddressedInput {
  issue_id: string;
  resolution_notes: string;
  attempt_number: number;
}

/**
 * Status filter for listing issues
 */
export type IssueStatusFilter = "open" | "all";

// ============================================================================
// Review Issues API
// ============================================================================

/**
 * Review Issues API object containing all issue-related Tauri command wrappers
 */
export const reviewIssuesApi = {
  /**
   * Get all issues for a task with optional status filter
   * @param taskId The task ID
   * @param statusFilter Optional filter: "open" for open issues only, "all" for all issues (default: "all")
   * @returns Array of review issues
   */
  getByTaskId: (
    taskId: string,
    statusFilter?: IssueStatusFilter
  ): Promise<ReviewIssue[]> =>
    typedInvokeWithTransform(
      "get_task_issues",
      {
        task_id: taskId,
        status_filter: statusFilter ?? null,
      },
      ReviewIssueListResponseSchema,
      transformReviewIssueList
    ),

  /**
   * Get issue progress summary for a task
   * @param taskId The task ID
   * @returns Issue progress summary with counts by status and severity
   */
  getProgress: (taskId: string): Promise<IssueProgressSummary> =>
    typedInvokeWithTransform(
      "get_issue_progress",
      { task_id: taskId },
      IssueProgressSummaryResponseSchema,
      transformIssueProgressSummary
    ),

  /**
   * Verify an issue (mark as verified after re-review)
   * @param input Verification input with issue_id and review_note_id
   * @returns The updated review issue
   */
  verify: (input: VerifyIssueInput): Promise<ReviewIssue> =>
    typedInvokeWithTransform(
      "verify_issue",
      { input },
      ReviewIssueResponseSchema,
      transformReviewIssue
    ),

  /**
   * Reopen an issue (issue not actually fixed)
   * @param input Reopen input with issue_id and optional reason
   * @returns The updated review issue
   */
  reopen: (input: ReopenIssueInput): Promise<ReviewIssue> =>
    typedInvokeWithTransform(
      "reopen_issue",
      { input },
      ReviewIssueResponseSchema,
      transformReviewIssue
    ),

  /**
   * Mark an issue as in progress (worker starting work)
   * @param input Mark in progress input with issue_id
   * @returns The updated review issue
   */
  markInProgress: (input: MarkIssueInProgressInput): Promise<ReviewIssue> =>
    typedInvokeWithTransform(
      "mark_issue_in_progress",
      { input },
      ReviewIssueResponseSchema,
      transformReviewIssue
    ),

  /**
   * Mark an issue as addressed (worker completed work)
   * @param input Mark addressed input with issue_id, resolution_notes, and attempt_number
   * @returns The updated review issue
   */
  markAddressed: (input: MarkIssueAddressedInput): Promise<ReviewIssue> =>
    typedInvokeWithTransform(
      "mark_issue_addressed",
      { input },
      ReviewIssueResponseSchema,
      transformReviewIssue
    ),
} as const;
