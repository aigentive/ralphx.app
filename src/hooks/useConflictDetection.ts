/**
 * useConflictDetection - TanStack Query hook for live merge conflict detection
 *
 * Polls the `detect_merge_conflicts` Tauri command every 5 seconds when enabled.
 * Only enabled for active merge-related task states (pending_merge, merging, merge_conflict).
 * Historical views should read from task.metadata.conflict_files instead.
 *
 * Data source strategy:
 * - Active states (isHistorical=false): Live query via this hook
 * - Historical states (isHistorical=true): Metadata snapshot (task.metadata.conflict_files)
 */

import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

/** Internal task statuses that should use live conflict detection */
const ACTIVE_MERGE_STATES = new Set([
  "pending_merge",
  "merging",
  "merge_conflict",
]);

/**
 * Query key factory for conflict detection
 */
export const conflictDetectionKeys = {
  all: ["conflict-detection"] as const,
  forTask: (taskId: string) =>
    [...conflictDetectionKeys.all, taskId] as const,
};

export interface UseConflictDetectionOptions {
  /** Task ID to check for conflicts */
  taskId: string;
  /** Current task internal status */
  internalStatus: string;
  /** Whether this is a historical view (uses metadata instead of live query) */
  isHistorical?: boolean;
  /** Whether the task has a branch assigned */
  hasBranch?: boolean;
}

export interface UseConflictDetectionResult {
  /** Array of file paths with conflicts (empty if no conflicts) */
  conflicts: string[];
  /** Whether the query is currently loading */
  isLoading: boolean;
  /** Whether the query encountered an error */
  isError: boolean;
  /** Error message if any */
  error: Error | null;
  /** Whether conflict detection is enabled (active merge state) */
  isEnabled: boolean;
}

/**
 * Hook for live merge conflict detection
 *
 * Uses TanStack Query with 5s polling interval when enabled.
 * Only active for merge-related states (pending_merge, merging, merge_conflict)
 * and when NOT in historical mode.
 *
 * @example
 * ```tsx
 * const { conflicts, isLoading, isEnabled } = useConflictDetection({
 *   taskId: task.id,
 *   internalStatus: task.internalStatus,
 *   isHistorical: false,
 *   hasBranch: !!task.taskBranch,
 * });
 *
 * // Use live conflicts when enabled and available, fall back to metadata
 * const displayConflicts = isHistorical
 *   ? metadataConflicts
 *   : (isEnabled && conflicts.length > 0 ? conflicts : metadataConflicts);
 * ```
 */
export function useConflictDetection({
  taskId,
  internalStatus,
  isHistorical = false,
  hasBranch = true,
}: UseConflictDetectionOptions): UseConflictDetectionResult {
  // Only enable live queries for active merge states with a branch
  const isEnabled =
    !isHistorical &&
    ACTIVE_MERGE_STATES.has(internalStatus) &&
    hasBranch;

  const query = useQuery<string[], Error>({
    queryKey: conflictDetectionKeys.forTask(taskId),
    queryFn: async () => {
      return await invoke<string[]>("detect_merge_conflicts", { taskId });
    },
    enabled: isEnabled,
    // Poll every 5 seconds for real-time conflict updates
    refetchInterval: isEnabled ? 5000 : false,
    // Don't refetch on window focus for conflict detection (polling is sufficient)
    refetchOnWindowFocus: false,
    // Keep data fresh for 2 seconds (prevents immediate refetch on remount)
    staleTime: 2000,
    // Retry once on failure, don't spam retries
    retry: 1,
  });

  return {
    conflicts: query.data ?? [],
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    isEnabled,
  };
}
