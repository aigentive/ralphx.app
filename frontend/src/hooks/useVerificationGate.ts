/**
 * useVerificationGate — shared canAccept logic for ideation session acceptance.
 *
 * Centralises the "can this session be accepted" decision so PlanDisplay,
 * ProposalsToolbar, and AcceptModal all use the same rules.
 */

import type { IdeationSessionResponse } from "@/api/ideation";
import type { VerificationStatus } from "@/types/ideation";

export interface VerificationGateResult {
  /** Whether the session plan can be accepted right now */
  canAccept: boolean;
  /** Human-readable reason why acceptance is blocked (undefined when canAccept=true) */
  reason: string | undefined;
  /** Current verification status */
  status: VerificationStatus;
}

/**
 * Determines whether the session plan can be accepted based on its
 * verification state.
 *
 * Rules:
 * - `verified` → can accept
 * - `skipped` → can accept (user explicitly opted out)
 * - `unverified` → blocked: must start verification first
 * - `reviewing` → blocked: verification loop is running
 * - `needs_revision` → blocked: gaps found, plan needs correction
 *
 * @param session IdeationSessionResponse (camelCase from API layer)
 * @returns VerificationGateResult with canAccept, reason, and status
 */
export function useVerificationGate(
  session: Pick<
    IdeationSessionResponse,
    "verificationStatus" | "verificationInProgress"
  > | null
): VerificationGateResult {
  if (!session) {
    // Safe default: no session data means no verification config, allow accept
    return {
      canAccept: true,
      reason: undefined,
      status: "unverified",
    };
  }

  const status = session.verificationStatus;
  const inProgress = session.verificationInProgress;

  if (status === "verified") {
    return { canAccept: true, reason: undefined, status };
  }

  if (status === "skipped") {
    return { canAccept: true, reason: undefined, status };
  }

  if (status === "reviewing" || inProgress) {
    return {
      canAccept: false,
      reason: "Verification is in progress. Wait for it to complete.",
      status: "reviewing",
    };
  }

  if (status === "needs_revision") {
    return {
      canAccept: false,
      reason: "Plan has unresolved gaps. Correct the plan or skip verification.",
      status,
    };
  }

  // unverified (default)
  return {
    canAccept: false,
    reason: "Plan has not been verified. Run verification or skip.",
    status: "unverified",
  };
}
