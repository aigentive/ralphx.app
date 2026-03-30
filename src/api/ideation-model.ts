// Tauri invoke wrappers for ideation model settings

import { typedInvoke } from "@/lib/tauri";
import { z } from "zod";

// ============================================================================
// Schema (camelCase — Rust uses #[serde(rename_all = "camelCase")] on response)
// ============================================================================

export const IdeationModelResponseSchema = z.object({
  primaryModel: z.string(),
  verifierModel: z.string(),
  effectivePrimaryModel: z.string(),
  effectiveVerifierModel: z.string(),
  primaryModelSource: z.string(),
  verifierModelSource: z.string(),
});

export type IdeationModelResponse = z.infer<typeof IdeationModelResponseSchema>;

// ============================================================================
// Default placeholder (preserves current YAML behavior: inherit everywhere)
// ============================================================================

export const defaultIdeationModelSettings: IdeationModelResponse = {
  primaryModel: "inherit",
  verifierModel: "inherit",
  effectivePrimaryModel: "sonnet",
  effectiveVerifierModel: "sonnet",
  primaryModelSource: "yaml_default",
  verifierModelSource: "yaml_default",
};

// ============================================================================
// API
// ============================================================================

export const ideationModelApi = {
  /**
   * Get model settings for global (projectId=null) or per-project row.
   * GET command uses flat params — no struct wrapping.
   */
  get: (projectId: string | null): Promise<IdeationModelResponse> =>
    typedInvoke(
      "get_ideation_model_settings",
      { projectId },
      IdeationModelResponseSchema
    ),

  /**
   * Update model settings for global or per-project row.
   * UPDATE command uses struct param wrapping under `input` key.
   * Only provided fields are updated (partial update).
   */
  update: (input: {
    projectId: string | null;
    primaryModel?: string;
    verifierModel?: string;
  }): Promise<IdeationModelResponse> =>
    typedInvoke(
      "update_ideation_model_settings",
      { input },
      IdeationModelResponseSchema
    ),
} as const;
