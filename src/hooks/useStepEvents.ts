/**
 * useStepEvents hook - Real-time step event listener
 *
 * Listens to step events from the backend and invalidates relevant queries
 * to keep the UI in sync with step progress updates from the worker agent.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { stepKeys } from "./useTaskSteps";

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
  const queryClient = useQueryClient();

  useEffect(() => {
    // Listen for step:created events
    const unlistenCreated: Promise<UnlistenFn> = listen<{ task_id: string }>(
      "step:created",
      (event) => {
        const taskId = event.payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      }
    );

    // Listen for step:updated events
    const unlistenUpdated: Promise<UnlistenFn> = listen<{ task_id: string }>(
      "step:updated",
      (event) => {
        const taskId = event.payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      }
    );

    // Listen for step:deleted events
    const unlistenDeleted: Promise<UnlistenFn> = listen<{ task_id: string }>(
      "step:deleted",
      (event) => {
        const taskId = event.payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
        }
      }
    );

    // Listen for steps:reordered events
    const unlistenReordered: Promise<UnlistenFn> = listen<{ task_id: string }>(
      "steps:reordered",
      (event) => {
        const taskId = event.payload.task_id;
        if (taskId) {
          queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
          // No need to invalidate progress - reordering doesn't affect counts
        }
      }
    );

    return () => {
      void (async () => {
        const createdFn = await unlistenCreated;
        const updatedFn = await unlistenUpdated;
        const deletedFn = await unlistenDeleted;
        const reorderedFn = await unlistenReordered;

        createdFn();
        updatedFn();
        deletedFn();
        reorderedFn();
      })();
    };
  }, [queryClient]);
}
