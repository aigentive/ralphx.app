/**
 * useTaskSearch hook - TanStack Query wrapper for server-side task search
 *
 * Implements server-side search using the search_tasks Tauri command.
 * Searches task titles and descriptions case-insensitively.
 * Requires minimum 2 characters to trigger search.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { Task } from "@/types/task";

/**
 * Parameters for task search query
 */
export interface TaskSearchParams {
  /** The project ID to search within */
  projectId: string;
  /** Search query string (min 2 chars required) */
  query: string | null;
  /** Whether to include archived tasks in search results (default false) */
  includeArchived?: boolean | undefined;
}

/**
 * Query key factory for task search queries
 */
export const taskSearchKeys = {
  all: ["tasks", "search"] as const,
  search: (params: TaskSearchParams) =>
    [
      ...taskSearchKeys.all,
      params.projectId,
      params.query,
      params.includeArchived,
    ] as const,
};

/**
 * Hook for searching tasks by query string
 *
 * Performs server-side search across task titles and descriptions.
 * Search is case-insensitive and requires minimum 2 characters.
 * Results are cached for 30 seconds (shorter than pagination cache
 * since search results can change more frequently).
 *
 * @param params - Search parameters
 * @param params.projectId - The project ID to search within
 * @param params.query - Search query string (null or < 2 chars disables search)
 * @param params.includeArchived - Whether to include archived tasks
 *
 * @returns TanStack Query result with search results
 *
 * @example
 * ```tsx
 * const { data: results, isLoading, isError } = useTaskSearch({
 *   projectId: "project-123",
 *   query: searchQuery,
 *   includeArchived: false,
 * });
 *
 * if (isLoading) return <SearchSpinner />;
 * if (isError) return <SearchError />;
 * if (!results?.length) return <EmptySearchState query={searchQuery} />;
 *
 * return (
 *   <div>
 *     {results.map(task => <TaskCard key={task.id} task={task} />)}
 *   </div>
 * );
 * ```
 */
export function useTaskSearch({
  projectId,
  query,
  includeArchived = false,
}: TaskSearchParams) {
  return useQuery<Task[], Error>({
    queryKey: taskSearchKeys.search({ projectId, query, includeArchived }),
    queryFn: async () => {
      // This should never be called when enabled=false, but TypeScript doesn't know that
      if (!query || query.length < 2) {
        return [];
      }
      return api.tasks.search(projectId, query, includeArchived);
    },
    // Only enable search when query has at least 2 characters
    enabled: query !== null && query.length >= 2,
    // Search results change frequently: shorter cache than infinite scroll
    staleTime: 30 * 1000, // 30 seconds
  });
}
