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
export const ExternalIdeationOverridesSchema = z.object({
  requireVerificationForAccept: z.boolean().nullable(),
  requireVerificationForProposals: z.boolean().nullable(),
  requireAcceptForFinalize: z.boolean().nullable(),
});

export type ExternalIdeationOverrides = z.infer<typeof ExternalIdeationOverridesSchema>;

export const IdeationSettingsSchema = z.object({
  planMode: IdeationPlanModeSchema,
  requirePlanApproval: z.boolean(),
  suggestPlansForComplex: z.boolean(),
  autoLinkProposals: z.boolean(),
  requireAcceptForFinalize: z.boolean(),
  requireVerificationForAccept: z.boolean(),
  requireVerificationForProposals: z.boolean(),
  externalOverrides: ExternalIdeationOverridesSchema,
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
  requireAcceptForFinalize: false,
  requireVerificationForAccept: false,
  requireVerificationForProposals: false,
  externalOverrides: {
    requireVerificationForAccept: null,
    requireVerificationForProposals: null,
    requireAcceptForFinalize: null,
  },
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
  require_accept_for_finalize: z.boolean(),
  require_verification_for_accept: z.boolean().default(false),
  require_verification_for_proposals: z.boolean().default(false),
  ext_require_verification_for_accept: z.number().nullable().default(null),
  ext_require_verification_for_proposals: z.number().nullable().default(null),
  ext_require_accept_for_finalize: z.number().nullable().default(null),
});

export type IdeationSettingsResponse = z.infer<typeof IdeationSettingsResponseSchema>;
