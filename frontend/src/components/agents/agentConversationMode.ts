import type {
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
} from "@/api/chat";

import type { AgentConversation } from "./agentConversations";

export function resolveConversationAgentMode(
  conversation: AgentConversation,
  workspace: AgentConversationWorkspace | null
): AgentConversationWorkspaceMode {
  return conversation.agentMode ?? workspace?.mode ?? "chat";
}

export function isWorkspaceModeLocked(workspace: AgentConversationWorkspace | null): boolean {
  return Boolean(workspace?.linkedIdeationSessionId || workspace?.linkedPlanBranchId);
}
