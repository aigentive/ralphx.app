import { useQuery } from "@tanstack/react-query";

import { chatApi } from "@/api/chat";
import type { AgentConversationWorkspace } from "@/api/chat";
import type { AgentRuntimeSelection } from "@/stores/agentSessionStore";

import type { AgentConversation } from "./agentConversations";
import {
  isWorkspaceModeLocked,
  resolveConversationAgentMode,
} from "./agentConversationMode";
import {
  getAgentTerminalUnavailableReason,
  runtimeFromConversation,
} from "./agentConversationRuntime";
import { normalizeRuntimeSelection } from "./agentOptions";
import { useDeferredAgentHydration } from "./useDeferredAgentHydration";

interface UseAgentsWorkspaceModelArgs {
  activeConversation: AgentConversation | null;
  optimisticWorkspacesByConversationId: Record<string, AgentConversationWorkspace>;
  runtimeByConversationId: Record<string, AgentRuntimeSelection>;
  selectedConversationId: string | null;
}

export function useAgentsWorkspaceModel({
  activeConversation,
  optimisticWorkspacesByConversationId,
  runtimeByConversationId,
  selectedConversationId,
}: UseAgentsWorkspaceModelArgs) {
  const conversationWorkspaceQuery = useQuery({
    queryKey: ["agents", "conversation-workspace", selectedConversationId],
    queryFn: () => chatApi.getAgentConversationWorkspace(selectedConversationId!),
    enabled:
      !!selectedConversationId &&
      activeConversation?.contextType === "project",
    staleTime: 5_000,
  });
  const activeWorkspace =
    conversationWorkspaceQuery.data ??
    (selectedConversationId
      ? optimisticWorkspacesByConversationId[selectedConversationId] ?? null
      : null);
  const activeConversationMode =
    activeConversation?.contextType === "project"
      ? resolveConversationAgentMode(activeConversation, activeWorkspace)
      : null;
  const activeRuntime = selectedConversationId
    ? runtimeByConversationId[selectedConversationId] ??
      runtimeFromConversation(activeConversation) ??
      null
    : null;
  const normalizedActiveRuntime = normalizeRuntimeSelection(activeRuntime);
  const canHydrateActiveWorkspaceFreshness = useDeferredAgentHydration(
    selectedConversationId && activeWorkspace?.mode === "edit"
      ? selectedConversationId
      : null,
  );
  const activeWorkspaceFreshnessQuery = useQuery({
    queryKey: ["agents", "conversation-workspace-freshness", selectedConversationId],
    queryFn: () => chatApi.getAgentConversationWorkspaceFreshness(selectedConversationId!),
    enabled:
      canHydrateActiveWorkspaceFreshness &&
      !!selectedConversationId &&
      activeWorkspace?.mode === "edit" &&
      activeWorkspace.status !== "missing",
    staleTime: 5_000,
  });
  const publishShortcutLabel = activeWorkspaceFreshnessQuery.data?.isBaseAhead
    ? `Update from ${activeWorkspace?.baseRef ?? activeWorkspaceFreshnessQuery.data.baseRef}`
    : "Commit & Publish";
  const activeConversationModeLocked =
    activeConversationMode === "ideation" || isWorkspaceModeLocked(activeWorkspace);
  const terminalUnavailableReason = getAgentTerminalUnavailableReason(
    activeConversation,
    activeWorkspace,
  );
  return {
    activeConversationMode,
    activeConversationModeLocked,
    activeWorkspace,
    normalizedActiveRuntime,
    publishShortcutLabel,
    terminalUnavailableReason,
  };
}
