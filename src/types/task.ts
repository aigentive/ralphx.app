// Task types and Zod schema
// Must match the Rust backend Task struct

import { z } from "zod";
import { InternalStatusSchema, type InternalStatus } from "./status";

// Re-export InternalStatus for convenience
export type { InternalStatus };

/**
 * Task schema matching Rust backend serialization (snake_case)
 * Backend outputs snake_case (Rust default). Transform layer converts to camelCase for UI.
 *
 * Note: Backend TaskResponse does NOT include source_proposal_id or plan_artifact_id.
 * Those fields are in the Task entity but not exposed in API responses.
 * Use get_task_context to fetch them if needed.
 */
export const TaskSchema = z.object({
  id: z.string().min(1),
  project_id: z.string().min(1),
  category: z.string().min(1),
  title: z.string().min(1),
  description: z.string().nullable(),
  priority: z.number().int(),
  internal_status: InternalStatusSchema,
  /** Whether this task needs a review point (human-in-loop checkpoint) */
  needs_review_point: z.boolean().default(false),
  // Accept RFC3339 timestamps with offset (e.g., +00:00)
  created_at: z.string().datetime({ offset: true }),
  updated_at: z.string().datetime({ offset: true }),
  started_at: z.string().datetime({ offset: true }).nullable(),
  completed_at: z.string().datetime({ offset: true }).nullable(),
  archived_at: z.string().datetime({ offset: true }).nullable(),
  blocked_reason: z.string().nullable(),
  // Git branch isolation fields (Phase 66)
  task_branch: z.string().nullable().optional(),
  worktree_path: z.string().nullable().optional(),
  merge_commit_sha: z.string().nullable().optional(),
  metadata: z.string().nullable().optional(),
});

/**
 * Frontend Task type (camelCase)
 * This is what components and stores use. Transformed from snake_case API responses.
 *
 * Note: sourceProposalId and planArtifactId are not included in TaskResponse.
 * They exist in the database but aren't serialized in API responses.
 * These fields are undefined unless explicitly populated from TaskContext.
 */
export interface Task {
  id: string;
  projectId: string;
  category: string;
  title: string;
  description: string | null;
  priority: number;
  internalStatus: InternalStatus;
  needsReviewPoint: boolean;
  createdAt: string;
  updatedAt: string;
  startedAt: string | null;
  completedAt: string | null;
  archivedAt: string | null;
  blockedReason: string | null;
  /** Not in TaskResponse - fetch via get_task_context */
  sourceProposalId?: string | null;
  /** Not in TaskResponse - fetch via get_task_context */
  planArtifactId?: string | null;
  // Git branch isolation fields (Phase 66)
  /** Branch name for this task (both Local and Worktree modes) */
  taskBranch?: string | null;
  /** Worktree path for this task (Worktree mode only) */
  worktreePath?: string | null;
  /** Merge commit SHA after successful merge */
  mergeCommitSha?: string | null;
  /** Task metadata as JSON string (e.g., conflict_files for merge states) */
  metadata?: string | null;
}

/**
 * Transform function to convert snake_case API response to camelCase frontend type
 */
export function transformTask(raw: z.infer<typeof TaskSchema>): Task {
  return {
    id: raw.id,
    projectId: raw.project_id,
    category: raw.category,
    title: raw.title,
    description: raw.description,
    priority: raw.priority,
    internalStatus: raw.internal_status,
    needsReviewPoint: raw.needs_review_point,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    startedAt: raw.started_at,
    completedAt: raw.completed_at,
    archivedAt: raw.archived_at,
    blockedReason: raw.blocked_reason,
    taskBranch: raw.task_branch ?? null,
    worktreePath: raw.worktree_path ?? null,
    mergeCommitSha: raw.merge_commit_sha ?? null,
    metadata: raw.metadata ?? null,
  };
}

/**
 * Common task categories
 */
export const TASK_CATEGORIES = [
  "feature",
  "bug",
  "chore",
  "docs",
  "test",
  "refactor",
] as const;

export const TaskCategorySchema = z.enum(TASK_CATEGORIES);
export type TaskCategory = z.infer<typeof TaskCategorySchema>;

/**
 * Schema for creating a new task
 * Excludes auto-generated fields (id, timestamps, status)
 */
export const CreateTaskSchema = z.object({
  projectId: z.string().min(1, "Project ID is required"),
  category: z.string().min(1, "Category is required").default("feature"),
  title: z.string().min(1, "Title is required"),
  description: z.string().optional(),
  priority: z.number().int().default(0),
  /** Override for QA enablement. null means inherit from global settings. */
  needsQa: z.boolean().nullable().optional(),
  /** Optional list of step titles to create for this task */
  steps: z.array(z.string()).optional(),
});

export type CreateTask = z.infer<typeof CreateTaskSchema>;

/**
 * Schema for updating a task
 * All fields are optional
 */
export const UpdateTaskSchema = z.object({
  category: z.string().min(1).optional(),
  title: z.string().min(1).optional(),
  description: z.string().nullable().optional(),
  priority: z.number().int().optional(),
});

export type UpdateTask = z.infer<typeof UpdateTaskSchema>;

/**
 * Schema for task list response (backend uses array)
 */
export const TaskListSchema = z.array(TaskSchema);
export type TaskList = Task[];

/**
 * Schema for paginated task list response (snake_case from backend)
 */
export const TaskListResponseSchema = z.object({
  tasks: z.array(TaskSchema),
  total: z.number(),
  has_more: z.boolean(),
  offset: z.number(),
});

/**
 * Frontend TaskListResponse type (camelCase)
 */
export interface TaskListResponse {
  tasks: Task[];
  total: number;
  hasMore: boolean;
  offset: number;
}

/**
 * Transform function for TaskListResponse
 */
export function transformTaskListResponse(raw: z.infer<typeof TaskListResponseSchema>): TaskListResponse {
  return {
    tasks: raw.tasks.map(transformTask),
    total: raw.total,
    hasMore: raw.has_more,
    offset: raw.offset,
  };
}

/**
 * Schema for status transition option
 */
export const StatusTransitionSchema = z.object({
  status: z.string(),
  label: z.string(),
});
export type StatusTransition = z.infer<typeof StatusTransitionSchema>;
