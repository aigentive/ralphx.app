// Transform functions for converting snake_case merge pipeline API responses to camelCase frontend types

import { z } from "zod";
import {
  MergePipelineTaskSchema,
  MergePipelineResponseSchema,
} from "./merge-pipeline.schemas";
import type {
  MergePipelineTask,
  MergePipelineResponse,
} from "./merge-pipeline.types";

/**
 * Transform MergePipelineTaskSchema (snake_case) → MergePipelineTask (camelCase)
 */
export function transformMergePipelineTask(
  raw: z.infer<typeof MergePipelineTaskSchema>
): MergePipelineTask {
  return {
    taskId: raw.task_id,
    title: raw.title,
    internalStatus: raw.internal_status,
    sourceBranch: raw.source_branch,
    targetBranch: raw.target_branch,
    isDeferred: raw.is_deferred,
    isMainMergeDeferred: raw.is_main_merge_deferred,
    blockingBranch: raw.blocking_branch,
    conflictFiles: raw.conflict_files,
    errorContext: raw.error_context,
  };
}

/**
 * Transform MergePipelineResponseSchema (snake_case) → MergePipelineResponse (camelCase)
 */
export function transformMergePipelineResponse(
  raw: z.infer<typeof MergePipelineResponseSchema>
): MergePipelineResponse {
  return {
    active: raw.active.map(transformMergePipelineTask),
    waiting: raw.waiting.map(transformMergePipelineTask),
    needsAttention: raw.needs_attention.map(transformMergePipelineTask),
  };
}
