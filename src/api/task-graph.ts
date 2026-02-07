// Tauri invoke wrappers for task graph API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  TaskDependencyGraphResponseSchema,
  TimelineEventsResponseSchema,
} from "./task-graph.schemas";
import {
  transformTaskDependencyGraphResponse,
  transformTimelineEventsResponse,
} from "./task-graph.transforms";
import type {
  TaskDependencyGraphResponse,
  TimelineEventsResponse,
} from "./task-graph.types";

// Re-export types for convenience
export type {
  TaskGraphNode,
  TaskGraphEdge,
  StatusSummary,
  PlanGroupInfo,
  TaskDependencyGraphResponse,
  TimelineEventType,
  TimelineEvent,
  TimelineEventsResponse,
} from "./task-graph.types";

// Re-export schemas for consumers that need validation
export {
  TaskGraphNodeSchema,
  TaskGraphEdgeSchema,
  StatusSummarySchema,
  PlanGroupInfoSchema,
  TaskDependencyGraphResponseSchema,
  TimelineEventTypeSchema,
  TimelineEventSchema,
  TimelineEventsResponseSchema,
} from "./task-graph.schemas";

// Re-export transforms for consumers that need manual transformation
export {
  transformTaskGraphNode,
  transformTaskGraphEdge,
  transformStatusSummary,
  transformPlanGroupInfo,
  transformTaskDependencyGraphResponse,
  transformTimelineEvent,
  transformTimelineEventsResponse,
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
   * @param includeArchived - Whether to include archived tasks (default false)
   * @returns Task dependency graph with nodes, edges, plan groups, critical path
   */
  getDependencyGraph: (projectId: string, includeArchived: boolean = false): Promise<TaskDependencyGraphResponse> =>
    typedInvokeWithTransform(
      "get_task_dependency_graph",
      { projectId, includeArchived },
      TaskDependencyGraphResponseSchema,
      transformTaskDependencyGraphResponse
    ),

  /**
   * Get timeline events for the task graph (execution history)
   * @param projectId - The project ID to get events for
   * @param limit - Maximum number of events to return (default: 50)
   * @param offset - Number of events to skip for pagination (default: 0)
   * @returns Timeline events response with events, total count, and pagination info
   */
  getTimelineEvents: (
    projectId: string,
    limit: number = 50,
    offset: number = 0
  ): Promise<TimelineEventsResponse> =>
    typedInvokeWithTransform(
      "get_task_timeline_events",
      { projectId, limit, offset },
      TimelineEventsResponseSchema,
      transformTimelineEventsResponse
    ),
} as const;
