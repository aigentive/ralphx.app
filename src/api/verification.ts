/**
 * verification.ts — HTTP API wrappers for verification confirmation flow.
 *
 * Endpoints at :3847 backend. Follows same fetch pattern as ideation.ts acceptance section.
 */

import { SpecialistsResponseSchema } from "@/types/verification-config";
import type { SpecialistsResponse } from "@/types/verification-config";

export const verificationApi = {
  /**
   * Confirm verification — triggers verify with specialist config.
   * Handles both: confirming an existing PendingVerification entry (agent-triggered path)
   * and initiating fresh verification when no pending entry exists (user-initiated path).
   */
  confirm: async (sessionId: string, disabledSpecialists: string[]): Promise<{ status: string }> => {
    const res = await fetch(`http://localhost:3847/api/verification/confirm`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ session_id: sessionId, disabled_specialists: disabledSpecialists }),
    });
    if (!res.ok) {
      const body = await res.json().catch(() => ({})) as Record<string, unknown>;
      throw new Error((body as { error?: string }).error ?? `Confirm verification failed: ${res.status}`);
    }
    return await res.json() as { status: string };
  },

  /**
   * Dismiss verification — removes pending entry, session stays Unverified.
   * No-op if no pending entry exists.
   */
  dismiss: async (sessionId: string): Promise<{ status: string }> => {
    const res = await fetch(`http://localhost:3847/api/verification/dismiss`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ session_id: sessionId }),
    });
    if (!res.ok) {
      const body = await res.json().catch(() => ({})) as Record<string, unknown>;
      throw new Error((body as { error?: string }).error ?? `Dismiss verification failed: ${res.status}`);
    }
    return await res.json() as { status: string };
  },

  /**
   * Toggle auto-accept per-session on the backend (in-memory).
   * Also tracked in uiStore for frontend-only auto-accept logic.
   */
  setAutoAccept: async (sessionId: string, enabled: boolean): Promise<{ status: string }> => {
    const res = await fetch(`http://localhost:3847/api/verification/auto-accept`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ session_id: sessionId, enabled }),
    });
    if (!res.ok) {
      const body = await res.json().catch(() => ({})) as Record<string, unknown>;
      throw new Error((body as { error?: string }).error ?? `Set auto-accept failed: ${res.status}`);
    }
    return await res.json() as { status: string };
  },

  /**
   * Get specialist list from registry.
   * Returns specialists configured in ralphx.yaml verification.specialists.
   */
  getSpecialists: async (): Promise<SpecialistsResponse> => {
    const res = await fetch(`http://localhost:3847/api/verification/specialists`);
    if (!res.ok) {
      throw new Error(`Failed to get specialists: ${res.status}`);
    }
    return SpecialistsResponseSchema.parse(await res.json());
  },
} as const;
