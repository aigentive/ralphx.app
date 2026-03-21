/**
 * Pipeline supervision tool handlers — Flow 3 (Phase 5)
 *
 * 11 tools for monitoring and controlling task pipeline execution.
 * Read-only tools proxy to new backend endpoints.
 * State transition tools proxy through task_transition or review_action endpoints.
 */
import type { ApiKeyContext } from "../types.js";
/**
 * v1_get_task_detail — get full task details + steps.
 * GET /api/external/task/:id
 */
export declare function handleGetTaskDetail(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_task_diff — get git diff stats for a task branch.
 * GET /api/external/task/:id/diff
 */
export declare function handleGetTaskDiff(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_review_summary — get review notes and findings for a task.
 * GET /api/external/task/:id/review_summary
 */
export declare function handleGetReviewSummary(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_approve_review — approve a review, moving it to merge.
 * POST /api/external/review_action
 */
export declare function handleApproveReview(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_request_changes — request changes on a task review with feedback.
 * POST /api/external/review_action
 */
export declare function handleRequestChanges(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_get_merge_pipeline — get all merge activity for a project.
 * GET /api/external/merge_pipeline/:project_id
 */
export declare function handleGetMergePipeline(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_resolve_escalation — handle an escalated review.
 * POST /api/external/review_action
 */
export declare function handleResolveEscalation(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_pause_task — pause a running task.
 * POST /api/external/task_transition
 */
export declare function handlePauseTask(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_cancel_task — cancel a task.
 * POST /api/external/task_transition
 */
export declare function handleCancelTask(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_retry_task — retry a failed or stopped task.
 * POST /api/external/task_transition
 */
export declare function handleRetryTask(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_resume_scheduling — resume a failed accept_plan_and_schedule from its last step.
 * Delegates to resumeScheduling composite.
 */
export declare function handleResumeScheduling(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
/**
 * v1_create_task_note — annotate a task with a progress note.
 * POST /api/external/task-note
 */
export declare function handleCreateTaskNote(args: Record<string, unknown>, context: ApiKeyContext): Promise<string>;
//# sourceMappingURL=pipeline.d.ts.map