/**
 * useTaskMutation hook - TanStack Query mutations for task operations
 *
 * Provides mutations for creating, updating, deleting, and moving tasks
 * with automatic cache invalidation.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api } from "@/lib/tauri";
import type { CreateTask, UpdateTask } from "@/types/task";
import { taskKeys } from "./useTasks";
import { infiniteTaskKeys } from "./useInfiniteTasksQuery";

/**
 * Hook for task mutation operations
 *
 * Provides mutations for:
 * - Creating tasks
 * - Updating tasks
 * - Deleting tasks
 * - Moving tasks to a new status
 * - Blocking/unblocking tasks
 *
 * @param projectId - The project ID for cache invalidation
 * @returns Object containing all task mutations
 *
 * @example
 * ```tsx
 * const { createMutation, moveMutation, blockMutation } = useTaskMutation("project-123");
 *
 * // Create a new task
 * createMutation.mutate({ projectId: "project-123", title: "New Task" });
 *
 * // Move a task
 * moveMutation.mutate({ taskId: "task-1", toStatus: "ready" });
 *
 * // Block a task
 * blockMutation.mutate({ taskId: "task-1", reason: "Waiting for API" });
 * ```
 */
export function useTaskMutation(projectId: string) {
  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: (input: CreateTask) => api.tasks.create(input),
    onSuccess: () => {
      // Invalidate both regular and infinite task queries
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({ taskId, input }: { taskId: string; input: UpdateTask }) =>
      api.tasks.update(taskId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
    },
  });

  /** @deprecated Use cleanupTaskMutation instead */
  const deleteMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.delete(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
    },
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
    },
  });

  const archiveMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.archive(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      queryClient.invalidateQueries({ queryKey: ["archived-count"] });
      toast.success("Task archived");
    },
    onError: (error: Error) => {
      toast.error(`Failed to archive task: ${error.message}`);
    },
  });

  const restoreMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.restore(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      queryClient.invalidateQueries({ queryKey: ["archived-count"] });
      toast.success("Task restored");
    },
    onError: (error: Error) => {
      toast.error(`Failed to restore task: ${error.message}`);
    },
  });

  /** @deprecated Use cleanupTaskMutation instead */
  const permanentlyDeleteMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.permanentlyDelete(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      queryClient.invalidateQueries({ queryKey: ["archived-count"] });
      toast.success("Task permanently deleted");
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete task: ${error.message}`);
    },
  });

  const blockMutation = useMutation({
    mutationFn: ({ taskId, reason }: { taskId: string; reason?: string }) =>
      api.tasks.block(taskId, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      toast.success("Task blocked");
    },
    onError: (error: Error) => {
      toast.error(`Failed to block task: ${error.message}`);
    },
  });

  const unblockMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.unblock(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      toast.success("Task unblocked");
    },
    onError: (error: Error) => {
      toast.error(`Failed to unblock task: ${error.message}`);
    },
  });

  const cleanupTaskMutation = useMutation({
    mutationFn: (taskId: string) => api.tasks.cleanupTask(taskId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      toast.success("Task cleaned up");
    },
    onError: (error: Error) => {
      toast.error(`Failed to cleanup task: ${error.message}`);
    },
  });

  const cleanupTasksInGroupMutation = useMutation({
    mutationFn: ({
      groupKind,
      groupId,
      projectId: pid,
    }: {
      groupKind: string;
      groupId: string;
      projectId: string;
    }) => api.tasks.cleanupTasksInGroup(groupKind, groupId, pid),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      queryClient.invalidateQueries({ queryKey: infiniteTaskKeys.all });
      toast.success(`Cleaned up ${data.deletedCount} task${data.deletedCount === 1 ? "" : "s"}`);
    },
    onError: (error: Error) => {
      toast.error(`Failed to cleanup tasks: ${error.message}`);
    },
  });

  return {
    createMutation,
    updateMutation,
    deleteMutation,
    moveMutation,
    archiveMutation,
    restoreMutation,
    permanentlyDeleteMutation,
    blockMutation,
    unblockMutation,
    cleanupTaskMutation,
    cleanupTasksInGroupMutation,
    isArchiving: archiveMutation.isPending,
    isRestoring: restoreMutation.isPending,
    isPermanentlyDeleting: permanentlyDeleteMutation.isPending,
    isBlocking: blockMutation.isPending,
    isUnblocking: unblockMutation.isPending,
    isCleaningTask: cleanupTaskMutation.isPending,
    isCleaningGroup: cleanupTasksInGroupMutation.isPending,
  };
}
