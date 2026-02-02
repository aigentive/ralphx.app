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

/**
 * Query key factory for task graph
 */
export const taskGraphKeys = {
  all: ["task-graph"] as const,
  graph: (projectId: string) => [...taskGraphKeys.all, "graph", projectId] as const,
};

/**
 * Hook to fetch task dependency graph for a project
 *
 * @param projectId - The project ID to fetch the graph for
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
export function useTaskGraph(projectId: string) {
  const queryClient = useQueryClient();
  const eventBus = useEventBus();

  // Subscribe to task updates for real-time graph refresh
  useEffect(() => {
    if (!projectId) return;

    const unsubscribe = eventBus.subscribe("task:updated", () => {
      queryClient.invalidateQueries({
        queryKey: taskGraphKeys.graph(projectId),
      });
    });

    return unsubscribe;
  }, [projectId, queryClient, eventBus]);

  return useQuery<TaskDependencyGraphResponse, Error>({
    queryKey: taskGraphKeys.graph(projectId),
    queryFn: () => taskGraphApi.getDependencyGraph(projectId),
    enabled: Boolean(projectId),
    // Refetch less frequently since graph structure doesn't change often
    staleTime: 30_000,
  });
}
