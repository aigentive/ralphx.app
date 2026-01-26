/**
 * useStepMutations hook - TanStack Query mutations for task step operations
 *
 * Provides mutations for creating, updating, deleting, and reordering task steps
 * with automatic cache invalidation and toast notifications.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api } from "@/lib/tauri";
import { stepKeys } from "./useTaskSteps";

/**
 * Hook for task step mutation operations
 *
 * Provides mutations for:
 * - Creating steps
 * - Updating steps
 * - Deleting steps
 * - Reordering steps
 *
 * @param taskId - The task ID for cache invalidation
 * @returns Object containing all step mutations
 *
 * @example
 * ```tsx
 * const { create, update, delete: deleteStep } = useStepMutations("task-123");
 *
 * // Create a new step
 * create.mutate({ title: "New Step", description: "Do something" });
 *
 * // Update a step
 * update.mutate({ stepId: "step-1", data: { title: "Updated Title" } });
 *
 * // Delete a step
 * deleteStep.mutate("step-1");
 *
 * // Reorder steps
 * reorder.mutate(["step-3", "step-1", "step-2"]);
 * ```
 */
export function useStepMutations(taskId: string) {
  const queryClient = useQueryClient();

  const create = useMutation({
    mutationFn: (data: {
      title: string;
      description?: string;
      sortOrder?: number;
    }) => api.steps.create(taskId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
      queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
      toast.success("Step created");
    },
    onError: (error: Error) => {
      toast.error(`Failed to create step: ${error.message}`);
    },
  });

  const update = useMutation({
    mutationFn: ({
      stepId,
      data,
    }: {
      stepId: string;
      data: { title?: string; description?: string; sortOrder?: number };
    }) => api.steps.update(stepId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
      queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
      toast.success("Step updated");
    },
    onError: (error: Error) => {
      toast.error(`Failed to update step: ${error.message}`);
    },
  });

  const deleteStep = useMutation({
    mutationFn: (stepId: string) => api.steps.delete(stepId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
      queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
      toast.success("Step deleted");
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete step: ${error.message}`);
    },
  });

  const reorder = useMutation({
    mutationFn: (stepIds: string[]) => api.steps.reorder(taskId, stepIds),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
      queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
      toast.success("Steps reordered");
    },
    onError: (error: Error) => {
      toast.error(`Failed to reorder steps: ${error.message}`);
    },
  });

  return {
    create,
    update,
    delete: deleteStep,
    reorder,
    isCreating: create.isPending,
    isUpdating: update.isPending,
    isDeleting: deleteStep.isPending,
    isReordering: reorder.isPending,
  };
}
