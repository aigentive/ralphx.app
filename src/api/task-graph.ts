// Tauri invoke wrappers for task graph API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { TaskDependencyGraphResponseSchema } from "./task-graph.schemas";
import { transformTaskDependencyGraphResponse } from "./task-graph.transforms";
import type { TaskDependencyGraphResponse } from "./task-graph.types";

// Re-export types for convenience
export type {
  TaskGraphNode,
  TaskGraphEdge,
  StatusSummary,
  PlanGroupInfo,
  TaskDependencyGraphResponse,
} from "./task-graph.types";

// Re-export schemas for consumers that need validation
export {
  TaskGraphNodeSchema,
  TaskGraphEdgeSchema,
  StatusSummarySchema,
  PlanGroupInfoSchema,
  TaskDependencyGraphResponseSchema,
} from "./task-graph.schemas";

// Re-export transforms for consumers that need manual transformation
export {
  transformTaskGraphNode,
  transformTaskGraphEdge,
  transformStatusSummary,
  transformPlanGroupInfo,
  transformTaskDependencyGraphResponse,
} from "./task-graph.transforms";

// ============================================================================
// Typed Invoke Helper
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
 * Task graph API wrappers for Tauri commands
 */
export const taskGraphApi = {
  /**
   * Get the task dependency graph for a project
   * @param projectId - The project ID to get the graph for
   * @returns Task dependency graph with nodes, edges, plan groups, critical path
   */
  getDependencyGraph: (projectId: string): Promise<TaskDependencyGraphResponse> =>
    typedInvokeWithTransform(
      "get_task_dependency_graph",
      { projectId },
      TaskDependencyGraphResponseSchema,
      transformTaskDependencyGraphResponse
    ),
} as const;
