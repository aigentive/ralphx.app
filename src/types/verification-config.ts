import { z } from "zod";

// ============================================================================
// Specialist Entry
// ============================================================================

export const SpecialistEntrySchema = z.object({
  name: z.string(),
  display_name: z.string(),
  description: z.string(),
  dispatch_mode: z.enum(["pre_round", "per_round"]),
  enabled_by_default: z.boolean(),
});

export type SpecialistEntry = z.infer<typeof SpecialistEntrySchema>;

export const SpecialistsResponseSchema = z.object({
  specialists: z.array(SpecialistEntrySchema),
});

export type SpecialistsResponse = z.infer<typeof SpecialistsResponseSchema>;

// ============================================================================
// Confirmation payloads
// ============================================================================

export const ConfirmVerificationPayloadSchema = z.object({
  session_id: z.string(),
  disabled_specialists: z.array(z.string()),
});

export type ConfirmVerificationPayload = z.infer<typeof ConfirmVerificationPayloadSchema>;

export const DismissVerificationPayloadSchema = z.object({
  session_id: z.string(),
});

export type DismissVerificationPayload = z.infer<typeof DismissVerificationPayloadSchema>;

// ============================================================================
// Pending verification event payload
// ============================================================================

export const PendingVerificationEventSchema = z.object({
  session_id: z.string(),
  session_title: z.string(),
  plan_artifact_id: z.string(),
});

export type PendingVerificationEvent = z.infer<typeof PendingVerificationEventSchema>;

// ============================================================================
// Pending confirmations API response
// ============================================================================

export const PendingVerificationConfirmationItemSchema = z.object({
  session_id: z.string(),
  title: z.string(),
  plan_artifact_id: z.string(),
});

export type PendingVerificationConfirmationItem = z.infer<typeof PendingVerificationConfirmationItemSchema>;

export const PendingVerificationConfirmationsResponseSchema = z.array(PendingVerificationConfirmationItemSchema);

export type PendingVerificationConfirmationsResponse = z.infer<typeof PendingVerificationConfirmationsResponseSchema>;

// ============================================================================
// Queue item (used in uiStore pendingVerificationQueue)
// ============================================================================

export interface PendingVerificationQueueItem {
  sessionId: string;
  sessionTitle: string;
  planArtifactId: string;
}
