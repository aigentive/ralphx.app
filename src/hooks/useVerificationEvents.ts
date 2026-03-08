/**
 * useVerificationEvents — typed listener for plan_verification:status_changed events.
 *
 * Listens to the Tauri event emitted by the backend whenever verification
 * state changes (POST /verification, revert-and-skip, reconciliation reset, etc.)
 * and invalidates TanStack Query caches so UI reflects the latest state.
 *
 * D20 payload schema:
 * {
 *   session_id: string,
 *   status: VerificationStatus,
 *   in_progress: boolean,
 *   round?: number,
 *   max_rounds?: number,
 *   gap_score?: number,
 *   convergence_reason?: string
 * }
 */

import { useEffect } from "react";
import { z } from "zod";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { useIdeationStore } from "@/stores/ideationStore";
import { ideationKeys } from "./useIdeation";
import type { Unsubscribe } from "@/lib/event-bus";
import { logger } from "@/lib/logger";
import type { VerificationStatus } from "@/types/ideation";

// ============================================================================
// D20 payload schema (snake_case — backend emits via serde_json::json!({}) without key transform)
// ============================================================================

const PlanVerificationStatusChangedSchema = z.object({
  session_id: z.string(),
  status: z.enum(["unverified", "reviewing", "verified", "needs_revision", "skipped"]),
  in_progress: z.boolean(),
  round: z.number().int().optional(),
  max_rounds: z.number().int().optional(),
  gap_score: z.number().int().optional(),
  convergence_reason: z.string().optional(),
});

export type PlanVerificationStatusChangedPayload = z.infer<
  typeof PlanVerificationStatusChangedSchema
>;

// Mapped camelCase view of the payload for consumers
export type PlanVerificationStatusChangedEvent = {
  sessionId: string;
  status: PlanVerificationStatusChangedPayload["status"];
  inProgress: boolean;
  round?: number;
  maxRounds?: number;
  gapScore?: number;
  convergenceReason?: string;
};

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
        } = parsed.data;

        // Partial store update so components re-render immediately
        updateSession(sessionId, {
          verificationStatus: status as VerificationStatus,
          verificationInProgress: inProgress,
          ...(gapScore !== undefined && { gapScore }),
        });

        // Full refetch to pick up any other fields that may have changed
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
