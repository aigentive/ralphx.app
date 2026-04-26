import { useCallback } from "react";

import type { AgentArtifactTab } from "@/stores/agentSessionStore";

import { getAgentArtifactStateSnapshot } from "./agentArtifactState";

interface UseAgentArtifactActionsArgs {
  hasAutoOpenArtifacts: boolean;
  openArtifactTab: (conversationId: string, tab: AgentArtifactTab) => void;
  scheduleArtifactPanePreload: () => void;
  selectedConversationId: string | null;
  setArtifactPaneVisibility: (conversationId: string, isOpen: boolean) => void;
}

export function useAgentArtifactActions({
  hasAutoOpenArtifacts,
  openArtifactTab,
  scheduleArtifactPanePreload,
  selectedConversationId,
  setArtifactPaneVisibility,
}: UseAgentArtifactActionsArgs) {
  const handleSelectArtifact = useCallback(
    (tab: AgentArtifactTab) => {
      if (!selectedConversationId) {
        return;
      }
      const currentArtifactState = getAgentArtifactStateSnapshot(
        selectedConversationId,
        hasAutoOpenArtifacts,
      );
      if (currentArtifactState.isOpen && currentArtifactState.activeTab === tab) {
        setArtifactPaneVisibility(selectedConversationId, false);
        return;
      }
      openArtifactTab(selectedConversationId, tab);
    },
    [
      hasAutoOpenArtifacts,
      openArtifactTab,
      selectedConversationId,
      setArtifactPaneVisibility,
    ]
  );

  const handleOpenPublishPane = useCallback(() => {
    if (!selectedConversationId) {
      return;
    }
    openArtifactTab(selectedConversationId, "publish");
  }, [openArtifactTab, selectedConversationId]);

  const handlePreloadArtifacts = useCallback(() => {
    scheduleArtifactPanePreload();
  }, [scheduleArtifactPanePreload]);

  return {
    handleOpenPublishPane,
    handlePreloadArtifacts,
    handleSelectArtifact,
  };
}
