/**
 * useVerificationGate — shared canAccept logic for ideation session acceptance.
 *
 * Centralises the "can this session be accepted" decision so PlanDisplay,
 * ProposalsToolbar, and AcceptModal all use the same rules. The hook also
 * consults the authoritative verification status endpoint so accept controls
 * do not depend on visiting the Verification tab first.
 */

import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { IdeationSessionResponse, SessionWithDataResponse } from "@/api/ideation";
import { ideationApi } from "@/api/ideation";
import type { VerificationStatus } from "@/types/ideation";

const sessionWithDataQueryKey = (sessionId: string) =>
  ["ideation", "sessions", "detail", sessionId, "with-data"] as const;

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
    "id" | "planArtifactId" | "sessionPurpose" | "verificationStatus" | "verificationInProgress"
  > | null
): VerificationGateResult {
  const queryClient = useQueryClient();
  const hasPlan = !!session?.planArtifactId;

  const { data: currentVerificationData } = useQuery({
    queryKey: session ? ["verification", session.id, "current"] : ["verification", "none", "current"],
    queryFn: async () => {
      try {
        return await ideationApi.verification.getStatus(session?.id ?? "");
      } catch (err) {
        if (err instanceof Error && err.message.includes("404")) return null;
        throw err;
      }
    },
    enabled: Boolean(session?.id && hasPlan && session.sessionPurpose !== "verification"),
    staleTime: 30_000,
    retry: (failureCount: number, err: unknown) => {
      if (err instanceof Error && err.message.includes("404")) return false;
      return failureCount < 2;
    },
    retryDelay: (attempt) => Math.min(1000 * 2 ** attempt, 10000),
  });

  useEffect(() => {
    if (!session || !currentVerificationData) return;
    if (
      currentVerificationData.status === session.verificationStatus &&
      currentVerificationData.inProgress === session.verificationInProgress
    ) {
      return;
    }

    queryClient.setQueryData<SessionWithDataResponse | null>(
      sessionWithDataQueryKey(session.id),
      (old) =>
        old
          ? {
              ...old,
              session: {
                ...old.session,
                verificationStatus: currentVerificationData.status,
                verificationInProgress: currentVerificationData.inProgress,
              },
            }
          : old
    );
  }, [currentVerificationData, queryClient, session]);

  if (!session) {
    // Safe default: no session data means no verification config, allow accept
    return {
      canAccept: true,
      reason: undefined,
      status: "unverified",
    };
  }

  const status = currentVerificationData?.status ?? session.verificationStatus;
  const inProgress = currentVerificationData?.inProgress ?? session.verificationInProgress;

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
