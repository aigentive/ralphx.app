/**
 * useTeamActions — Mutation hooks for team operations
 *
 * Provides sendTeamMessage, stopTeammate, and stopTeam mutations
 * following the TanStack Query mutation pattern.
 *
 * Resolves team_name from teamStore internally (set by team:created event).
 */

import { useMemo } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { sendTeamMessage, stopTeammate, stopTeam } from "@/api/team";
import { useTeamStore } from "@/stores/teamStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { teamKeys } from "@/hooks/useTeamStatus";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Hook
// ============================================================================

export function useTeamActions(contextType: ContextType, contextId: string) {
  const queryClient = useQueryClient();
  const contextKey = useMemo(
    () => buildStoreKey(contextType, contextId),
    [contextType, contextId],
  );
  const teamName = useTeamStore((s) => s.activeTeams[contextKey]?.teamName ?? "");

  const invalidateTeamStatus = () => {
    queryClient.invalidateQueries({
      queryKey: teamKeys.status(contextType, contextId),
    });
  };

  const sendMessage = useMutation({
    mutationFn: ({ target, content }: { target: string; content: string }) =>
      sendTeamMessage(teamName, target, content),
    onSuccess: invalidateTeamStatus,
  });

  const stopTeammateMutation = useMutation({
    mutationFn: (teammateName: string) =>
      stopTeammate(teamName, teammateName),
    onSuccess: invalidateTeamStatus,
  });

  const stopTeamMutation = useMutation({
    mutationFn: () => stopTeam(teamName),
    onSuccess: invalidateTeamStatus,
  });

  return {
    sendTeamMessage: sendMessage,
    stopTeammate: stopTeammateMutation,
    stopTeam: stopTeamMutation,
  };
}
