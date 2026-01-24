/**
 * useTaskMutation hook - TanStack Query mutations for task operations
 *
 * Provides mutations for creating, updating, deleting, and moving tasks
 * with automatic cache invalidation.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import type { CreateTask, UpdateTask } from "@/types/task";
import { taskKeys } from "./useTasks";

/**
 * Hook for task mutation operations
 *
 * Provides mutations for:
 * - Creating tasks
 * - Updating tasks
 * - Deleting tasks
 * - Moving tasks to a new status
 *
 * @param projectId - The project ID for cache invalidation
 * @returns Object containing all task mutations
 *
 * @example
 * ```tsx
 * const { createMutation, moveMutation } = useTaskMutation("project-123");
 *
 * // Create a new task
 * createMutation.mutate({ projectId: "project-123", title: "New Task" });
 *
 * // Move a task
 * moveMutation.mutate({ taskId: "task-1", toStatus: "ready" });
 * ```
 */
export function useTaskMutation(projectId: string) {
  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: (input: CreateTask) => api.tasks.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ taskId, input }: { taskId: string; input: UpdateTask }) =>
      api.tasks.update(taskId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.delete(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    },
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    },
  });

  return {
    createMutation,
    updateMutation,
    deleteMutation,
    moveMutation,
  };
}
