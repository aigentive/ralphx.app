// Tauri invoke wrappers for ideation model settings

import { typedInvoke } from "@/lib/tauri";
import { z } from "zod";

// ============================================================================
// Schema (camelCase — Rust uses #[serde(rename_all = "camelCase")] on response)
// ============================================================================

export const IdeationModelResponseSchema = z.object({
  primaryModel: z.string(),
  verifierModel: z.string(),
  verifierSubagentModel: z.string(),
  effectivePrimaryModel: z.string(),
  effectiveVerifierModel: z.string(),
  effectiveVerifierSubagentModel: z.string(),
  primaryModelSource: z.string(),
  verifierModelSource: z.string(),
  verifierSubagentModelSource: z.string(),
});

export type IdeationModelResponse = z.infer<typeof IdeationModelResponseSchema>;

// ============================================================================
// Default placeholder (preserves current YAML behavior: inherit everywhere)
// ============================================================================

export const defaultIdeationModelSettings: IdeationModelResponse = {
  primaryModel: "inherit",
  verifierModel: "inherit",
  verifierSubagentModel: "inherit",
  effectivePrimaryModel: "",
  effectiveVerifierModel: "",
  effectiveVerifierSubagentModel: "",
  primaryModelSource: "",
  verifierModelSource: "",
  verifierSubagentModelSource: "",
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
    verifierSubagentModel?: string;
  }): Promise<IdeationModelResponse> =>
    typedInvoke(
      "update_ideation_model_settings",
      { input },
      IdeationModelResponseSchema
    ),
} as const;
