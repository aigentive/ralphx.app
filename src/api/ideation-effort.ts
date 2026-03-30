// Tauri invoke wrappers for ideation effort settings

import { typedInvoke } from "@/lib/tauri";
import { z } from "zod";

// ============================================================================
// Schema (camelCase — Rust uses #[serde(rename_all = "camelCase")] on response)
// ============================================================================

export const IdeationEffortResponseSchema = z.object({
  primaryEffort: z.string(),
  verifierEffort: z.string(),
  effectivePrimary: z.string(),
  effectiveVerifier: z.string(),
  primarySource: z.string(),
  verifierSource: z.string(),
});

export type IdeationEffortResponse = z.infer<typeof IdeationEffortResponseSchema>;

// ============================================================================
// Default placeholder (preserves current YAML behavior: inherit everywhere)
// ============================================================================

export const defaultIdeationEffortSettings: IdeationEffortResponse = {
  primaryEffort: "inherit",
  verifierEffort: "inherit",
  effectivePrimary: "inherit",
  effectiveVerifier: "inherit",
  primarySource: "yaml_default",
  verifierSource: "yaml_default",
};

// ============================================================================
// API
// ============================================================================

export const ideationEffortApi = {
  /**
   * Get effort settings for global (projectId=null) or per-project row.
   * GET command uses flat params — no struct wrapping.
   */
  get: (projectId: string | null): Promise<IdeationEffortResponse> =>
    typedInvoke(
      "get_ideation_effort_settings",
      { projectId },
      IdeationEffortResponseSchema
    ),

  /**
   * Update effort settings for global or per-project row.
   * UPDATE command uses struct param wrapping under `input` key.
   * Only provided fields are updated (partial update).
   */
  update: (input: {
    projectId: string | null;
    primaryEffort?: string;
    verifierEffort?: string;
  }): Promise<IdeationEffortResponse> =>
    typedInvoke(
      "update_ideation_effort_settings",
      { input },
      IdeationEffortResponseSchema
    ),
} as const;
