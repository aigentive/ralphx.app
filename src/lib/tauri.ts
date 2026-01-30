// Tauri invoke wrappers with type safety using Zod schemas
// This file serves as the main entry point and re-exports domain-specific API modules

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Core Utilities
// ============================================================================

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

// ============================================================================
// Health Check (Universal)
// ============================================================================

/**
 * Health check response schema
 */
export const HealthResponseSchema = z.object({
  status: z.string(),
});

export type HealthResponse = z.infer<typeof HealthResponseSchema>;

// ============================================================================
// Re-exports from Domain API Modules
// ============================================================================

// Execution API
export {
  executionApi,
  type ExecutionStatusResponse,
  type ExecutionCommandResponse,
  ExecutionStatusResponseSchema,
  ExecutionCommandResponseSchema,
  transformExecutionStatus,
  transformExecutionCommand,
} from "@/api/execution";

// Test Data API
export {
  testDataApi,
  type SeedResponse,
  type TestDataProfile,
} from "@/api/test-data";

// Projects API
export {
  projectsApi,
  workflowsApi,
  getGitBranches,
} from "@/api/projects";

// QA API
export {
  qaApi,
  type UpdateQASettingsInput,
  AcceptanceCriterionResponseSchema,
  QATestStepResponseSchema,
  QAStepResultResponseSchema,
  QAResultsResponseSchema,
  TaskQAResponseSchema,
  type AcceptanceCriterionResponse,
  type QATestStepResponse,
  type QAStepResultResponse,
  type QAResultsResponse,
  type TaskQAResponse,
} from "@/api/qa-api";

// Reviews API
export {
  reviewsApi,
  fixTasksApi,
  type ApproveReviewInput,
  type RequestChangesInput,
  type RejectReviewInput,
  type ApproveFixTaskInput,
  type RejectFixTaskInput,
  ReviewResponseSchema,
  ReviewActionResponseSchema,
  ReviewNoteResponseSchema,
  FixTaskAttemptsResponseSchema,
  ReviewListResponseSchema,
  ReviewNoteListResponseSchema,
  type ReviewResponse,
  type ReviewActionResponse,
  type ReviewNoteResponse,
  type FixTaskAttemptsResponse,
} from "@/api/reviews-api";

// Tasks API
export {
  tasksApi,
  stepsApi,
  type InjectTaskInput,
  type InjectTaskResponse,
  InjectTaskResponseSchemaRaw,
  transformInjectTaskResponse,
} from "@/api/tasks";

// ============================================================================
// Aggregate API Object
// ============================================================================

import { executionApi } from "@/api/execution";
import { testDataApi } from "@/api/test-data";
import { projectsApi, workflowsApi } from "@/api/projects";
import { qaApi } from "@/api/qa-api";
import { reviewsApi, fixTasksApi } from "@/api/reviews-api";
import { tasksApi, stepsApi } from "@/api/tasks";

/**
 * Aggregate API object containing all typed Tauri command wrappers
 * This provides backward compatibility for existing imports of `api`
 */
export const api = {
  health: {
    /**
     * Check if the backend is running
     * @returns { status: "ok" } if healthy
     */
    check: () => typedInvoke("health_check", {}, HealthResponseSchema),
  },

  tasks: tasksApi,
  projects: projectsApi,
  workflows: workflowsApi,
  qa: qaApi,
  reviews: reviewsApi,
  fixTasks: fixTasksApi,
  execution: executionApi,
  steps: stepsApi,
  testData: testDataApi,
} as const;
