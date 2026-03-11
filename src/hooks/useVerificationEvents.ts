/**
 * useVerificationEvents — typed listener for plan_verification:status_changed events.
 *
 * Listens to the Tauri event emitted by the backend whenever verification
 * state changes (POST /verification, revert-and-skip, reconciliation reset, etc.)
 * and invalidates TanStack Query caches so UI reflects the latest state.
 *
 * Extended payload schema (B1):
 * {
 *   session_id: string,
 *   status: VerificationStatus,
 *   in_progress: boolean,
 *   round?: number,
 *   max_rounds?: number,
 *   gap_score?: number,
 *   convergence_reason?: string,
 *   current_gaps?: EventVerificationGap[],
 *   rounds?: EventRoundSummary[]
 * }
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useEventBus } from "@/providers/EventProvider";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import type { Unsubscribe } from "@/lib/event-bus";
import { logger } from "@/lib/logger";
import type { VerificationStatus, VerificationGap } from "@/types/ideation";
import type { VerificationStatusResponse } from "@/api/ideation.types";
import { PlanVerificationStatusChangedSchema } from "@/types/events";
export type { PlanVerificationStatusChangedEvent, PlanVerificationStatusChangedPayload } from "@/types/events";

// ============================================================================
// Hook
// ============================================================================

/**
 * Subscribes to `plan_verification:status_changed` events from the Tauri backend.
 *
 * On each event:
 * 1. Validates the D20 payload with Zod
 * 2. Updates the ideation store session inline (optimistic partial update)
 * 3. Cancels in-flight verification fetches to prevent stale overwrites (race fix)
 * 4. Fast path: populates verification cache directly with round guard + planVersion stamp
 * 5. Conditionally invalidates (only verification refetch when no fast-path data)
 *
 * Mount this once near the root of the ideation feature tree (e.g. alongside
 * `useIdeationEvents`).
 */
export function useVerificationEvents() {
  const bus = useEventBus();
  const updateSession = useIdeationStore((s) => s.updateSession);
  const queryClient = useQueryClient();

  useEffect(() => {
    logger.debug("[VerificationEvents] Setting up plan_verification:status_changed listener");
    const unsubscribes: Unsubscribe[] = [];

    unsubscribes.push(
      bus.subscribe<unknown>("plan_verification:status_changed", (payload) => {
        logger.debug("[VerificationEvents] Received plan_verification:status_changed:", payload);

        const parsed = PlanVerificationStatusChangedSchema.safeParse(payload);
        if (!parsed.success) {
          console.error(
            "Invalid plan_verification:status_changed event:",
            parsed.error.message
          );
          return;
        }

        const {
          session_id: sessionId,
          status,
          in_progress: inProgress,
          gap_score: gapScore,
          round,
          max_rounds: maxRounds,
          convergence_reason: convergenceReason,
          current_gaps: currentGaps,
          rounds,
        } = parsed.data;

        // Partial store update so components re-render immediately (sync)
        // Increment verificationUpdateSeq so resolvedSession merge prefers store over stale React Query data
        const currentSeq = useIdeationStore.getState().sessions[sessionId]?.verificationUpdateSeq ?? 0;
        updateSession(sessionId, {
          verificationStatus: status as VerificationStatus,
          verificationInProgress: inProgress,
          ...(gapScore !== undefined && { gapScore }),
          verificationUpdateSeq: currentSeq + 1,
        });

        // Toast notifications for terminal transitions (in_progress=false with terminal status)
        if (!inProgress) {
          if (status === "verified") {
            toast.success("Plan verified — no critical gaps remain.", { duration: 5000 });
          } else if (status === "needs_revision") {
            toast.warning("Verification complete — gaps found that need attention.", { duration: 6000 });
          }
        }

        // Async portion wrapped in IIFE — bus.subscribe callbacks must be synchronous
        void (async () => {
          // 1. Cancel in-flight verification fetches BEFORE setQueryData.
          //    This closes the race window where a background refetch started by a
          //    previous event could complete after setQueryData and overwrite fresh data.
          await queryClient.cancelQueries({ queryKey: ["verification", sessionId] });

          // 2. Fast path: if event carries full gap/round data, populate cache directly
          //    so UI updates instantly without waiting for a refetch round-trip.
          if (currentGaps !== undefined && rounds !== undefined) {
            const cached = queryClient.getQueryData<VerificationStatusResponse>(["verification", sessionId]);

            // Round guard: reject out-of-order events UNLESS verification was reset.
            // Reset sends status=unverified (round undefined) → always accept.
            // When reset happens cached round may be higher — allow new data through.
            const isStaleRound =
              cached?.currentRound !== undefined &&
              round !== undefined &&
              round < cached.currentRound &&
              status !== "unverified";

            if (isStaleRound) {
              logger.debug("[VerificationEvents] Skipping stale event: round", round, "< cached", cached?.currentRound);
            } else {
              const transformedGaps: VerificationGap[] = currentGaps.map((g) => ({
                severity: g.severity,
                category: g.category,
                description: g.description,
                ...(g.why_it_matters != null && { whyItMatters: g.why_it_matters }),
              }));

              // Stamp current plan version so staleness can be derived later by comparing
              // planArtifact.version (store) vs planVersion (verification cache).
              const planVersion = useIdeationStore.getState().planArtifact?.metadata.version;
              if (planVersion === undefined) {
                logger.debug(
                  "[VerificationEvents] planVersion undefined at stamp time — " +
                  "staleness comparison will fall through to 'not stale'"
                );
              }

              const cacheData: VerificationStatusResponse = {
                sessionId,
                status: status as VerificationStatusResponse["status"],
                inProgress,
                ...(round !== undefined && { currentRound: round }),
                ...(maxRounds !== undefined && { maxRounds }),
                ...(gapScore !== undefined && { gapScore }),
                ...(convergenceReason != null && { convergenceReason }),
                gaps: transformedGaps,
                rounds: [],  // Event rounds have different shape (fingerprints); safety net refetch fills this
                ...(planVersion !== undefined && { planVersion }),
              };
              queryClient.setQueryData(["verification", sessionId], cacheData);
              logger.debug("[VerificationEvents] setQueryData fast path for", sessionId, "round", round);
            }

            // Fast path has authoritative data — skip verification invalidation.
            // cancelQueries above also cancels user-initiated refetches (window refocus).
            // Acceptable: Tauri event is always more recent than in-flight HTTP response
            // (backend emits event AFTER DB write completes).
          } else {
            // No fast path data — must invalidate to trigger refetch.
            // This branch currently never fires (all emission points include full data)
            // but serves as safety net for future emission points.
            queryClient.invalidateQueries({ queryKey: ["verification", sessionId] });
          }

          // Always invalidate session queries (different data source, no race risk)
          queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
          queryClient.invalidateQueries({
            queryKey: ideationKeys.sessionWithData(sessionId),
          });
        })();
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, updateSession, queryClient]);
}
