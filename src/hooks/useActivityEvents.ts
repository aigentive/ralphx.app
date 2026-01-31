/**
 * useActivityEvents hooks - TanStack Query wrappers for activity events with infinite scroll
 *
 * Provides cursor-based pagination using TanStack Query's useInfiniteQuery.
 * Supports filtering by event type, role, and status.
 */

import { useInfiniteQuery } from "@tanstack/react-query";
import {
  activityEventsApi,
  type ActivityEventFilter,
  type ActivityEventPageResponse,
  type ActivityEventResponse,
} from "@/api/activity-events";

/**
 * Parameters for task activity events query
 */
export interface TaskActivityEventsParams {
  /** The task ID to fetch events for */
  taskId: string;
  /** Optional filter criteria */
  filter?: ActivityEventFilter;
  /** Page size (default 50, max 100) */
  limit?: number;
}

/**
 * Parameters for session activity events query
 */
export interface SessionActivityEventsParams {
  /** The ideation session ID to fetch events for */
  sessionId: string;
  /** Optional filter criteria */
  filter?: ActivityEventFilter;
  /** Page size (default 50, max 100) */
  limit?: number;
}

/**
 * Query key factory for activity events queries
 */
export const activityEventKeys = {
  all: ["activityEvents"] as const,
  task: (taskId: string, filter?: ActivityEventFilter) =>
    [...activityEventKeys.all, "task", taskId, filter] as const,
  session: (sessionId: string, filter?: ActivityEventFilter) =>
    [...activityEventKeys.all, "session", sessionId, filter] as const,
};

/**
 * Hook for fetching task activity events with infinite scroll pagination
 *
 * Uses cursor-based pagination for efficient browsing of historical events.
 * Events are ordered by created_at DESC (newest first).
 *
 * @param params - Query parameters
 * @returns TanStack Query infinite result with pagination helpers
 *
 * @example
 * ```tsx
 * const {
 *   data,
 *   fetchNextPage,
 *   hasNextPage,
 *   isFetchingNextPage,
 *   isLoading,
 * } = useTaskActivityEvents({
 *   taskId: "task-123",
 *   filter: { eventTypes: ["tool_call", "tool_result"] },
 * });
 *
 * const events = flattenActivityPages(data);
 * ```
 */
export function useTaskActivityEvents({
  taskId,
  filter,
  limit = 50,
}: TaskActivityEventsParams) {
  return useInfiniteQuery<ActivityEventPageResponse, Error>({
    queryKey: activityEventKeys.task(taskId, filter),
    queryFn: async ({ pageParam }) => {
      const cursor = pageParam as string | undefined;
      return activityEventsApi.task.list(taskId, {
        ...(cursor !== undefined && { cursor }),
        limit,
        ...(filter !== undefined && { filter }),
      });
    },
    getNextPageParam: (lastPage) => {
      // Return the cursor for the next page, or undefined if no more pages
      return lastPage.hasMore ? lastPage.cursor ?? undefined : undefined;
    },
    initialPageParam: undefined as string | undefined,
    // Activity events are append-only historical data, longer cache is appropriate
    staleTime: 10 * 60 * 1000, // 10 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes cache retention
    enabled: !!taskId,
  });
}

/**
 * Hook for fetching session activity events with infinite scroll pagination
 *
 * Uses cursor-based pagination for efficient browsing of historical events.
 * Events are ordered by created_at DESC (newest first).
 *
 * @param params - Query parameters
 * @returns TanStack Query infinite result with pagination helpers
 *
 * @example
 * ```tsx
 * const {
 *   data,
 *   fetchNextPage,
 *   hasNextPage,
 *   isFetchingNextPage,
 *   isLoading,
 * } = useSessionActivityEvents({
 *   sessionId: "session-123",
 *   filter: { roles: ["agent"] },
 * });
 *
 * const events = flattenActivityPages(data);
 * ```
 */
export function useSessionActivityEvents({
  sessionId,
  filter,
  limit = 50,
}: SessionActivityEventsParams) {
  return useInfiniteQuery<ActivityEventPageResponse, Error>({
    queryKey: activityEventKeys.session(sessionId, filter),
    queryFn: async ({ pageParam }) => {
      const cursor = pageParam as string | undefined;
      return activityEventsApi.session.list(sessionId, {
        ...(cursor !== undefined && { cursor }),
        limit,
        ...(filter !== undefined && { filter }),
      });
    },
    getNextPageParam: (lastPage) => {
      // Return the cursor for the next page, or undefined if no more pages
      return lastPage.hasMore ? lastPage.cursor ?? undefined : undefined;
    },
    initialPageParam: undefined as string | undefined,
    // Activity events are append-only historical data, longer cache is appropriate
    staleTime: 10 * 60 * 1000, // 10 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes cache retention
    enabled: !!sessionId,
  });
}

/**
 * Helper function to flatten paginated activity data into a single array
 *
 * Extracts all events from all loaded pages into a flat array.
 * Returns empty array if data is undefined.
 *
 * @param data - The infinite query data object
 * @returns Flattened array of activity events from all pages
 *
 * @example
 * ```tsx
 * const { data } = useTaskActivityEvents({ taskId: "123" });
 * const allEvents = flattenActivityPages(data);
 * ```
 */
export function flattenActivityPages(
  data: { pages: ActivityEventPageResponse[] } | undefined
): ActivityEventResponse[] {
  if (!data?.pages) return [];

  return data.pages.flatMap((page) => page.events);
}
