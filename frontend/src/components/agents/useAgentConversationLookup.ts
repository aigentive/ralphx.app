import { useCallback } from "react";

import type { AgentConversation } from "./agentConversations";

interface UseAgentConversationLookupArgs {
  focusedConversations: { data?: AgentConversation[] };
  selectedConversationFallback: AgentConversation | null;
}

export function useAgentConversationLookup({
  focusedConversations,
  selectedConversationFallback,
}: UseAgentConversationLookupArgs) {
  const findConversationById = useCallback(
    (conversationId: string) =>
      focusedConversations.data?.find((item) => item.id === conversationId) ??
      (selectedConversationFallback?.id === conversationId
        ? selectedConversationFallback
        : null),
    [focusedConversations.data, selectedConversationFallback]
  );

  return findConversationById;
}
