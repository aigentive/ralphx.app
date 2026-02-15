/**
 * useTeamActions — Mutation hooks for team operations
 *
 * Provides sendTeamMessage, stopTeammate, and stopTeam mutations
 * following the TanStack Query mutation pattern.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { sendTeamMessage, stopTeammate, stopTeam } from "@/api/team";
import { teamKeys } from "@/hooks/useTeamStatus";
import type { ContextType } from "@/types/chat-conversation";

// ============================================================================
// Hook
// ============================================================================

export function useTeamActions(contextType: ContextType, contextId: string) {
  const queryClient = useQueryClient();

  const invalidateTeamStatus = () => {
    queryClient.invalidateQueries({
      queryKey: teamKeys.status(contextType, contextId),
    });
  };

  const sendMessage = useMutation({
    mutationFn: ({ target, content }: { target: string; content: string }) =>
      sendTeamMessage(contextType, contextId, target, content),
    onSuccess: invalidateTeamStatus,
  });

  const stopTeammateMutation = useMutation({
    mutationFn: (teammateName: string) =>
      stopTeammate(contextType, contextId, teammateName),
    onSuccess: invalidateTeamStatus,
  });

  const stopTeamMutation = useMutation({
    mutationFn: () => stopTeam(contextType, contextId),
    onSuccess: invalidateTeamStatus,
  });

  return {
    sendTeamMessage: sendMessage,
    stopTeammate: stopTeammateMutation,
    stopTeam: stopTeamMutation,
  };
}
