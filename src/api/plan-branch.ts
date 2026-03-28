// Tauri invoke wrappers for plan-branch API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  PlanBranchSchema,
  PlanBranchListSchema,
  PlanBranchNullableSchema,
} from "./plan-branch.schemas";
import { transformPlanBranch } from "./plan-branch.transforms";
import type { PlanBranch, EnableFeatureBranchInput } from "./plan-branch.types";

// Re-export types for convenience
export type {
  PlanBranch,
  PlanBranchStatus,
  EnableFeatureBranchInput,
} from "./plan-branch.types";

// Re-export schemas for consumers that need validation
export {
  PlanBranchSchema,
  PlanBranchStatusSchema,
  PlanBranchListSchema,
  PlanBranchNullableSchema,
} from "./plan-branch.schemas";

// Re-export transforms for consumers that need manual transformation
export { transformPlanBranch } from "./plan-branch.transforms";

// ============================================================================
// Typed Invoke Helpers
// ============================================================================

async function typedInvokeWithTransform<TRaw, TResult>(
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
// API Object
// ============================================================================

/**
 * Plan Branch API wrappers for Tauri commands
 * Provides feature branch management for plan groups
 */
export const planBranchApi = {
  /**
   * Get plan branch by plan artifact ID
   * @param planArtifactId - The plan artifact ID to look up
   * @returns PlanBranch or null if none exists
   */
  getByPlan: (planArtifactId: string): Promise<PlanBranch | null> =>
    typedInvokeWithTransform(
      "get_plan_branch",
      { planArtifactId },
      PlanBranchNullableSchema,
      (raw) => (raw ? transformPlanBranch(raw) : null)
    ),

  /**
   * Get all plan branches for a project
   * @param projectId - The project ID to get branches for
   * @returns Array of plan branches
   */
  getByProject: (projectId: string): Promise<PlanBranch[]> =>
    typedInvokeWithTransform(
      "get_project_plan_branches",
      { projectId },
      PlanBranchListSchema,
      (branches) => branches.map(transformPlanBranch)
    ),

  /**
   * Enable feature branch for a plan (mid-plan conversion)
   * Creates git branch, DB record, and merge task with dependencies
   * @param input - Plan artifact ID, session ID, and project ID
   * @returns The created plan branch
   */
  enable: (input: EnableFeatureBranchInput): Promise<PlanBranch> =>
    typedInvokeWithTransform(
      "enable_feature_branch",
      {
        input: {
          plan_artifact_id: input.planArtifactId,
          session_id: input.sessionId,
          project_id: input.projectId,
          ...(input.baseBranchOverride !== undefined && {
            base_branch_override: input.baseBranchOverride,
          }),
        },
      },
      PlanBranchSchema,
      transformPlanBranch
    ),

  /**
   * Get plan branch by merge task ID
   * @param taskId - The merge task ID to look up
   * @returns PlanBranch or null if none exists
   */
  getByTaskId: (taskId: string): Promise<PlanBranch | null> =>
    typedInvokeWithTransform(
      "get_plan_branch_by_task_id",
      { taskId },
      PlanBranchNullableSchema,
      (raw) => (raw ? transformPlanBranch(raw) : null)
    ),
} as const;
