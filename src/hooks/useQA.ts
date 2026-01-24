/**
 * QA hooks - React hooks for QA settings and task QA data
 *
 * Provides hooks for:
 * - useQASettings: Global QA settings with load/update
 * - useTaskQA: Per-task QA data
 * - useQAResults: QA test results with optional polling
 */

import { useEffect, useCallback, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type TaskQAResponse, type QAResultsResponse, type UpdateQASettingsInput } from "@/lib/tauri";
import { useQAStore, selectTaskQA, selectIsTaskLoading, selectTaskQAResults } from "@/stores/qaStore";
import type { QASettings } from "@/types/qa-config";

// ============================================================================
// Query Keys
// ============================================================================

export const qaKeys = {
  all: ["qa"] as const,
  settings: () => [...qaKeys.all, "settings"] as const,
  taskQA: () => [...qaKeys.all, "taskQA"] as const,
  taskQAById: (taskId: string) => [...qaKeys.taskQA(), taskId] as const,
  results: () => [...qaKeys.all, "results"] as const,
  resultsById: (taskId: string) => [...qaKeys.results(), taskId] as const,
};

// ============================================================================
// useQASettings
// ============================================================================

/**
 * Hook to manage global QA settings
 *
 * @returns Settings, loading state, error, and update function
 *
 * @example
 * ```tsx
 * const { settings, isLoading, updateSettings } = useQASettings();
 *
 * // Toggle QA globally
 * await updateSettings({ qa_enabled: !settings.qa_enabled });
 * ```
 */
