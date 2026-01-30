/**
 * Task event hooks - Tauri task event listeners with type-safe validation
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { TaskEventSchema } from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";
import { taskKeys } from "@/hooks/useTasks";
import { infiniteTaskKeys } from "@/hooks/useInfiniteTasksQuery";
import { transformTask, type Task } from "@/types/task";

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
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "created":
          addTask(transformTask(taskEvent.task));
          // Invalidate both regular and infinite task queries
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
          break;
        case "updated": {
          // Transform snake_case changes to camelCase
          const transformedChanges: Partial<Task> = {};
          const changes = taskEvent.changes;
          if (changes.id !== undefined) transformedChanges.id = changes.id;
          if (changes.project_id !== undefined) transformedChanges.projectId = changes.project_id;
          if (changes.category !== undefined) transformedChanges.category = changes.category;
          if (changes.title !== undefined) transformedChanges.title = changes.title;
          if (changes.description !== undefined) transformedChanges.description = changes.description;
          if (changes.priority !== undefined) transformedChanges.priority = changes.priority;
          if (changes.internal_status !== undefined) transformedChanges.internalStatus = changes.internal_status;
          if (changes.needs_review_point !== undefined) transformedChanges.needsReviewPoint = changes.needs_review_point;
          if (changes.created_at !== undefined) transformedChanges.createdAt = changes.created_at;
          if (changes.updated_at !== undefined) transformedChanges.updatedAt = changes.updated_at;
          if (changes.started_at !== undefined) transformedChanges.startedAt = changes.started_at;
          if (changes.completed_at !== undefined) transformedChanges.completedAt = changes.completed_at;
          if (changes.archived_at !== undefined) transformedChanges.archivedAt = changes.archived_at;

          updateTask(taskEvent.taskId, transformedChanges);
          // Invalidate both regular and infinite task queries
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
          break;
        }
        case "deleted":
          removeTask(taskEvent.taskId);
          // Invalidate both regular and infinite task queries
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          // Invalidate both regular and infinite task queries so Kanban refetches
          queryClient.invalidateQueries({ queryKey: taskKeys.lists() });
          queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addTask, updateTask, removeTask, queryClient]);
}
