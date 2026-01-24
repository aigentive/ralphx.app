/**
 * useGitDiff - Hook for fetching git diff data for reviews
 *
 * Provides file changes, commit history, and diff data for a task
 * Used primarily in the ReviewsPanel with integrated DiffViewer
 */

import { useState, useCallback, useEffect } from "react";
import type { FileChange, Commit, DiffData } from "@/components/diff";

export interface UseGitDiffOptions {
  /** Task ID to fetch diff data for */
  taskId: string;
  /** Project path for git operations */
  projectPath?: string;
  /** Whether to enable the hook */
  enabled?: boolean;
}

export interface UseGitDiffResult {
  /** Current uncommitted file changes */
  changes: FileChange[];
  /** Commit history */
  commits: Commit[];
  /** Loading state for changes */
  isLoadingChanges: boolean;
  /** Loading state for history */
  isLoadingHistory: boolean;
  /** Error if any */
  error: Error | null;
  /** Fetch diff data for a specific file */
  fetchDiff: (filePath: string, commitSha?: string) => Promise<DiffData | null>;
  /** Refresh all data */
  refresh: () => Promise<void>;
}

/**
 * Mock implementation for git diff operations
 *
 * In the future, this will connect to Tauri backend commands:
 * - get_git_changes(projectPath) -> FileChange[]
 * - get_git_commits(projectPath, limit) -> Commit[]
 * - get_file_diff(projectPath, filePath, commitSha?) -> DiffData
 *
 * For now, returns mock data for UI development and testing
 */
export function useGitDiff({
  taskId,
  projectPath: _projectPath,
  enabled = true,
}: UseGitDiffOptions): UseGitDiffResult {
  const [changes, setChanges] = useState<FileChange[]>([]);
  const [commits, setCommits] = useState<Commit[]>([]);
  const [isLoadingChanges, setIsLoadingChanges] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Fetch changes on mount/enable
  useEffect(() => {
    if (!enabled || !taskId) return;

    const fetchData = async () => {
      setIsLoadingChanges(true);
      setIsLoadingHistory(true);
      setError(null);

      try {
        // Simulate network delay
        await new Promise((resolve) => setTimeout(resolve, 300));

        // Mock changes data based on taskId for deterministic testing
        const mockChanges: FileChange[] = [
          {
            path: "src/components/auth/LoginForm.tsx",
            status: "modified",
            additions: 25,
            deletions: 10,
          },
          {
            path: "src/hooks/useAuth.ts",
            status: "modified",
            additions: 15,
            deletions: 3,
          },
          {
            path: "src/lib/api/auth.ts",
            status: "added",
            additions: 45,
            deletions: 0,
          },
          {
            path: "src/types/auth.ts",
            status: "added",
            additions: 20,
            deletions: 0,
          },
        ];

        const mockCommits: Commit[] = [
          {
            sha: "a1b2c3d4e5f6789012345678901234567890abcd",
            shortSha: "a1b2c3d",
            message: "feat: implement login form validation",
            author: "Claude Worker",
            date: new Date(Date.now() - 1000 * 60 * 5), // 5 min ago
          },
          {
            sha: "b2c3d4e5f6789012345678901234567890abcde",
            shortSha: "b2c3d4e",
            message: "feat: add authentication hook",
            author: "Claude Worker",
            date: new Date(Date.now() - 1000 * 60 * 30), // 30 min ago
          },
          {
            sha: "c3d4e5f6789012345678901234567890abcdef",
            shortSha: "c3d4e5f",
            message: "chore: initial task setup",
            author: "Claude Worker",
            date: new Date(Date.now() - 1000 * 60 * 60), // 1 hr ago
          },
        ];

        setChanges(mockChanges);
        setCommits(mockCommits);
      } catch (err) {
        setError(
          err instanceof Error ? err : new Error("Failed to fetch git data")
        );
      } finally {
        setIsLoadingChanges(false);
        setIsLoadingHistory(false);
      }
    };

    fetchData();
  }, [enabled, taskId]);

  // Fetch diff for a specific file
  const fetchDiff = useCallback(
    async (filePath: string, commitSha?: string): Promise<DiffData | null> => {
      if (!enabled) return null;

      try {
        // Simulate network delay
        await new Promise((resolve) => setTimeout(resolve, 200));

        // Generate mock diff based on file path
        const fileName = filePath.split("/").pop() ?? "file";
        const isTypeScript = filePath.endsWith(".ts") || filePath.endsWith(".tsx");
        const language = isTypeScript ? "typescript" : "plaintext";

        // Mock diff content
        const oldContent = commitSha
          ? `// Previous version of ${fileName}\n// Commit: ${commitSha}\n\nexport function example() {\n  return "old";\n}\n`
          : `// ${fileName}\n\nexport function example() {\n  return "old";\n}\n`;

        const newContent = `// ${fileName}\n// Updated for task\n\nexport function example() {\n  // Improved implementation\n  return "new";\n}\n\nexport function newFunction() {\n  return "added";\n}\n`;

        const mockDiff: DiffData = {
          filePath,
          oldContent,
          newContent,
          hunks: [
            "@@ -1,5 +1,12 @@",
            `-// ${fileName}`,
            `+// ${fileName}`,
            "+// Updated for task",
            " ",
            " export function example() {",
            `-  return \"old\";`,
            `+  // Improved implementation`,
            `+  return \"new\";`,
            " }",
            "+",
            "+export function newFunction() {",
            `+  return \"added\";`,
            "+}",
          ],
          language,
        };

        return mockDiff;
      } catch {
        return null;
      }
    },
    [enabled]
  );

  // Refresh all data
  const refresh = useCallback(async () => {
    if (!enabled || !taskId) return;

    setIsLoadingChanges(true);
    setIsLoadingHistory(true);
    setError(null);

    try {
      await new Promise((resolve) => setTimeout(resolve, 300));
      // Data would be refetched from backend
    } catch (err) {
      setError(
        err instanceof Error ? err : new Error("Failed to refresh git data")
      );
    } finally {
      setIsLoadingChanges(false);
      setIsLoadingHistory(false);
    }
  }, [enabled, taskId]);

  return {
    changes,
    commits,
    isLoadingChanges,
    isLoadingHistory,
    error,
    fetchDiff,
    refresh,
  };
}