export function useQASettings() {
  const queryClient = useQueryClient();
  const storeSettings = useQAStore((s) => s.settings);
  const settingsLoaded = useQAStore((s) => s.settingsLoaded);
  const setSettings = useQAStore((s) => s.setSettings);
  const storeUpdateSettings = useQAStore((s) => s.updateSettings);
  const setError = useQAStore((s) => s.setError);

  // Query for initial load
  const query = useQuery<QASettings, Error>({
    queryKey: qaKeys.settings(),
    queryFn: () => api.qa.getSettings(),
    enabled: !settingsLoaded, // Only fetch if not already loaded
    staleTime: 5 * 60 * 1000, // 5 minutes
  });

  // Sync query data to store
  useEffect(() => {
    if (query.data && !settingsLoaded) {
      setSettings(query.data);
    }
  }, [query.data, settingsLoaded, setSettings]);

  // Sync error to store
  useEffect(() => {
    if (query.error) {
      setError(query.error.message);
    }
  }, [query.error, setError]);

  // Mutation for updating settings
  const mutation = useMutation<QASettings, Error, UpdateQASettingsInput>({
    mutationFn: (input) => api.qa.updateSettings(input),
    onSuccess: (data) => {
      setSettings(data);
      queryClient.setQueryData(qaKeys.settings(), data);
    },
    onError: (error) => {
      setError(error.message);
    },
  });

  const updateSettings = useCallback(
    async (input: UpdateQASettingsInput) => {
      // Optimistic update
      storeUpdateSettings(input);
      return mutation.mutateAsync(input);
    },
    [storeUpdateSettings, mutation]
  );

  return {
    /** Current QA settings */
    settings: settingsLoaded ? storeSettings : query.data ?? storeSettings,
    /** Whether settings are loading */
    isLoading: query.isLoading && !settingsLoaded,
    /** Whether settings update is in progress */
    isUpdating: mutation.isPending,
    /** Error message if any */
    error: query.error?.message ?? mutation.error?.message ?? null,
    /** Update settings */
    updateSettings,
    /** Refetch settings from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useTaskQA
// ============================================================================

/**
 * Hook to get QA data for a specific task
 *
 * @param taskId - The task ID to fetch QA data for
 * @param options - Hook options
 * @returns TaskQA data, loading state, and refetch function
 *
 * @example
 * ```tsx
 * const { data, isLoading, error } = useTaskQA("task-123");
 *
 * if (isLoading) return <Spinner />;
 * if (!data) return <p>No QA data</p>;
 * return <QAPanel data={data} />;
 * ```
 */
export function useTaskQA(taskId: string, options: { enabled?: boolean } = {}) {
  const { enabled = true } = options;

  const setTaskQA = useQAStore((s) => s.setTaskQA);
  const setLoadingTask = useQAStore((s) => s.setLoadingTask);
  const storeData = useQAStore(selectTaskQA(taskId));
  const isStoreLoading = useQAStore(selectIsTaskLoading(taskId));

  const query = useQuery<TaskQAResponse | null, Error>({
    queryKey: qaKeys.taskQAById(taskId),
    queryFn: () => api.qa.getTaskQA(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Sync loading state
  useEffect(() => {
    if (query.isLoading) {
      setLoadingTask(taskId, true);
    }
  }, [query.isLoading, taskId, setLoadingTask]);

  // Sync query data to store
  useEffect(() => {
    if (query.data !== undefined) {
      setTaskQA(taskId, query.data);
    }
  }, [query.data, taskId, setTaskQA]);

  return {
    /** TaskQA data (from store if available, else from query) */
    data: storeData ?? query.data ?? null,
    /** Whether data is loading */
    isLoading: query.isLoading || isStoreLoading,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Refetch data from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useQAResults
// ============================================================================

/**
 * Hook to get QA test results for a specific task
 *
 * Supports polling for active tests (when overall_status is "running").
 *
 * @param taskId - The task ID to fetch results for
 * @param options - Hook options
 * @returns QA results, loading state, and refetch function
 *
 * @example
 * ```tsx
 * const { data, isLoading } = useQAResults("task-123", { poll: true });
 *
 * if (data?.overall_status === "passed") {
 *   return <PassedBadge />;
 * }
 * ```
 */
export function useQAResults(
  taskId: string,
  options: {
    /** Whether to enable polling */
    poll?: boolean;
    /** Polling interval in milliseconds (default: 2000) */
    pollInterval?: number;
    /** Whether to enable the query */
    enabled?: boolean;
  } = {}
) {
  const { poll = false, pollInterval = 2000, enabled = true } = options;

  const storeResults = useQAStore(selectTaskQAResults(taskId));

  const query = useQuery<QAResultsResponse | null, Error>({
    queryKey: qaKeys.resultsById(taskId),
    queryFn: () => api.qa.getResults(taskId),
    enabled: enabled && !!taskId,
    staleTime: 10 * 1000, // 10 seconds
    // Enable polling if poll=true and status is "running"
    // Note: We use storeResults here because query.data isn't available in initializer
    refetchInterval: poll && storeResults?.overall_status === "running" ? pollInterval : false,
  });

  // Use query data preferentially for computed state
  const effectiveResults = query.data ?? storeResults ?? null;

  // Computed state based on effective results
  const isActive = effectiveResults?.overall_status === "running" || effectiveResults?.overall_status === "pending";
  const isPassed = effectiveResults?.overall_status === "passed";
  const isFailed = effectiveResults?.overall_status === "failed";

  return {
    /** QA results (from query if available, else from store) */
    data: effectiveResults,
    /** Whether results are loading */
    isLoading: query.isLoading,
    /** Whether polling is active */
    isPolling: query.isRefetching,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Whether QA is active (running or pending) */
    isActive,
    /** Whether QA passed */
    isPassed,
    /** Whether QA failed */
    isFailed,
    /** Refetch results from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useQAActions
// ============================================================================

/**
 * Hook to get QA action functions (retry, skip)
 *
 * @param taskId - The task ID to perform actions on
 * @returns Action functions and their loading states
 *
 * @example
 * ```tsx
 * const { retry, skip, isRetrying, isSkipping } = useQAActions("task-123");
 *
 * <Button onClick={retry} disabled={isRetrying}>Retry QA</Button>
 * <Button onClick={skip} disabled={isSkipping}>Skip QA</Button>
 * ```
 */
export function useQAActions(taskId: string) {
  const queryClient = useQueryClient();
  const setTaskQA = useQAStore((s) => s.setTaskQA);

  const retryMutation = useMutation<TaskQAResponse, Error>({
    mutationFn: () => api.qa.retry(taskId),
    onSuccess: (data) => {
      setTaskQA(taskId, data);
      queryClient.setQueryData(qaKeys.taskQAById(taskId), data);
      queryClient.invalidateQueries({ queryKey: qaKeys.resultsById(taskId) });
    },
  });

  const skipMutation = useMutation<TaskQAResponse, Error>({
    mutationFn: () => api.qa.skip(taskId),
    onSuccess: (data) => {
      setTaskQA(taskId, data);
      queryClient.setQueryData(qaKeys.taskQAById(taskId), data);
      queryClient.invalidateQueries({ queryKey: qaKeys.resultsById(taskId) });
    },
  });

  const retry = useCallback(() => retryMutation.mutateAsync(), [retryMutation]);
  const skip = useCallback(() => skipMutation.mutateAsync(), [skipMutation]);

  return {
    /** Retry QA tests for the task */
    retry,
    /** Skip QA for the task */
    skip,
    /** Whether retry is in progress */
    isRetrying: retryMutation.isPending,
    /** Whether skip is in progress */
    isSkipping: skipMutation.isPending,
    /** Error from retry action */
    retryError: retryMutation.error?.message ?? null,
    /** Error from skip action */
    skipError: skipMutation.error?.message ?? null,
  };
}

// ============================================================================
// Convenience Hooks
// ============================================================================

/**
 * Hook to check if QA is globally enabled
 */
export function useIsQAEnabled(): boolean {
  const settings = useQAStore((s) => s.settings);
  return settings.qa_enabled;
}

/**
 * Hook to check if a task needs QA based on global settings and task category
 *
 * @param category - Task category (e.g., "ui", "api", "feature")
 * @param needsQAOverride - Per-task override (null = use global)
 */
export function useTaskNeedsQA(category: string, needsQAOverride: boolean | null): boolean {
  const settings = useQAStore((s) => s.settings);

  return useMemo(() => {
    // Per-task override takes precedence
    if (needsQAOverride !== null) {
      return needsQAOverride;
    }

    // Check global setting
    if (!settings.qa_enabled) {
      return false;
    }

    // Check category-specific settings
    const uiCategories = ["ui", "component", "feature"];
    const apiCategories = ["api", "backend", "endpoint"];

    if (uiCategories.includes(category)) {
      return settings.auto_qa_for_ui_tasks;
    }

    if (apiCategories.includes(category)) {
      return settings.auto_qa_for_api_tasks;
    }

    return false;
  }, [settings, category, needsQAOverride]);
}
