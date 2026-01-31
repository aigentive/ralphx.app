/**
 * useStepEvents hook - Real-time step event listener
 *
 * Listens to step events from the backend and invalidates relevant queries
 * to keep the UI in sync with step progress updates from the worker agent.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { stepKeys } from "./useTaskSteps";
import type { Unsubscribe } from "@/lib/event-bus";

/**
 * Hook to listen for step events from the backend
 *
 * Listens to 'step:created', 'step:updated', 'step:deleted', and 'steps:reordered' events
 * and invalidates TanStack Query caches to trigger refetching of step data.
 *
 * This ensures the UI stays in sync when the worker agent updates step progress.
 *
 * @example
 * ```tsx
 * function TaskFullView() {
 *   useStepEvents(); // Auto-refreshes step data on backend events
 *   return <StepList taskId={taskId} />;
 * }
 * ```
 */
export function useStepEvents() {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Listen for step:created events
    unsubscribes.push(
      bus.subscribe<{ task_id: string }>("step:created", (payload) => {
        const taskId = payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      })
    );

    // Listen for step:updated events
    unsubscribes.push(
      bus.subscribe<{ task_id: string }>("step:updated", (payload) => {
        const taskId = payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      })
    );

    // Listen for step:deleted events
    unsubscribes.push(
      bus.subscribe<{ task_id: string }>("step:deleted", (payload) => {
        const taskId = payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      })
    );

    // Listen for steps:reordered events
    unsubscribes.push(
      bus.subscribe<{ task_id: string }>("steps:reordered", (payload) => {
        const taskId = payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          // No need to invalidate progress - reordering doesn't affect counts
        }
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, queryClient]);
}
