/**
 * useProjectStats hook - TanStack Query wrapper for project engineering metrics
 *
 * Fetches all 5 core engineering metrics (throughput, quality, cycle time, EME)
 * for a project in a single query. Automatically re-fetches when tasks change
 * state by listening to the `task:status_changed` event bus event.
 *
 * The backend caches stats for 60s per project, so re-fetches after state
 * changes will return fresh data on the next backend cache miss.
 */

import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { getProjectStats } from "@/api/metrics";
import type { ProjectStats } from "@/types/project-stats";
import type { Unsubscribe } from "@/lib/event-bus";

// ============================================================================
// Query Key Factory
// ============================================================================

export const projectStatsKeys = {
  all: ["projectStats"] as const,
  byProject: (projectId: string) => [...projectStatsKeys.all, projectId] as const,
};

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to fetch engineering metrics for a project.
 *
 * Automatically invalidates the TanStack Query cache when task statuses change
 * so the column header always shows fresh cycle time data without manual refresh.
 *
 * @param projectId - The project ID to fetch stats for (may be undefined)
 * @returns TanStack Query result with project stats
 */
export function useProjectStats(projectId: string | undefined) {
  const queryClient = useQueryClient();
  const bus = useEventBus();

  // Invalidate stats cache whenever any task changes state in this project.
  // The backend's in-process cache is also invalidated on state changes, so the
  // next query will fetch fresh data from SQLite.
  useEffect(() => {
    if (!projectId) return;

    const unsubscribes: Unsubscribe[] = [];

    unsubscribes.push(
      bus.subscribe<{ project_id?: string }>("task:status_changed", (payload) => {
        // Only invalidate if the event is for this project (or if project_id is not provided)
        if (!payload.project_id || payload.project_id === projectId) {
          queryClient.invalidateQueries({
            queryKey: projectStatsKeys.byProject(projectId),
          });
        }
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient, projectId]);

  return useQuery<ProjectStats, Error>({
    queryKey: projectStatsKeys.byProject(projectId ?? ""),
    queryFn: () => getProjectStats(projectId!),
    enabled: !!projectId,
    // Stats are cached on the backend for 60s, so stale time matches that
    staleTime: 60_000,
    // Keep data in memory for 5 minutes after component unmounts
    gcTime: 5 * 60_000,
  });
}
