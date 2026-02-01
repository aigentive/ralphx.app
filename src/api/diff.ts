// Tauri invoke wrappers for diff API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import {
  FileChangesResponseSchema,
  FileDiffSchema,
} from "./diff.schemas";
import {
  transformFileChange,
  transformFileDiff,
} from "./diff.transforms";
import type { FileChange, FileDiff } from "./diff.types";

// Re-export types for convenience
export type { FileChange, FileDiff, FileChangeStatus } from "./diff.types";

// Re-export schemas for consumers that need validation
export {
  FileChangeSchema,
  FileChangeStatusSchema,
  FileDiffSchema,
  FileChangesResponseSchema,
} from "./diff.schemas";

// Re-export transforms for consumers that need manual transformation
export { transformFileChange, transformFileDiff } from "./diff.transforms";

// ============================================================================
// Typed Invoke Helper
// ============================================================================

async function typedInvokeWithTransform<TRaw, TResult>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<TRaw>,
  transform: (raw: TRaw) => TResult
): Promise<TResult> {
  const result = await invoke(cmd, args);
  const validated = schema.parse(result);
  return transform(validated);
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Diff API wrappers for Tauri commands
 * Provides file change and diff data from agent execution
 */
export const diffApi = {
  /**
   * Get all files changed by the agent for a task
   * Extracts file paths from Write/Edit tool calls in activity events
   * and uses git to determine change status
   * @param taskId - The task ID to get file changes for
   * @param projectPath - The project path for git operations
   * @returns Array of file changes with status and line counts
   */
  getTaskFileChanges: (taskId: string, projectPath: string): Promise<FileChange[]> =>
    typedInvokeWithTransform(
      "get_task_file_changes",
      { taskId, projectPath },
      FileChangesResponseSchema,
      (changes) => changes.map(transformFileChange)
    ),

  /**
   * Get the diff content for a specific file
   * Compares current file content with git HEAD
   * @param filePath - The file path relative to project root
   * @param projectPath - The project path for git operations
   * @returns File diff with old and new content
   */
  getFileDiff: (filePath: string, projectPath: string): Promise<FileDiff> =>
    typedInvokeWithTransform(
      "get_file_diff",
      { filePath, projectPath },
      FileDiffSchema,
      transformFileDiff
    ),
} as const;
