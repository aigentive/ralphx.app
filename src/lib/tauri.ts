// Tauri invoke wrappers with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  TaskSchema,
  TaskListSchema,
  TaskListResponseSchema,
  StatusTransitionSchema,
  transformTask,
  transformTaskListResponse,
  type CreateTask,
  type UpdateTask,
  type Task,
  type TaskListResponse,
} from "@/types/task";
import {
  ProjectSchema,
  type CreateProject,
  type UpdateProject,
} from "@/types/project";
import { WorkflowSchemaZ } from "@/types/workflow";
import {
  QASettingsSchema,
  AcceptanceCriteriaTypeSchema,
  QAStepStatusSchema,
  QAOverallStatusSchema,
  ReviewerTypeSchema,
  ReviewStatusSchema,
  ReviewOutcomeSchema,
} from "@/types";
import {
  TaskStepSchema,
  StepProgressSummarySchema,
} from "@/types/task-step";

/**
 * Generic invoke wrapper with runtime Zod validation
 * @param cmd The Tauri command name
 * @param args The arguments to pass to the command
 * @param schema The Zod schema to validate the response
 * @returns The validated response
 * @throws If the response doesn't match the schema
 */
export async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

/**
 * Generic invoke wrapper with runtime Zod validation and transform
 * @param cmd The Tauri command name
 * @param args The arguments to pass to the command
 * @param schema The Zod schema to validate the response (snake_case from backend)
 * @param transform Transform function to convert validated response to camelCase
 * @returns The transformed response
 * @throws If the response doesn't match the schema
 */
export async function typedInvokeWithTransform<TRaw, TResult>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<TRaw>,
  transform: (raw: TRaw) => TResult
): Promise<TResult> {
  const result = await invoke(cmd, args);
  const validated = schema.parse(result);
  return transform(validated);
}

/**
 * Health check response schema
 */
export const HealthResponseSchema = z.object({
  status: z.string(),
});

export type HealthResponse = z.infer<typeof HealthResponseSchema>;

/**
 * Project list schema for array responses
 */
const ProjectListSchema = z.array(ProjectSchema);

/**
 * Workflow list schema for array responses
 */
const WorkflowListSchema = z.array(WorkflowSchemaZ);

// ============================================================================
// QA Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Acceptance criterion response from Rust
 * Note: `type` field is renamed to `criteria_type` in Rust response
 */
export const AcceptanceCriterionResponseSchema = z.object({
  id: z.string(),
  description: z.string(),
  testable: z.boolean(),
  criteria_type: AcceptanceCriteriaTypeSchema,
});

export type AcceptanceCriterionResponse = z.infer<typeof AcceptanceCriterionResponseSchema>;

/**
 * QA test step response from Rust
 */
export const QATestStepResponseSchema = z.object({
  id: z.string(),
  criteria_id: z.string(),
  description: z.string(),
  commands: z.array(z.string()),
  expected: z.string(),
});

export type QATestStepResponse = z.infer<typeof QATestStepResponseSchema>;

/**
 * QA step result response from Rust
 */
export const QAStepResultResponseSchema = z.object({
  step_id: z.string(),
  status: QAStepStatusSchema,
  screenshot: z.string().optional(),
  actual: z.string().optional(),
  expected: z.string().optional(),
  error: z.string().optional(),
});

export type QAStepResultResponse = z.infer<typeof QAStepResultResponseSchema>;

/**
 * QA results response from Rust
 */
export const QAResultsResponseSchema = z.object({
  task_id: z.string(),
  overall_status: QAOverallStatusSchema,
  total_steps: z.number().int().nonnegative(),
  passed_steps: z.number().int().nonnegative(),
  failed_steps: z.number().int().nonnegative(),
  steps: z.array(QAStepResultResponseSchema),
});

export type QAResultsResponse = z.infer<typeof QAResultsResponseSchema>;

/**
 * TaskQA response from Rust - full QA record for a task
 */
