/**
 * Task event hooks - Tauri task event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { TaskEventSchema } from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";
import { taskKeys } from "@/hooks/useTasks";
import type { Task } from "@/types/task";

/**
 * Hook to listen for task events from the backend
 *
 * Listens to 'task:event' events and updates the task store accordingly.
 * Validates incoming events using TaskEventSchema before processing.
 *
 * @example
 * ```tsx
 * function App() {
 *   useTaskEvents(); // Sets up listener automatically
 *   return <TaskBoard />;
 * }
 * ```
 */
export function useTaskEvents() {
  const addTask = useTaskStore((s) => s.addTask);
  const updateTask = useTaskStore((s) => s.updateTask);
  const removeTask = useTaskStore((s) => s.removeTask);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("task:event", (event) => {
      // Runtime validation of backend events
      const parsed = TaskEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid task event:", parsed.error.message);
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "created":
          addTask(taskEvent.task);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "updated":
          // Cast to Partial<Task> for exactOptionalPropertyTypes compatibility
          updateTask(taskEvent.taskId, taskEvent.changes as Partial<Task>);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "deleted":
          removeTask(taskEvent.taskId);
          // Invalidate task list queries to refetch
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          // Invalidate task list queries so Kanban board refetches
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addTask, updateTask, removeTask, queryClient]);
}
