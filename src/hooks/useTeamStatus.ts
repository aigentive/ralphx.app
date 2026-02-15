/**
 * useTeamStatus — TanStack Query hook for polling team status
 *
 * Polls team status every 5s when a team is active. Driven by chatStore's
 * isTeamActive flag to enable/disable polling.
 */

import { useQuery } from "@tanstack/react-query";
import { useChatStore, selectIsTeamActive } from "@/stores/chatStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { getTeamStatus } from "@/api/team";
import type { ContextType } from "@/types/chat-conversation";
import { useMemo } from "react";

// ============================================================================
// Query Keys
// ============================================================================

export const teamKeys = {
  all: ["teams"] as const,
  status: (contextType: ContextType, contextId: string) =>
    [...teamKeys.all, "status", contextType, contextId] as const,
  messages: (contextType: ContextType, contextId: string) =>
    [...teamKeys.all, "messages", contextType, contextId] as const,
};

// ============================================================================
// Hook
// ============================================================================

export function useTeamStatus(contextType: ContextType, contextId: string) {
  const contextKey = useMemo(
    () => buildStoreKey(contextType, contextId),
    [contextType, contextId]
  );
  const isTeamActiveSelector = useMemo(() => selectIsTeamActive(contextKey), [contextKey]);
  const isTeamActive = useChatStore(isTeamActiveSelector);

  return useQuery({
    queryKey: teamKeys.status(contextType, contextId),
    queryFn: () => getTeamStatus(contextType, contextId),
    enabled: isTeamActive,
    refetchInterval: 5000,
  });
}