export const TaskQAResponseSchema = z.object({
  id: z.string(),
  task_id: z.string(),

  // Phase 1: QA Prep
  acceptance_criteria: z.array(AcceptanceCriterionResponseSchema).optional(),
  qa_test_steps: z.array(QATestStepResponseSchema).optional(),
  prep_agent_id: z.string().optional(),
  prep_started_at: z.string().optional(),
  prep_completed_at: z.string().optional(),

  // Phase 2: QA Refinement
  actual_implementation: z.string().optional(),
  refined_test_steps: z.array(QATestStepResponseSchema).optional(),
  refinement_agent_id: z.string().optional(),
  refinement_completed_at: z.string().optional(),

  // Phase 3: QA Testing
  test_results: QAResultsResponseSchema.optional(),
  screenshots: z.array(z.string()),
  test_agent_id: z.string().optional(),
  test_completed_at: z.string().optional(),

  created_at: z.string(),
});

export type TaskQAResponse = z.infer<typeof TaskQAResponseSchema>;

/**
 * Input type for updating QA settings (partial update)
 */
export interface UpdateQASettingsInput {
  qa_enabled?: boolean;
  auto_qa_for_ui_tasks?: boolean;
  auto_qa_for_api_tasks?: boolean;
  qa_prep_enabled?: boolean;
  browser_testing_enabled?: boolean;
  browser_testing_url?: string;
}

// ============================================================================
// Review Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Review response from Rust
 * Note: field names use snake_case as that's what Rust serde produces
 */
export const ReviewResponseSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  task_id: z.string(),
  reviewer_type: ReviewerTypeSchema,
  status: ReviewStatusSchema,
  notes: z.string().nullable().optional(),
  created_at: z.string(),
  completed_at: z.string().nullable().optional(),
});

export type ReviewResponse = z.infer<typeof ReviewResponseSchema>;

/**
 * Review action response from Rust
 */
export const ReviewActionResponseSchema = z.object({
  id: z.string(),
  review_id: z.string(),
  action_type: z.string(),
  target_task_id: z.string().nullable().optional(),
  created_at: z.string(),
});

export type ReviewActionResponse = z.infer<typeof ReviewActionResponseSchema>;

/**
 * Review note response from Rust (state history)
 */
export const ReviewNoteResponseSchema = z.object({
  id: z.string(),
  task_id: z.string(),
  reviewer: ReviewerTypeSchema,
  outcome: ReviewOutcomeSchema,
  notes: z.string().nullable().optional(),
  created_at: z.string(),
});

export type ReviewNoteResponse = z.infer<typeof ReviewNoteResponseSchema>;

/**
 * Fix task attempts response from Rust
 */
export const FixTaskAttemptsResponseSchema = z.object({
  task_id: z.string(),
  attempt_count: z.number().int().nonnegative(),
});

export type FixTaskAttemptsResponse = z.infer<typeof FixTaskAttemptsResponseSchema>;

/**
 * List schemas for array responses
 */
const ReviewListResponseSchema = z.array(ReviewResponseSchema);
const ReviewNoteListResponseSchema = z.array(ReviewNoteResponseSchema);
const TaskStepListSchema = z.array(TaskStepSchema);

/**
 * Input types for review operations
 */
export interface ApproveReviewInput {
  review_id: string;
  notes?: string;
}

export interface RequestChangesInput {
  review_id: string;
  notes: string;
  fix_description?: string;
}

export interface RejectReviewInput {
  review_id: string;
  notes: string;
}

export interface ApproveFixTaskInput {
  fix_task_id: string;
}

export interface RejectFixTaskInput {
  fix_task_id: string;
  feedback: string;
  original_task_id: string;
}

// ============================================================================
// Execution Control Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Execution status response from Rust (snake_case)
 * Backend outputs snake_case by default (no rename_all annotation)
 */
