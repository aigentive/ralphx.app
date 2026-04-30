import { useState } from "react";

import type { AgentConversationWorkspace } from "@/api/chat";
import type { AgentConversation } from "./agentConversations";

export function useAgentsOptimisticState() {
  const [optimisticConversationsById, setOptimisticConversationsById] = useState<
    Record<string, AgentConversation>
  >({});
  const [optimisticWorkspacesByConversationId, setOptimisticWorkspacesByConversationId] =
    useState<Record<string, AgentConversationWorkspace>>({});
  const [optimisticSelectedConversationId, setOptimisticSelectedConversationId] =
    useState<string | null>(null);

  return {
    optimisticConversationsById,
    optimisticSelectedConversationId,
    optimisticWorkspacesByConversationId,
    setOptimisticConversationsById,
    setOptimisticSelectedConversationId,
    setOptimisticWorkspacesByConversationId,
  };
}
