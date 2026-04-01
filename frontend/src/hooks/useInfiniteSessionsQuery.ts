/**
 * useInfiniteSessionsQuery hook - TanStack Query wrapper for paginated session group data
 *
 * Implements offset-based pagination using TanStack Query's useInfiniteQuery.
 * Mirrors the useInfiniteTasksQuery pattern. Used by PlanBrowser for lazy-loading
 * session groups on expand with infinite scroll.
 */

import { useInfiniteQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { ideationKeys } from "@/hooks/useIdeation";
import type { SessionListResponse, SessionGroupKey } from "@/types/ideation";

export const PAGE_SIZE = 20;

/**
 * Helper function to flatten paginated session data into a single array
 *
 * @param data - The infinite query data object
 * @returns Flattened array of sessions from all pages
 */
export function flattenSessionPages(
  data: { pages: SessionListResponse[] } | undefined
) {
  if (!data?.pages) return [];
  return data.pages.flatMap((page) => page.sessions);
}

/**
 * Hook for fetching sessions in a specific group with infinite scroll pagination
 *
 * Uses TanStack Query's useInfiniteQuery for automatic pagination management.
 * Each page contains 20 sessions. Lazy-loads when a group is expanded in PlanBrowser.
 *
 * @param projectId - The project ID to fetch sessions for
 * @param group - The session group key (underscore convention: "in_progress")
 * @param options.enabled - Whether to run the query (default true; set false when group is collapsed)
 *
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
 * } = useInfiniteSessionsQuery(projectId, "in_progress", { enabled: isExpanded });
 *
 * const sessions = flattenSessionPages(data);
 * ```
 */
export function useInfiniteSessionsQuery(
  projectId: string,
  group: SessionGroupKey,
  options?: { enabled?: boolean; search?: string }
) {
  const search = options?.search;
  return useInfiniteQuery<SessionListResponse, Error>({
    queryKey: ideationKeys.sessionsByGroup(projectId, group, search),
    queryFn: ({ pageParam = 0 }) =>
      invoke<SessionListResponse>("list_sessions_by_group", {
        projectId,
        group,
        offset: pageParam as number,
        limit: PAGE_SIZE,
        ...(search ? { search } : {}),
      }),
    getNextPageParam: (lastPage) =>
      lastPage.hasMore ? lastPage.offset + lastPage.sessions.length : undefined,
    initialPageParam: 0,
    enabled: Boolean(projectId) && (options?.enabled ?? true),
    staleTime: 10 * 60 * 1000, // 10 minutes
    gcTime: 30 * 60 * 1000, // 30 minutes cache retention
  });
}
