/**
 * useTaskGraph hook - TanStack Query wrapper for task graph data
 *
 * Fetches task dependency graph for a project using the Tauri API
 * with automatic caching, refetching, and error handling.
 */

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { taskGraphApi, type TaskDependencyGraphResponse } from "@/api/task-graph";
import { useEventBus } from "@/providers/EventProvider";
import { usePlanStore, selectActivePlanId } from "@/stores/planStore";

/**
 * Query key factory for task graph
 */
export const taskGraphKeys = {
  all: ["task-graph"] as const,
  /** Prefix for all graph queries for a project (matches all includeArchived variants) */
  graphPrefix: (projectId: string) => [...taskGraphKeys.all, "graph", projectId] as const,
  graph: (projectId: string, includeArchived?: boolean, executionPlanId?: string | null) =>
    [...taskGraphKeys.graphPrefix(projectId), { includeArchived: includeArchived ?? false, executionPlanId: executionPlanId ?? null }] as const,
};

/**
 * Hook to fetch task dependency graph for a project
 *
 * @param projectId - The project ID to fetch the graph for
 * @param includeArchived - Whether to include archived tasks (default false)
 * @param executionPlanId - Optional execution plan ID to filter tasks by plan (default null)
 * @returns TanStack Query result with graph data
 *
 * @example
 * ```tsx
 * const { data: graph, isLoading, error } = useTaskGraph("project-123");
 *
 * if (isLoading) return <Loading />;
 * if (error) return <Error message={error.message} />;
 * return <GraphCanvas nodes={graph.nodes} edges={graph.edges} />;
 * ```
 */
export function useTaskGraph(
  projectId: string,
  includeArchived: boolean = false,
  executionPlanId: string | null = null
) {
  const queryClient = useQueryClient();
  const eventBus = useEventBus();
  // Guard: if there's an active plan but executionPlanId hasn't resolved yet (loading gap),
  // disable the query to prevent fetching all 750 tasks without a plan filter.
  const activePlanId = usePlanStore(selectActivePlanId(projectId));

  // Subscribe to task updates for real-time graph refresh, debounced to coalesce rapid events.
  useEffect(() => {
    if (!projectId) return;

    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    const unsubscribe = eventBus.subscribe("task:updated", () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        queryClient.invalidateQueries({
          queryKey: taskGraphKeys.graph(projectId, includeArchived, executionPlanId),
        });
      }, 500);
    });

    return () => {
      unsubscribe();
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  }, [projectId, includeArchived, executionPlanId, queryClient, eventBus]);

  return useQuery<TaskDependencyGraphResponse, Error>({
    queryKey: taskGraphKeys.graph(projectId, includeArchived, executionPlanId),
    queryFn: () => taskGraphApi.getDependencyGraph(projectId, includeArchived, executionPlanId),
    // Disable during plan loading gap: activePlanId set but executionPlanId not yet resolved.
    enabled: Boolean(projectId) && !(activePlanId && !executionPlanId),
    // Refetch less frequently since graph structure doesn't change often
    staleTime: 30_000,
  });
}
