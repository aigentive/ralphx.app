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

  return useQuery({
    queryKey: ["child-session-status", sessionId],
    queryFn: async () => {
      const data = await getChildSessionStatus(sessionId!);

      // First-fetch history mode guard: idle + no messages → permanent disable
      if (
        data.agent_state.estimated_status === "idle" &&
        data.recent_messages.length === 0
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
      return data.agent_state.estimated_status !== "idle" ? 5_000 : false;
    },
  });
}
