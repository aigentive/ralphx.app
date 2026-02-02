/**
 * useGitDiff - Hook for fetching git diff data for reviews
 *
 * Provides file changes, commit history, and diff data for a task.
 * Connects to Tauri backend to fetch real diff data from agent activity events.
 * Used primarily in the ReviewsPanel with integrated DiffViewer.
 */

import { useState, useCallback, useEffect } from "react";
import { diffApi } from "@/api/diff";
import type { CommitInfo } from "@/api/diff";
import { getLanguageFromPath } from "@/components/diff/DiffViewer.types";
import type { FileChange, Commit, DiffData } from "@/components/diff";

export interface UseGitDiffOptions {
  /** Task ID to fetch diff data for */
  taskId: string;
  /** Whether to enable the hook */
  enabled?: boolean | undefined;
}

export interface UseGitDiffResult {
  /** Current uncommitted file changes */
  changes: FileChange[];
  /** Commit history */
  commits: Commit[];
  /** Files changed in selected commit */
  commitFiles: FileChange[];
  /** Loading state for changes */
  isLoadingChanges: boolean;
  /** Loading state for history */
  isLoadingHistory: boolean;
  /** Loading state for commit files */
  isLoadingCommitFiles: boolean;
  /** Error if any */
  error: Error | null;
  /** Fetch diff data for a specific file (optionally for a specific commit) */
  fetchDiff: (filePath: string, commitSha?: string) => Promise<DiffData | null>;
  /** Fetch files changed in a specific commit */
  fetchCommitFiles: (commitSha: string) => Promise<void>;
  /** Refresh all data */
  refresh: () => Promise<void>;
}

/**
 * Convert API CommitInfo to UI Commit type
 * Types are structurally identical but come from different sources
 */
function toCommit(info: CommitInfo): Commit {
  return {
    sha: info.sha,
    shortSha: info.shortSha,
    message: info.message,
    author: info.author,
    date: info.date,
  };
}

/**
 * Hook for fetching git diff data from agent activity events
 *
 * Connects to Tauri backend commands:
 * - get_task_file_changes(taskId) -> FileChange[]
 * - get_file_diff(taskId, filePath) -> FileDiff
 * - get_task_commits(taskId) -> CommitInfo[]
 */
export function useGitDiff({
  taskId,
  enabled = true,
}: UseGitDiffOptions): UseGitDiffResult {
  const [changes, setChanges] = useState<FileChange[]>([]);
  const [commits, setCommits] = useState<Commit[]>([]);
  const [commitFiles, setCommitFiles] = useState<FileChange[]>([]);
  const [isLoadingChanges, setIsLoadingChanges] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [isLoadingCommitFiles, setIsLoadingCommitFiles] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Fetch file changes on mount/enable
  useEffect(() => {
    if (!enabled || !taskId) return;

    const fetchData = async () => {
      setIsLoadingChanges(true);
      setError(null);

      try {
        const fileChanges = await diffApi.getTaskFileChanges(taskId);
        setChanges(fileChanges);
      } catch (err) {
        setError(
          err instanceof Error ? err : new Error("Failed to fetch git data")
        );
        setChanges([]);
      } finally {
        setIsLoadingChanges(false);
      }
    };

    fetchData();
  }, [enabled, taskId]);

  // Fetch commit history on mount/enable (only requires taskId)
  useEffect(() => {
    if (!enabled || !taskId) return;

    const fetchCommits = async () => {
      setIsLoadingHistory(true);

      try {
        const commitInfos = await diffApi.getTaskCommits(taskId);
        setCommits(commitInfos.map(toCommit));
      } catch {
        // Silently fail for commits - not critical and task may not have a branch yet
        setCommits([]);
      } finally {
        setIsLoadingHistory(false);
      }
    };

    fetchCommits();
  }, [enabled, taskId]);

  // Fetch diff for a specific file (optionally for a specific commit)
  const fetchDiff = useCallback(
    async (filePath: string, commitSha?: string): Promise<DiffData | null> => {
      if (!enabled || !taskId) return null;

      try {
        // Use commit-specific diff API if commitSha provided, otherwise use overall diff
        const fileDiff = commitSha
          ? await diffApi.getCommitFileDiff(taskId, commitSha, filePath)
          : await diffApi.getFileDiff(taskId, filePath);

        // Convert API response to DiffData format
        const diffData: DiffData = {
          filePath: fileDiff.filePath,
          oldContent: fileDiff.oldContent,
          newContent: fileDiff.newContent,
          hunks: [], // SimpleDiffView computes hunks from content
          language: fileDiff.language || getLanguageFromPath(filePath),
        };

        return diffData;
      } catch {
        return null;
      }
    },
    [enabled, taskId]
  );

  // Fetch files changed in a specific commit
  const fetchCommitFiles = useCallback(
    async (commitSha: string): Promise<void> => {
      if (!enabled || !taskId) return;

      setIsLoadingCommitFiles(true);
      try {
        const files = await diffApi.getCommitFileChanges(taskId, commitSha);
        setCommitFiles(files);
      } catch (err) {
        setError(
          err instanceof Error ? err : new Error("Failed to fetch commit files")
        );
        setCommitFiles([]);
      } finally {
        setIsLoadingCommitFiles(false);
      }
    },
    [enabled, taskId]
  );

  // Refresh all data
  const refresh = useCallback(async () => {
    if (!enabled || !taskId) return;

    setError(null);

    // Refresh commits
    setIsLoadingHistory(true);
    try {
      const commitInfos = await diffApi.getTaskCommits(taskId);
      setCommits(commitInfos.map(toCommit));
    } catch {
      setCommits([]);
    } finally {
      setIsLoadingHistory(false);
    }

    // Refresh file changes
    setIsLoadingChanges(true);
    try {
      const fileChanges = await diffApi.getTaskFileChanges(taskId);
      setChanges(fileChanges);
    } catch (err) {
      setError(
        err instanceof Error ? err : new Error("Failed to refresh git data")
      );
    } finally {
      setIsLoadingChanges(false);
    }
  }, [enabled, taskId]);

  return {
    changes,
    commits,
    commitFiles,
    isLoadingChanges,
    isLoadingHistory,
    isLoadingCommitFiles,
    error,
    fetchDiff,
    fetchCommitFiles,
    refresh,
  };
}
