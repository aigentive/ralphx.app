import type { IdeationSession } from "@/types/ideation";

/**
 * Prefer the current React Query payload when it belongs to the active session,
 * but let newer Tauri event data override fields that are known to race.
 */
export function resolveIdeationSession(
  fetchedSession: IdeationSession | undefined,
  activeSession: IdeationSession | null
): IdeationSession | null {
  const isFetchedSessionCurrent = fetchedSession?.id === activeSession?.id;
  const base = isFetchedSessionCurrent && fetchedSession ? fetchedSession : activeSession;

  if (!base || !activeSession || activeSession.id !== base.id) {
    return base;
  }

  const hasVerificationOverride = (activeSession.verificationUpdateSeq ?? 0) > 0;
  const hasPlanOverride = (activeSession.planUpdateSeq ?? 0) > 0;

  if (!hasVerificationOverride && !hasPlanOverride) {
    return base;
  }

  return {
    ...base,
    ...(hasVerificationOverride && {
      verificationStatus: activeSession.verificationStatus ?? base.verificationStatus,
      verificationInProgress:
        activeSession.verificationInProgress ?? base.verificationInProgress,
      gapScore:
        activeSession.gapScore !== undefined ? activeSession.gapScore : base.gapScore,
    }),
    ...(hasPlanOverride && {
      planArtifactId: activeSession.planArtifactId ?? base.planArtifactId,
      inheritedPlanArtifactId:
        activeSession.inheritedPlanArtifactId ?? base.inheritedPlanArtifactId ?? null,
    }),
  };
}
