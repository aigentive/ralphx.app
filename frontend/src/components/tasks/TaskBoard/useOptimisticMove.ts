/**
 * Optimistic move hook with race condition handling
 */

import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { taskKeys } from "@/hooks/useTasks";

interface UseOptimisticMoveResult {
  move: (taskId: string, toStatus: string) => void;
  isMoving: boolean;
  error: string | null;
  clearError: () => void;
}

const ERROR_DISPLAY_DURATION = 3000;

/**
 * Hook for moving tasks with optimistic updates and error handling
 *
 * Handles race conditions when moving from Planned column:
 * - Shows error toast if task was already picked up
 * - Automatically clears error after timeout
 */
export function useOptimisticMove(projectId: string): UseOptimisticMoveResult {
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
      setError(null);
    },
    onError: (err: Error) => {
      setError(err.message);
      // Auto-clear error after timeout
      setTimeout(() => {
        setError(null);
      }, ERROR_DISPLAY_DURATION);
    },
  });

  const move = useCallback(
    (taskId: string, toStatus: string) => {
      mutation.mutate({ taskId, toStatus });
    },
    [mutation]
  );

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    move,
    isMoving: mutation.isPending,
    error,
    clearError,
  };
}
