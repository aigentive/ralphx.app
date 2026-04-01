/**
 * useTaskMetrics hook — TanStack Query wrapper for per-task metrics.
 *
 * Fetches step counts, review cycles, execution time, and derived complexity
 * tier for a single task. Data is cached for 5 minutes (staleTime) since
 * task metrics don't change frequently and the backend does not cache them.
 *
 * Usage:
 * - In task detail views: call `useTaskMetrics(task.id)` to get full data.
 * - On task cards: use `queryClient.getQueryData(taskMetricsKeys.detail(id))`
 *   to read cached data without triggering a new fetch.
 */

import { useQuery } from "@tanstack/react-query";
import { getTaskMetrics, type TaskMetrics } from "@/api/task-metrics";

// ============================================================================
// Query Key Factory
// ============================================================================

export const taskMetricsKeys = {
  all: ["task-metrics"] as const,
  detail: (taskId: string) => [...taskMetricsKeys.all, taskId] as const,
};

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to fetch engineering metrics for a single task.
 *
 * @param taskId - The task ID to fetch metrics for
 * @returns TanStack Query result with per-task metrics
 *
 * @example
 * ```tsx
 * const { data: metrics } = useTaskMetrics(task.id);
 * const complexity = metrics ? deriveComplexityTier(metrics) : null;
 * ```
 */
export function useTaskMetrics(taskId: string) {
  return useQuery<TaskMetrics, Error>({
    queryKey: taskMetricsKeys.detail(taskId),
    queryFn: () => getTaskMetrics(taskId),
    // Task metrics are stable — cache for 5 minutes before re-fetching
    staleTime: 5 * 60_000,
    gcTime: 10 * 60_000,
  });
}
