/**
 * useVerificationBootstrap — hydrates the verification confirmation queue on
 * app startup and project switch.
 *
 * Calls GET /api/verification/pending-confirmations on mount and whenever
 * activeProjectId changes. Merges results into the existing queue via
 * hydrateVerificationQueue (deduplicated, never replaces).
 *
 * Errors are silent (console.warn) — the real-time event path is the fallback
 * for new confirmations; only pre-restart stale entries would be missed on failure.
 */

import { useEffect } from "react";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { verificationApi } from "@/api/verification";
import { logger } from "@/lib/logger";

export function useVerificationBootstrap() {
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const hydrateVerificationQueue = useUiStore((s) => s.hydrateVerificationQueue);

  useEffect(() => {
    if (!activeProjectId) return;

    verificationApi.getPendingVerificationConfirmations(activeProjectId)
      .then((items) => {
        const sessionIds = items.map((item) => item.session_id);
        hydrateVerificationQueue(sessionIds);
        logger.debug(
          "[VerificationBootstrap] Hydrated",
          sessionIds.length,
          "pending verification(s) for project",
          activeProjectId
        );
      })
      .catch((err: Error) => {
        console.warn("[VerificationBootstrap] Failed to hydrate verification queue:", err.message);
      });
  }, [activeProjectId, hydrateVerificationQueue]);
}
