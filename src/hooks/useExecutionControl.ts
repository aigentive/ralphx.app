/**
 * Execution control hooks - manage pause/resume/stop execution state
 *
 * Provides hooks for:
 * - useExecutionStatus: Query execution status (running/queued counts, pause state)
 * - usePauseExecution: Toggle pause/resume execution
 * - useStopExecution: Stop all running tasks
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type ExecutionStatusResponse } from "@/lib/tauri";
import { useUiStore } from "@/stores/uiStore";

/**
 * Query key factory for execution status
 */
export const executionKeys = {
  all: ["execution"] as const,
  status: () => [...executionKeys.all, "status"] as const,
};

/**
 * Hook to fetch and track execution status
 *
 * @returns TanStack Query result with execution status data
 * Also returns convenience accessors: isPaused, runningCount, queuedCount, etc.
 *
 * @example
 * ```tsx
 * const { isPaused, runningCount, queuedCount, isLoading } = useExecutionStatus();
 *
 * if (isLoading) return <Loading />;
 * return <ExecutionControlBar isPaused={isPaused} running={runningCount} />;
 * ```
 */
export function useExecutionStatus() {
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);

  const query = useQuery<ExecutionStatusResponse, Error>({
    queryKey: executionKeys.status(),
    queryFn: async () => {
      const status = await api.execution.getStatus();
      // Update uiStore with fresh status
      setExecutionStatus(status);
      return status;
    },
    // Fallback poll every 30s - real-time updates come via useExecutionEvents
    refetchInterval: 30000,
    // Also refetch on window focus
    refetchOnWindowFocus: true,
  });

  return {
    ...query,
    // Convenience accessors
    isPaused: query.data?.isPaused ?? false,
    runningCount: query.data?.runningCount ?? 0,
    queuedCount: query.data?.queuedCount ?? 0,
    maxConcurrent: query.data?.maxConcurrent ?? 2,
    canStartTask: query.data?.canStartTask ?? true,
  };
}

/**
 * Hook to pause/resume execution
 *
 * @returns Mutation for toggling pause state, plus convenience methods
 *
 * @example
 * ```tsx
 * const { toggle, isPending } = usePauseExecution();
 *
 * return (
 *   <button onClick={toggle} disabled={isPending}>
 *     {uiStore.executionStatus.isPaused ? 'Resume' : 'Pause'}
 *   </button>
 * );
 * ```
 */
export function usePauseExecution() {
  const queryClient = useQueryClient();
  const executionStatus = useUiStore((state) => state.executionStatus);
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);

  const pauseMutation = useMutation({
    mutationFn: async () => {
      const response = await api.execution.pause();
      return response;
    },
    onSuccess: (data) => {
      setExecutionStatus(data.status);
      queryClient.invalidateQueries({ queryKey: executionKeys.status() });
    },
  });

  const resumeMutation = useMutation({
    mutationFn: async () => {
      const response = await api.execution.resume();
      return response;
    },
    onSuccess: (data) => {
      setExecutionStatus(data.status);
      queryClient.invalidateQueries({ queryKey: executionKeys.status() });
    },
  });

  const toggle = () => {
    if (executionStatus.isPaused) {
      resumeMutation.mutate();
    } else {
      pauseMutation.mutate();
    }
  };

  return {
    toggle,
    pause: () => pauseMutation.mutate(),
    resume: () => resumeMutation.mutate(),
    isPending: pauseMutation.isPending || resumeMutation.isPending,
    isError: pauseMutation.isError || resumeMutation.isError,
    error: pauseMutation.error || resumeMutation.error,
  };
}

/**
 * Hook to stop all running tasks
 *
 * @returns Mutation for stopping execution
 *
 * @example
 * ```tsx
 * const { stop, isPending, canStop } = useStopExecution();
 *
 * return (
 *   <button onClick={stop} disabled={!canStop || isPending}>
 *     Stop
 *   </button>
 * );
 * ```
 */
export function useStopExecution() {
  const queryClient = useQueryClient();
  const executionStatus = useUiStore((state) => state.executionStatus);
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);

  const mutation = useMutation({
    mutationFn: async () => {
      const response = await api.execution.stop();
      return response;
    },
    onSuccess: (data) => {
      setExecutionStatus(data.status);
      queryClient.invalidateQueries({ queryKey: executionKeys.status() });
    },
  });

  return {
    stop: () => mutation.mutate(),
    isPending: mutation.isPending,
    isError: mutation.isError,
    error: mutation.error,
    // Can only stop if there are running tasks
    canStop: executionStatus.runningCount > 0,
  };
}
