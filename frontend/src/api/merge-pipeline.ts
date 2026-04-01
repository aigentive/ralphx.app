// Tauri invoke wrappers for merge pipeline API with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { MergePipelineResponseSchema } from "./merge-pipeline.schemas";
import { transformMergePipelineResponse } from "./merge-pipeline.transforms";
import type { MergePipelineResponse } from "./merge-pipeline.types";

// Re-export types for convenience
export type {
  MergePipelineTask,
  MergePipelineResponse,
} from "./merge-pipeline.types";

// Re-export schemas for consumers that need validation
export {
  MergePipelineTaskSchema,
  MergePipelineResponseSchema,
} from "./merge-pipeline.schemas";

// Re-export transforms for consumers that need manual transformation
export {
  transformMergePipelineTask,
  transformMergePipelineResponse,
} from "./merge-pipeline.transforms";

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
 * Merge pipeline API wrappers for Tauri commands
 */
export const mergePipelineApi = {
  /**
   * Get the merge pipeline.
   * If projectId is provided, results are scoped to that project.
   * Returns tasks in merge-related states grouped into active, waiting, and needs_attention
   * @returns Merge pipeline with active merges, waiting merges, and tasks needing attention
   */
  getMergePipeline: (projectId?: string): Promise<MergePipelineResponse> =>
    typedInvokeWithTransform(
      "get_merge_pipeline",
      { projectId: projectId ?? null },
      MergePipelineResponseSchema,
      transformMergePipelineResponse
    ),
};
