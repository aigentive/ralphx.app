import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { getChildSessionStatus } from "@/api/chat";
import type { ChildSessionStatusResponse } from "@/api/chat";

export type { ChildSessionStatusResponse };

/**
 * TanStack Query hook for polling child session status and recent messages.
 *
 * Polling strategy:
 * - 5s interval when estimated_status !== "idle"
 * - 5s interval while the session's verification snapshot is reviewing
 * - Disabled when idle (or when sessionId is null/undefined)
 *
 * History mode guard:
 * - If the FIRST successful fetch returns estimated_status === "idle"
 *   AND recent_messages is empty, polling is permanently disabled
 *   (session is historical/completed — no point polling).
 */
export function useChildSessionStatus(
  sessionId: string | null | undefined,
  enabled = true
) {
  // useState triggers a re-render when history mode is detected, naturally
  // updating the `enabled` flag without accessing a ref during render.
  const [historyMode, setHistoryMode] = useState(false);

  const query = useQuery({
    queryKey: ["child-session-status", sessionId],
    queryFn: async () => {
      const data = await getChildSessionStatus(sessionId!);

      // First-fetch history mode guard: idle + no messages + no pending prompt -> permanent disable.
      // When pending_initial_prompt is set, keep polling so we detect when the drain spawns an agent.
      if (
        data.agent_state.estimated_status === "idle" &&
        data.recent_messages.length === 0 &&
        !data.pending_initial_prompt
      ) {
        setHistoryMode(true);
      }

      return data;
    },
    enabled: enabled && !!sessionId && !historyMode,
    staleTime: 10_000,
    refetchOnWindowFocus: false,
    refetchInterval: (query) => {
      if (historyMode) return false;
      const data = query.state.data;
      if (!data) return false;
      // Keep polling when agent is active OR when a pending prompt is waiting to be drained.
      if (data.agent_state.estimated_status !== "idle") return 5_000;
      if (data.pending_initial_prompt) return 5_000;
      if (data.verification?.status === "reviewing") return 5_000;
      return false;
    },
  });

  return {
    ...query,
    lastEffectiveModel: query.data?.lastEffectiveModel ?? null,
  };
}
