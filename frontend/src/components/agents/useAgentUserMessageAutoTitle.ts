import { useCallback } from "react";

import type { AgentConversation } from "./agentConversations";

interface UseAgentUserMessageAutoTitleArgs {
  activeProjectId: string | null;
  findConversationById: (conversationId: string) => AgentConversation | null;
  handleAutoManagedTitle: (input: {
    content: string;
    conversationId: string;
    targetProjectId: string;
    shouldSpawnSessionNamer: boolean;
  }) => void;
  selectedConversationId: string | null;
}

export function useAgentUserMessageAutoTitle({
  activeProjectId,
  findConversationById,
  handleAutoManagedTitle,
  selectedConversationId,
}: UseAgentUserMessageAutoTitleArgs) {
  return useCallback(
    ({ content, result }: { content: string; result: { conversationId: string } }) => {
      const conversationId = result.conversationId || selectedConversationId;
      if (!conversationId || !activeProjectId) {
        return;
      }
      handleAutoManagedTitle({
        content,
        conversationId,
        targetProjectId: activeProjectId,
        shouldSpawnSessionNamer: findConversationById(conversationId)?.contextType === "project",
      });
    },
    [
      activeProjectId,
      findConversationById,
      handleAutoManagedTitle,
      selectedConversationId,
    ]
  );
}
