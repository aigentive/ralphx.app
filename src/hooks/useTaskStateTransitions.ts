/**
 * useTaskStateTransitions hook - TanStack Query wrapper for task state history
 *
 * Fetches the chronological list of state transitions for a task,
 * enabling the StateTimelineNav component to display task history
 * and support the "time travel" feature.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { StateTransition } from "@/api/tasks";

/**
 * Query key factory for task state transitions
 */
export const stateTransitionKeys = {
  all: ["stateTransitions"] as const,
  task: (taskId: string) => [...stateTransitionKeys.all, taskId] as const,
};

/**
 * Hook to fetch state transitions for a task
 *
 * @param taskId - The task ID to fetch state transitions for
 * @returns TanStack Query result with state transitions data
 *
 * @example
 * ```tsx
 * const { data: transitions, isLoading } = useTaskStateTransitions("task-123");
 *
 * if (isLoading) return <Loading />;
 * return <StateTimeline transitions={transitions} />;
 * ```
 */
export function useTaskStateTransitions(taskId: string | undefined) {
  return useQuery<StateTransition[], Error>({
    queryKey: stateTransitionKeys.task(taskId ?? ""),
    queryFn: async () => {
      if (!taskId) return [];
      return api.tasks.getStateTransitions(taskId);
    },
    enabled: Boolean(taskId),
  });
}
