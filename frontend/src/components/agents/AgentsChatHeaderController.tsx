import { useCallback, useEffect, useRef } from "react";

import { useResolvedAgentArtifactState } from "./agentArtifactState";
import {
  cancelDeferredFrameJob,
  scheduleDeferredFrameJob,
  type DeferredFrameJob,
} from "./agentDeferredFrame";
import {
  AgentsChatHeader,
  type AgentsChatHeaderProps,
} from "./AgentsChatHeader";
import { preloadAgentTerminalExperience } from "./agentTerminalPreload";
import { useAgentTerminalStore } from "./agentTerminalStore";

interface AgentsChatHeaderControllerProps
  extends Omit<
    AgentsChatHeaderProps,
    | "artifactOpen"
    | "activeArtifactTab"
    | "onToggleArtifacts"
    | "terminalOpen"
    | "onToggleTerminal"
    | "onPreloadTerminal"
  > {
  hasAutoOpenArtifacts: boolean;
  onToggleArtifacts: (conversationId: string) => void;
}

export function AgentsChatHeaderController({
  conversation,
  hasAutoOpenArtifacts,
  terminalUnavailableReason = null,
  onToggleArtifacts,
  ...props
}: AgentsChatHeaderControllerProps) {
  const { artifactState, artifactPaneOpen } = useResolvedAgentArtifactState(
    conversation?.id ?? null,
    hasAutoOpenArtifacts,
  );
  const terminalOpen = useAgentTerminalStore((state) =>
    conversation?.id ? state.openByConversationId[conversation.id] ?? false : false,
  );
  const toggleTerminalOpen = useAgentTerminalStore((state) => state.toggleOpen);
  const terminalPreloadJobRef = useRef<DeferredFrameJob | null>(null);

  const cancelTerminalPreloadJob = useCallback(() => {
    cancelDeferredFrameJob(terminalPreloadJobRef.current);
    terminalPreloadJobRef.current = null;
  }, []);

  useEffect(() => () => cancelTerminalPreloadJob(), [cancelTerminalPreloadJob]);

  const handlePreloadTerminal = useCallback(() => {
    cancelTerminalPreloadJob();
    preloadAgentTerminalExperience();
  }, [cancelTerminalPreloadJob]);

  const scheduleTerminalPreload = useCallback(() => {
    cancelTerminalPreloadJob();
    terminalPreloadJobRef.current = scheduleDeferredFrameJob(() => {
      terminalPreloadJobRef.current = null;
      preloadAgentTerminalExperience();
    });
  }, [cancelTerminalPreloadJob]);

  const handleToggleTerminal = useCallback(() => {
    if (!conversation || terminalUnavailableReason) {
      return;
    }
    const nextOpen = !terminalOpen;
    toggleTerminalOpen(conversation.id);
    if (nextOpen) {
      scheduleTerminalPreload();
    } else {
      cancelTerminalPreloadJob();
    }
  }, [
    cancelTerminalPreloadJob,
    conversation,
    scheduleTerminalPreload,
    terminalOpen,
    terminalUnavailableReason,
    toggleTerminalOpen,
  ]);
  const handleToggleArtifacts = useCallback(() => {
    if (!conversation) {
      return;
    }
    onToggleArtifacts(conversation.id);
  }, [conversation, onToggleArtifacts]);

  return (
    <AgentsChatHeader
      {...props}
      conversation={conversation}
      artifactOpen={artifactPaneOpen}
      activeArtifactTab={artifactState.activeTab}
      terminalOpen={terminalOpen}
      terminalUnavailableReason={terminalUnavailableReason}
      onToggleTerminal={handleToggleTerminal}
      onPreloadTerminal={handlePreloadTerminal}
      onToggleArtifacts={handleToggleArtifacts}
    />
  );
}
