// Ideation configuration types and Zod schemas
// Types for IdeationSettings and IdeationPlanMode

import { z } from "zod";

// ============================================================================
// Ideation Plan Mode
// ============================================================================

/**
 * Plan workflow mode values
 */
export const IDEATION_PLAN_MODE_VALUES = [
  "required",
  "optional",
  "parallel",
] as const;

export const IdeationPlanModeSchema = z.enum(IDEATION_PLAN_MODE_VALUES);
export type IdeationPlanMode = z.infer<typeof IdeationPlanModeSchema>;

// ============================================================================
// Ideation Settings
// ============================================================================

/**
 * Ideation settings schema matching Rust backend serialization
 */
export const IdeationSettingsSchema = z.object({
  planMode: IdeationPlanModeSchema,
  requirePlanApproval: z.boolean(),
  suggestPlansForComplex: z.boolean(),
  autoLinkProposals: z.boolean(),
});

export type IdeationSettings = z.infer<typeof IdeationSettingsSchema>;

/**
 * Default ideation settings (matches Rust backend defaults)
 */
export const defaultIdeationSettings: IdeationSettings = {
  planMode: "optional",
  requirePlanApproval: false,
  suggestPlansForComplex: true,
  autoLinkProposals: true,
};

// ============================================================================
// Response Schema (snake_case from Rust)
// ============================================================================

/**
 * Ideation settings response schema (snake_case from Rust)
 */
export const IdeationSettingsResponseSchema = z.object({
  plan_mode: z.string(),
  require_plan_approval: z.boolean(),
  suggest_plans_for_complex: z.boolean(),
  auto_link_proposals: z.boolean(),
});

export type IdeationSettingsResponse = z.infer<typeof IdeationSettingsResponseSchema>;