export const ExecutionStatusResponseSchema = z.object({
  is_paused: z.boolean(),
  running_count: z.number().int().nonnegative(),
  max_concurrent: z.number().int().nonnegative(),
  queued_count: z.number().int().nonnegative(),
  can_start_task: z.boolean(),
});

/**
 * Frontend representation with camelCase (after transform)
 */
export interface ExecutionStatusResponse {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  queuedCount: number;
  canStartTask: boolean;
}

/**
 * Transform ExecutionStatusResponseSchema (snake_case) → ExecutionStatusResponse (camelCase)
 */
export function transformExecutionStatus(
  raw: z.infer<typeof ExecutionStatusResponseSchema>
): ExecutionStatusResponse {
  return {
    isPaused: raw.is_paused,
    runningCount: raw.running_count,
    maxConcurrent: raw.max_concurrent,
    queuedCount: raw.queued_count,
    canStartTask: raw.can_start_task,
  };
}

/**
 * Execution command response from Rust (for pause/resume/stop) (snake_case)
 */
export const ExecutionCommandResponseSchema = z.object({
  success: z.boolean(),
  status: ExecutionStatusResponseSchema,
});

/**
 * Frontend representation with camelCase status
 */
export interface ExecutionCommandResponse {
  success: boolean;
  status: ExecutionStatusResponse;
}

/**
 * Transform ExecutionCommandResponseSchema → ExecutionCommandResponse
 */
export function transformExecutionCommand(
  raw: z.infer<typeof ExecutionCommandResponseSchema>
): ExecutionCommandResponse {
  return {
    success: raw.success,
    status: transformExecutionStatus(raw.status),
  };
}

// ============================================================================
// Task Injection Response Schemas (matching Rust responses)
// ============================================================================

/**
 * Inject task response from Rust
 * Note: field names use camelCase as that's what Rust serde produces with rename_all
 */
export const InjectTaskResponseSchema = z.object({
  task: TaskSchema,
  target: z.enum(["backlog", "planned"]),
  priority: z.number().int(),
  makeNextApplied: z.boolean(),
});

export type InjectTaskResponse = z.infer<typeof InjectTaskResponseSchema>;

/**
 * Input type for injecting a task mid-loop
 */
export interface InjectTaskInput {
  /** The project ID to inject the task into */
  projectId: string;
  /** Title of the task */
  title: string;
  /** Optional description */
  description?: string;
  /** Category (defaults to "feature") */
  category?: string;
  /** Where to inject: "backlog" (deferred) or "planned" (immediate queue) */
  target?: "backlog" | "planned";
  /** If true and target is "planned", make this task the highest priority */
  makeNext?: boolean;
}

/**
 * API object containing all typed Tauri command wrappers
 */
/**
 * Get git branches for a working directory
 * @param workingDirectory The path to the git repository
 * @returns Array of branch names (main/master sorted first)
 */
export async function getGitBranches(workingDirectory: string): Promise<string[]> {
  const result = await invoke<string[]>("get_git_branches", { workingDirectory });
  return result;
}

