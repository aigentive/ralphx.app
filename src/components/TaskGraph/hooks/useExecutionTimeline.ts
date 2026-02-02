/**
 * useExecutionTimeline hook - TanStack Query wrapper for timeline events
 *
 * Fetches task execution timeline events for a project using the Tauri API
 * with automatic caching, refetching, and real-time updates.
 *
 * @see specs/plans/task_graph_view.md section "Task D.4"
 */

import { useQuery, useQueryClient, useInfiniteQuery } from "@tanstack/react-query";
import { useEffect, useCallback } from "react";
import { taskGraphApi, type TimelineEventsResponse } from "@/api/task-graph";
import { useEventBus } from "@/providers/EventProvider";

// ============================================================================
// Query Keys
// ============================================================================

/**
 * Query key factory for timeline events
 */
export const timelineKeys = {
  all: ["timeline"] as const,
  events: (projectId: string) => [...timelineKeys.all, "events", projectId] as const,
  eventsWithFilters: (projectId: string, filters: TimelineFilters) =>
    [...timelineKeys.events(projectId), filters] as const,
};

// ============================================================================
// Types
// ============================================================================

/**
 * Filter options for timeline events
 */
export interface TimelineFilters {
  /** Filter by event types (null = all types) */
  eventTypes?: ("status_change" | "plan_accepted" | "plan_completed")[] | null;
  /** Filter by specific task ID */
  taskId?: string | null;
}

/**
 * Options for useExecutionTimeline hook
 */
export interface UseExecutionTimelineOptions {
  /** Number of events per page (default: 50) */
  pageSize?: number;
  /** Filter options */
  filters?: TimelineFilters;
  /** Whether to enable real-time updates (default: true) */
  realTimeUpdates?: boolean;
}

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook to fetch and manage execution timeline events for a project
 *
 * Features:
 * - Fetches timeline events with pagination support
 * - Subscribes to task:updated events for real-time refresh
 * - Client-side filtering by event type
 *
 * @param projectId - The project ID to fetch timeline events for
 * @param options - Configuration options
 * @returns Query result with timeline data and pagination helpers
 *
 * @example
 * ```tsx
 * const {
 *   data,
 *   isLoading,
 *   error,
 *   fetchNextPage,
 *   hasNextPage,
 *   isFetchingNextPage,
 * } = useExecutionTimeline("project-123", {
 *   pageSize: 25,
 *   filters: { eventTypes: ["status_change"] },
 * });
 *
 * // data.pages contains paginated results
 * const allEvents = data?.pages.flatMap(page => page.events) ?? [];
 * ```
 */
export function useExecutionTimeline(
  projectId: string,
  options: UseExecutionTimelineOptions = {}
) {
  const {
    pageSize = 50,
    filters = {},
    realTimeUpdates = true,
  } = options;

  const queryClient = useQueryClient();
  const eventBus = useEventBus();

  // Invalidate timeline query on task updates for real-time refresh
  useEffect(() => {
    if (!projectId || !realTimeUpdates) return;

    const unsubscribe = eventBus.subscribe("task:updated", () => {
      queryClient.invalidateQueries({
        queryKey: timelineKeys.events(projectId),
      });
    });

    return unsubscribe;
  }, [projectId, queryClient, eventBus, realTimeUpdates]);

  // Build query key with filters
  const queryKey = filters && Object.keys(filters).length > 0
    ? timelineKeys.eventsWithFilters(projectId, filters)
    : timelineKeys.events(projectId);

  const query = useInfiniteQuery<TimelineEventsResponse, Error>({
    queryKey,
    queryFn: async ({ pageParam = 0 }) => {
      const response = await taskGraphApi.getTimelineEvents(
        projectId,
        pageSize,
        pageParam as number
      );

      // Apply client-side filters if specified
      if (filters?.eventTypes?.length) {
        return {
          ...response,
          events: response.events.filter((event) =>
            filters.eventTypes!.includes(event.eventType)
          ),
        };
      }

      if (filters?.taskId) {
        return {
          ...response,
          events: response.events.filter(
            (event) => event.taskId === filters.taskId
          ),
        };
      }

      return response;
    },
    getNextPageParam: (lastPage, allPages) => {
      const totalFetched = allPages.reduce(
        (sum, page) => sum + page.events.length,
        0
      );
      return lastPage.hasMore ? totalFetched : undefined;
    },
    initialPageParam: 0,
    enabled: Boolean(projectId),
    staleTime: 15_000, // More frequent updates for timeline
  });

  // Helper to manually refresh timeline
  const refresh = useCallback(() => {
    queryClient.invalidateQueries({
      queryKey: timelineKeys.events(projectId),
    });
  }, [queryClient, projectId]);

  return {
    ...query,
    refresh,
  };
}

/**
 * Simple hook to fetch timeline events without pagination (for smaller displays)
 *
 * @param projectId - The project ID
 * @param limit - Maximum events to fetch (default: 20)
 * @returns TanStack Query result with timeline events
 */
export function useTimelineEvents(projectId: string, limit: number = 20) {
  const queryClient = useQueryClient();
  const eventBus = useEventBus();

  // Subscribe to task updates for real-time refresh
  useEffect(() => {
    if (!projectId) return;

    const unsubscribe = eventBus.subscribe("task:updated", () => {
      queryClient.invalidateQueries({
        queryKey: timelineKeys.events(projectId),
      });
    });

    return unsubscribe;
  }, [projectId, queryClient, eventBus]);

  return useQuery<TimelineEventsResponse, Error>({
    queryKey: [...timelineKeys.events(projectId), { limit }],
    queryFn: () => taskGraphApi.getTimelineEvents(projectId, limit, 0),
    enabled: Boolean(projectId),
    staleTime: 15_000,
  });
}
