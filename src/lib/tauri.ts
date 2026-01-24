// Tauri invoke wrappers with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  TaskSchema,
  TaskListSchema,
  type CreateTask,
  type UpdateTask,
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
} from "@/types";

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

/**
 * API object containing all typed Tauri command wrappers
 */
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
     * List all tasks for a project
     * @param projectId The project ID
     * @returns Array of tasks
     */
    list: (projectId: string) =>
      typedInvoke("list_tasks", { projectId }, TaskListSchema),

    /**
     * Get a single task by ID
     * @param taskId The task ID
     * @returns The task
     */
    get: (taskId: string) => typedInvoke("get_task", { taskId }, TaskSchema),

    /**
     * Create a new task
     * @param input Task creation data
     * @returns The created task
     */
    create: (input: CreateTask) =>
      typedInvoke("create_task", { input }, TaskSchema),

    /**
     * Update an existing task
     * @param taskId The task ID
     * @param input Partial task data to update
     * @returns The updated task
     */
    update: (taskId: string, input: UpdateTask) =>
      typedInvoke("update_task", { taskId, input }, TaskSchema),

    /**
     * Delete a task
     * @param taskId The task ID
     * @returns true if deleted
     */
    delete: (taskId: string) =>
      typedInvoke("delete_task", { taskId }, z.boolean()),

    /**
     * Move a task to a new status
     * @param taskId The task ID
     * @param toStatus The target status
     * @returns The updated task
     */
    move: (taskId: string, toStatus: string) =>
      typedInvoke("move_task", { taskId, toStatus }, TaskSchema),
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
      typedInvoke("get_workflow", { workflowId }, WorkflowSchemaZ),

    /**
     * List all workflows
     * @returns Array of workflows
     */
    list: () => typedInvoke("list_workflows", {}, WorkflowListSchema),
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
} as const;