export const api = {
  health: {
    /**
     * Check if the backend is running
     * @returns { status: "ok" } if healthy
     */
    check: () => typedInvoke("health_check", {}, HealthResponseSchema),
  },

  tasks: {
    /**
     * List all tasks for a project with optional pagination and filtering
     * @param params Parameters for listing tasks
     * @param params.projectId The project ID
     * @param params.statuses Optional status filter (array of statuses, matches any)
     * @param params.offset Optional pagination offset (default 0)
     * @param params.limit Optional pagination limit (default 20)
     * @param params.includeArchived Optional flag to include archived tasks (default false)
     * @returns Paginated task list response
     */
    list: (params: {
      projectId: string;
      statuses?: string[];
      offset?: number;
      limit?: number;
      includeArchived?: boolean;
    }): Promise<TaskListResponse> =>
      typedInvokeWithTransform("list_tasks", params, TaskListResponseSchema, transformTaskListResponse),

    /**
     * Search tasks by query string
     * @param projectId The project ID
     * @param query The search query (searches title and description)
     * @param includeArchived Optional flag to include archived tasks (default false)
     * @returns Array of matching tasks
     */
    search: (projectId: string, query: string, includeArchived?: boolean): Promise<Task[]> =>
      typedInvokeWithTransform(
        "search_tasks",
        { projectId, query, includeArchived },
        TaskListSchema,
        (tasks) => tasks.map(transformTask)
      ),

    /**
     * Get a single task by ID
     * @param taskId The task ID
     * @returns The task
     */
    get: (taskId: string): Promise<Task> =>
      typedInvokeWithTransform("get_task", { id: taskId }, TaskSchema, transformTask),

    /**
     * Create a new task
     * @param input Task creation data
     * @returns The created task
     */
    create: (input: CreateTask): Promise<Task> =>
      typedInvokeWithTransform("create_task", { input }, TaskSchema, transformTask),

    /**
     * Update an existing task
     * @param taskId The task ID
     * @param input Partial task data to update
     * @returns The updated task
     */
    update: (taskId: string, input: UpdateTask): Promise<Task> =>
      typedInvokeWithTransform("update_task", { taskId, input }, TaskSchema, transformTask),

    /**
     * Delete a task
     * @param taskId The task ID
     * @returns true if deleted
     */
    delete: (taskId: string) =>
      typedInvoke("delete_task", { taskId }, z.boolean()),

    /**
     * Archive a task (soft delete)
     * @param taskId The task ID
     * @returns The archived task
     */
    archive: (taskId: string): Promise<Task> =>
      typedInvokeWithTransform("archive_task", { taskId }, TaskSchema, transformTask),

    /**
     * Restore an archived task
     * @param taskId The task ID
     * @returns The restored task
     */
    restore: (taskId: string): Promise<Task> =>
      typedInvokeWithTransform("restore_task", { taskId }, TaskSchema, transformTask),

    /**
     * Permanently delete a task (only works on archived tasks)
     * @param taskId The task ID
     * @returns void on success
     */
    permanentlyDelete: (taskId: string) =>
      typedInvoke("permanently_delete_task", { taskId }, z.void()),

    /**
     * Get count of archived tasks for a project
     * @param projectId The project ID
     * @returns Count of archived tasks
     */
    getArchivedCount: (projectId: string) =>
      typedInvoke("get_archived_count", { projectId }, z.number()),

    /**
     * Get valid status transitions for a task
     * @param taskId The task ID
     * @returns Array of valid status transitions
     */
    getValidTransitions: (taskId: string) =>
      typedInvoke(
        "get_valid_transitions",
        { taskId },
        z.array(StatusTransitionSchema)
      ),

    /**
     * Move a task to a new status
     * @param taskId The task ID
     * @param toStatus The target status
     * @returns The updated task
     */
    move: (taskId: string, toStatus: string): Promise<Task> =>
      typedInvokeWithTransform("move_task", { taskId, toStatus }, TaskSchema, transformTask),

    /**
     * Inject a task mid-loop
     * Tasks can be sent to backlog (deferred) or planned (immediate queue).
     * If makeNext is true and target is "planned", the task gets the highest priority.
     * Emits a task:created event on success.
     * @param input Inject task input
     * @returns The inject task response with created task and injection details
     */
    inject: (input: InjectTaskInput) =>
      typedInvoke("inject_task", { input }, InjectTaskResponseSchema),
  },

  projects: {
    /**
     * List all projects
     * @returns Array of projects
     */
    list: () => typedInvoke("list_projects", {}, ProjectListSchema),

    /**
     * Get a single project by ID
     * @param projectId The project ID
     * @returns The project
     */
    get: (projectId: string) =>
      typedInvoke("get_project", { projectId }, ProjectSchema),

    /**
     * Create a new project
     * @param input Project creation data
     * @returns The created project
     */
    create: (input: CreateProject) =>
      typedInvoke("create_project", { input }, ProjectSchema),

    /**
     * Update an existing project
     * @param projectId The project ID
     * @param input Partial project data to update
     * @returns The updated project
     */
    update: (projectId: string, input: UpdateProject) =>
      typedInvoke("update_project", { projectId, input }, ProjectSchema),

    /**
     * Delete a project
     * @param projectId The project ID
     * @returns true if deleted
     */
    delete: (projectId: string) =>
      typedInvoke("delete_project", { projectId }, z.boolean()),
  },

  workflows: {
    /**
     * Get a workflow by ID
     * @param workflowId The workflow ID
     * @returns The workflow
     */
    get: (workflowId: string) =>
      typedInvoke("get_workflow", { id: workflowId }, WorkflowSchemaZ.nullable()).then(
        (result) => {
          if (!result) throw new Error(`Workflow not found: ${workflowId}`);
          return result;
        }
      ),

    /**
     * List all workflows
     * @returns Array of workflows
     */
    list: () => typedInvoke("list_workflows", {}, WorkflowListSchema),

    /**
     * Seed builtin workflows if they don't exist
     * @returns Number of workflows created
     */
    seedBuiltin: () => typedInvoke("seed_builtin_workflows", {}, z.number()),
  },

  qa: {
    /**
     * Get global QA settings
     * @returns The current QA settings
     */
    getSettings: () => typedInvoke("get_qa_settings", {}, QASettingsSchema),

    /**
     * Update global QA settings
     * @param input Partial settings to update
     * @returns The updated QA settings
     */
    updateSettings: (input: UpdateQASettingsInput) =>
      typedInvoke("update_qa_settings", { input }, QASettingsSchema),

    /**
     * Get TaskQA data for a specific task
     * @param taskId The task ID
     * @returns TaskQA record or null if none exists
     */
    getTaskQA: (taskId: string) =>
      typedInvoke(
        "get_task_qa",
        { taskId },
        TaskQAResponseSchema.nullable()
      ),

    /**
     * Get QA test results for a specific task
     * @param taskId The task ID
     * @returns QA results or null if no results yet
     */
    getResults: (taskId: string) =>
      typedInvoke(
        "get_qa_results",
        { taskId },
        QAResultsResponseSchema.nullable()
      ),

    /**
     * Retry QA tests for a task
     * Resets test results to pending for re-testing
     * @param taskId The task ID
     * @returns Updated TaskQA record
     */
    retry: (taskId: string) =>
      typedInvoke("retry_qa", { taskId }, TaskQAResponseSchema),

    /**
     * Skip QA for a task
     * Marks all test steps as skipped to bypass QA failure
     * @param taskId The task ID
     * @returns Updated TaskQA record
     */
    skip: (taskId: string) =>
      typedInvoke("skip_qa", { taskId }, TaskQAResponseSchema),
  },

  reviews: {
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
  },

  fixTasks: {
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
  },

  execution: {
    /**
     * Get current execution status
     * @returns Execution status with pause state, running count, queued count
     */
    getStatus: () =>
      typedInvokeWithTransform(
        "get_execution_status",
        {},
        ExecutionStatusResponseSchema,
        transformExecutionStatus
      ),

    /**
     * Pause execution (stops picking up new tasks)
     * @returns Command response with success and current status
     */
    pause: () =>
      typedInvokeWithTransform(
        "pause_execution",
        {},
        ExecutionCommandResponseSchema,
        transformExecutionCommand
      ),

    /**
     * Resume execution (allows picking up new tasks)
     * @returns Command response with success and current status
     */
    resume: () =>
      typedInvokeWithTransform(
        "resume_execution",
        {},
        ExecutionCommandResponseSchema,
        transformExecutionCommand
      ),

    /**
     * Stop execution (cancels current tasks and pauses)
     * @returns Command response with success and current status
     */
    stop: () =>
      typedInvokeWithTransform(
        "stop_execution",
        {},
        ExecutionCommandResponseSchema,
        transformExecutionCommand
      ),
  },

  steps: {
    /**
     * Get all steps for a task
     * @param taskId The task ID
     * @returns Array of task steps
     */
    getByTask: (taskId: string) =>
      typedInvoke("get_task_steps", { taskId }, TaskStepListSchema),

    /**
     * Create a new task step
     * @param taskId The task ID
     * @param data Step creation data (title, description, sortOrder)
     * @returns The created task step
     */
    create: (
      taskId: string,
      data: { title: string; description?: string; sortOrder?: number }
    ) =>
      typedInvoke("create_task_step", { taskId, ...data }, TaskStepSchema),

    /**
     * Update an existing task step
     * @param stepId The step ID
     * @param data Partial step data to update (title, description, sortOrder)
     * @returns The updated task step
     */
    update: (
      stepId: string,
      data: { title?: string; description?: string; sortOrder?: number }
    ) =>
      typedInvoke("update_task_step", { stepId, ...data }, TaskStepSchema),

    /**
     * Delete a task step
     * @param stepId The step ID
     * @returns void on success
     */
    delete: (stepId: string) =>
      typedInvoke("delete_task_step", { stepId }, z.void()),

    /**
     * Reorder task steps
     * @param taskId The task ID
     * @param stepIds Array of step IDs in desired order
     * @returns Array of reordered task steps
     */
    reorder: (taskId: string, stepIds: string[]) =>
      typedInvoke("reorder_task_steps", { taskId, stepIds }, TaskStepListSchema),

    /**
     * Get step progress summary for a task
     * @param taskId The task ID
     * @returns Step progress summary with counts and percentages
     */
    getProgress: (taskId: string) =>
      typedInvoke("get_step_progress", { taskId }, StepProgressSummarySchema),

    /**
     * Start a task step (marks as in_progress)
     * @param stepId The step ID
     * @returns The updated task step
     */
    start: (stepId: string) =>
      typedInvoke("start_step", { stepId }, TaskStepSchema),

    /**
     * Complete a task step (marks as completed)
     * @param stepId The step ID
     * @param note Optional completion note
     * @returns The updated task step
     */
    complete: (stepId: string, note?: string) =>
      typedInvoke("complete_step", { stepId, note }, TaskStepSchema),

    /**
     * Skip a task step (marks as skipped)
     * @param stepId The step ID
     * @param reason Reason for skipping
     * @returns The updated task step
     */
    skip: (stepId: string, reason: string) =>
      typedInvoke("skip_step", { stepId, reason }, TaskStepSchema),

    /**
     * Fail a task step (marks as failed)
     * @param stepId The step ID
     * @param error Error message
     * @returns The updated task step
     */
    fail: (stepId: string, error: string) =>
      typedInvoke("fail_step", { stepId, error }, TaskStepSchema),
  },

  testData: {
    /**
     * Seed test data with specified profile
     * @param profile - "minimal" | "kanban" | "ideation" | "full" (default: kanban)
     * @returns Seed response with counts
     */
    seed: (profile?: "minimal" | "kanban" | "ideation" | "full") =>
      typedInvoke(
        "seed_test_data",
        { profile },
        z.object({
          profile: z.string(),
          projectId: z.string(),
          projectName: z.string(),
          tasksCreated: z.number(),
          sessionsCreated: z.number(),
          proposalsCreated: z.number(),
        })
      ),

    /**
     * Seed demo data for visual audits (alias for seed("kanban"))
     * Creates a test project with sample tasks in various states
     * @returns Seed response with project info and task count
     */
    seedVisualAudit: () =>
      typedInvoke(
        "seed_visual_audit_data",
        {},
        z.object({
          profile: z.string(),
          projectId: z.string(),
          projectName: z.string(),
          tasksCreated: z.number(),
          sessionsCreated: z.number(),
          proposalsCreated: z.number(),
        })
      ),

    /**
     * Clear all test data
     * @returns Confirmation message
     */
    clear: () => typedInvoke("clear_test_data", {}, z.string()),
  },
} as const;
