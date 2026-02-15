/**
 * Mock Merge Pipeline API
 *
 * Mirrors the interface of src/api/merge-pipeline.ts with mock implementations.
 */

import type { MergePipelineResponse } from "@/api/merge-pipeline";

// ============================================================================
// Mock State
// ============================================================================

const mockMergePipeline: MergePipelineResponse = {
  active: [
    {
      taskId: "task-1",
      title: "Add JWT authentication",
      internalStatus: "merging",
      sourceBranch: "ralphx/app/task-a1b2c3",
      targetBranch: "ralphx/app/plan-a1b2",
      isDeferred: false,
      isMainMergeDeferred: false,
      blockingBranch: null,
      conflictFiles: ["src/auth.rs", "src/main.rs"],
      errorContext: null,
    },
  ],
  waiting: [
    {
      taskId: "task-2",
      title: "Fix login validation",
      internalStatus: "pending_merge",
      sourceBranch: "ralphx/app/task-d4e5f6",
      targetBranch: "main",
      isDeferred: true,
      isMainMergeDeferred: false,
      blockingBranch: "ralphx/app/task-a1b2c3",
      conflictFiles: null,
      errorContext: null,
    },
    {
      taskId: "task-3",
      title: "Refactor auth module",
      internalStatus: "pending_merge",
      sourceBranch: "ralphx/app/task-c3d4e5",
      targetBranch: "ralphx/app/plan-c3d4",
      isDeferred: true,
      isMainMergeDeferred: false,
      blockingBranch: "ralphx/app/task-a1b2c3",
      conflictFiles: null,
      errorContext: null,
    },
  ],
  needsAttention: [
    {
      taskId: "task-4",
      title: "Update user model",
      internalStatus: "merge_conflict",
      sourceBranch: "ralphx/app/task-u1v2w3",
      targetBranch: "main",
      isDeferred: false,
      isMainMergeDeferred: false,
      blockingBranch: null,
      conflictFiles: ["src/models/user.rs", "src/db/schema.rs", "migrations/2026-01-user.sql"],
      errorContext: "Agent couldn't resolve conflicts automatically",
    },
    {
      taskId: "task-5",
      title: "Add notifications",
      internalStatus: "merge_incomplete",
      sourceBranch: "ralphx/app/task-e5f6g7",
      targetBranch: "ralphx/app/plan-e5f6",
      isDeferred: false,
      isMainMergeDeferred: false,
      blockingBranch: null,
      conflictFiles: null,
      errorContext: "rebase failed: index.lock exists",
    },
  ],
};

// ============================================================================
// Mock Merge Pipeline API
// ============================================================================

export const mockMergePipelineApi = {
  getMergePipeline: async (): Promise<MergePipelineResponse> => {
    return { ...mockMergePipeline };
  },
} as const;
