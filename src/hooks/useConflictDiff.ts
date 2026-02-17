/**
 * useConflictDiff - TanStack Query hook for fetching conflict file diff data
 *
 * Fetches the `get_conflict_file_diff` Tauri command to get conflict details
 * for a specific file in a merge-conflicted task.
 *
 * Unlike useConflictDetection (which polls), this hook fetches once on demand.
 */

import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

/**
 * Conflict diff data returned from the backend
 */
export interface ConflictDiff {
  /** Path to the conflicted file */
  filePath: string;
  /** Content from the base/common ancestor */
  baseContent: string;
  /** Content from ours (target branch) */
  oursContent: string;
  /** Content from theirs (source branch) */
  theirsContent: string;
  /** Merged content with conflict markers */
  mergedWithMarkers: string;
  /** Detected language for syntax highlighting */
  language: string;
}

/**
 * Query key factory for conflict diff
 */
export const conflictDiffKeys = {
  all: ["conflict-diff"] as const,
  forFile: (taskId: string, filePath: string) =>
    [...conflictDiffKeys.all, taskId, filePath] as const,
};

export interface UseConflictDiffOptions {
  /** Task ID to fetch conflict diff for */
  taskId: string;
  /** File path to fetch conflict diff for (null disables the query) */
  filePath: string | null;
  /** Whether to enable the query (default: true when filePath is provided) */
  enabled?: boolean;
}

export interface UseConflictDiffResult {
  /** Conflict diff data, or null if not loaded */
  data: ConflictDiff | null;
  /** Whether the query is currently loading */
  isLoading: boolean;
  /** Whether the query encountered an error */
  isError: boolean;
  /** Error if any */
  error: Error | null;
}

/**
 * Hook for fetching conflict diff for a specific file
 *
 * @example
 * ```tsx
 * const { data, isLoading, isError } = useConflictDiff({
 *   taskId: task.id,
 *   filePath: selectedFile,
 * });
 *
 * if (isLoading) return <Spinner />;
 * if (isError || !data) return <Error />;
 * return <ConflictDiffViewer conflictDiff={data} />;
 * ```
 */
export function useConflictDiff({
  taskId,
  filePath,
  enabled = true,
}: UseConflictDiffOptions): UseConflictDiffResult {
  const isEnabled = enabled && filePath !== null;

  const query = useQuery<ConflictDiff, Error>({
    queryKey: conflictDiffKeys.forFile(taskId, filePath ?? ""),
    queryFn: async () => {
      return await invoke<ConflictDiff>("get_conflict_file_diff", {
        taskId,
        filePath,
      });
    },
    enabled: isEnabled,
    // No polling - fetch once on demand
    refetchInterval: false,
    refetchOnWindowFocus: false,
    // Keep data fresh for 30 seconds
    staleTime: 30000,
    // Retry once on failure
    retry: 1,
  });

  return {
    data: query.data ?? null,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
  };
}
