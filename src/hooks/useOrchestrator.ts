/**
 * useOrchestrator - Hook for interacting with the Orchestrator agent
 *
 * Provides mutations for sending messages to the orchestrator and
 * automatically invalidates related queries when proposals are created.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import type { OrchestratorMessageResponse } from "@/api/chat";

/**
 * Hook for sending messages to the orchestrator
 *
 * @param sessionId - The ideation session ID
 * @returns Mutation for sending messages
 */
export function useOrchestratorMessage(sessionId: string) {
  const queryClient = useQueryClient();

  return useMutation<OrchestratorMessageResponse, Error, string>({
    mutationFn: async (content: string) => {
      return chatApi.sendOrchestratorMessage(sessionId, content);
    },
    onSuccess: (data) => {
      // Invalidate session messages to show the new messages
      queryClient.invalidateQueries({
        queryKey: ["session-messages", sessionId],
      });

      // If proposals were created, invalidate the proposals query
      if (data.proposalsCreated.length > 0) {
        queryClient.invalidateQueries({
          queryKey: ["session-proposals", sessionId],
        });
      }

      // Also invalidate the session data query
      queryClient.invalidateQueries({
        queryKey: ["ideation-session", sessionId],
      });
    },
    onError: (error) => {
      console.error("Failed to send orchestrator message:", error);
    },
  });
}

/**
 * Hook for checking if orchestrator is available
 */
export function useOrchestratorAvailability() {
  return useMutation<boolean, Error, void>({
    mutationFn: async () => {
      return chatApi.isOrchestratorAvailable();
    },
  });
}
