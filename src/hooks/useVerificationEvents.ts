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
 * 3. Invalidates session list and session-detail queries for a full refetch
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

        // Partial store update so components re-render immediately
        // Increment verificationUpdateSeq so resolvedSession merge prefers store over stale React Query data
        const currentSeq = useIdeationStore.getState().sessions[sessionId]?.verificationUpdateSeq ?? 0;
        updateSession(sessionId, {
          verificationStatus: status as VerificationStatus,
          verificationInProgress: inProgress,
          ...(gapScore !== undefined && { gapScore }),
          verificationUpdateSeq: currentSeq + 1,
        });

        // B1 fast path: if event carries full gap/round data, populate cache directly
        // so UI updates instantly without waiting for a refetch round-trip.
        if (currentGaps !== undefined && rounds !== undefined) {
          const transformedGaps: VerificationGap[] = currentGaps.map((g) => ({
            severity: g.severity,
            category: g.category,
            description: g.description,
            ...(g.why_it_matters != null && { whyItMatters: g.why_it_matters }),
          }));
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
          };
          queryClient.setQueryData(["verification", sessionId], cacheData);
          logger.debug("[VerificationEvents] setQueryData fast path for", sessionId);
        }

        // Safety net: always invalidate so a background refetch picks up authoritative server state
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessionWithData(sessionId),
        });
        // Invalidate verification endpoint so gaps + rounds re-fetch with latest data
        queryClient.invalidateQueries({ queryKey: ["verification", sessionId] });
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, updateSession, queryClient]);
}
