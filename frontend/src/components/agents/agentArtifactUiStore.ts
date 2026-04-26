import { create } from "zustand";
import type { AgentArtifactState } from "@/stores/agentSessionStore";

export const DEFAULT_AGENT_ARTIFACT_UI_STATE: AgentArtifactState = {
  isOpen: false,
  activeTab: "plan",
  taskMode: "graph",
};

interface AgentArtifactUiState {
  artifactByConversationId: Record<string, AgentArtifactState>;
}

interface AgentArtifactUiActions {
  setArtifactState: (conversationId: string, state: AgentArtifactState) => void;
  clearArtifactState: (conversationId: string) => void;
}

export const useAgentArtifactUiStore = create<
  AgentArtifactUiState & AgentArtifactUiActions
>((set) => ({
  artifactByConversationId: {},

  setArtifactState: (conversationId, state) =>
    set((current) => ({
      artifactByConversationId: {
        ...current.artifactByConversationId,
        [conversationId]: { ...state },
      },
    })),

  clearArtifactState: (conversationId) =>
    set((current) => {
      const next = { ...current.artifactByConversationId };
      delete next[conversationId];
      return { artifactByConversationId: next };
    }),
}));

export function selectOptimisticArtifactState(conversationId: string | null) {
  return (state: AgentArtifactUiState): AgentArtifactState | null =>
    conversationId ? state.artifactByConversationId[conversationId] ?? null : null;
}

