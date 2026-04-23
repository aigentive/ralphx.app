import { create } from "zustand";
import { persist } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";

export type AgentProvider = "claude" | "codex";
export type AgentArtifactTab = "plan" | "verification" | "proposal" | "tasks";
export type AgentTaskArtifactMode = "graph" | "kanban";
export type AgentProjectSort = "latest" | "az" | "za";

export interface AgentRuntimeSelection {
  provider: AgentProvider;
  modelId: string;
}

interface AgentArtifactState {
  isOpen: boolean;
  activeTab: AgentArtifactTab;
  taskMode: AgentTaskArtifactMode;
}

interface AgentSessionState {
  focusedProjectId: string | null;
  selectedProjectId: string | null;
  selectedConversationId: string | null;
  expandedProjectIds: Record<string, boolean>;
  showAllProjects: boolean;
  projectSort: AgentProjectSort;
  artifactByConversationId: Record<string, AgentArtifactState>;
  runtimeByConversationId: Record<string, AgentRuntimeSelection>;
  lastRuntimeByProjectId: Record<string, AgentRuntimeSelection>;
}

interface AgentSessionActions {
  setFocusedProject: (projectId: string | null) => void;
  selectConversation: (projectId: string, conversationId: string) => void;
  clearSelection: () => void;
  setProjectExpanded: (projectId: string, expanded: boolean) => void;
  toggleProjectExpanded: (projectId: string) => void;
  setShowAllProjects: (showAllProjects: boolean) => void;
  setProjectSort: (projectSort: AgentProjectSort) => void;
  setArtifactOpen: (conversationId: string, isOpen: boolean) => void;
  setArtifactTab: (conversationId: string, tab: AgentArtifactTab) => void;
  setTaskArtifactMode: (conversationId: string, mode: AgentTaskArtifactMode) => void;
  setRuntimeForConversation: (
    conversationId: string,
    projectId: string,
    runtime: AgentRuntimeSelection
  ) => void;
}

const DEFAULT_ARTIFACT_STATE: AgentArtifactState = {
  isOpen: false,
  activeTab: "plan",
  taskMode: "graph",
};

function ensureArtifactState(state: AgentSessionState, conversationId: string): AgentArtifactState {
  if (!state.artifactByConversationId[conversationId]) {
    state.artifactByConversationId[conversationId] = { ...DEFAULT_ARTIFACT_STATE };
  }
  return state.artifactByConversationId[conversationId];
}

export const useAgentSessionStore = create<AgentSessionState & AgentSessionActions>()(
  persist(
    immer((set) => ({
      focusedProjectId: null,
      selectedProjectId: null,
      selectedConversationId: null,
      expandedProjectIds: {},
      showAllProjects: false,
      projectSort: "latest",
      artifactByConversationId: {},
      runtimeByConversationId: {},
      lastRuntimeByProjectId: {},

      setFocusedProject: (projectId) =>
        set((state) => {
          state.focusedProjectId = projectId;
          if (projectId) {
            state.expandedProjectIds[projectId] = true;
          }
        }),

      selectConversation: (projectId, conversationId) =>
        set((state) => {
          state.focusedProjectId = projectId;
          state.selectedProjectId = projectId;
          state.selectedConversationId = conversationId;
          state.expandedProjectIds[projectId] = true;
        }),

      clearSelection: () =>
        set((state) => {
          state.selectedProjectId = null;
          state.selectedConversationId = null;
        }),

      setProjectExpanded: (projectId, expanded) =>
        set((state) => {
          state.expandedProjectIds[projectId] = expanded;
        }),

      toggleProjectExpanded: (projectId) =>
        set((state) => {
          state.expandedProjectIds[projectId] = !(state.expandedProjectIds[projectId] ?? true);
        }),

      setShowAllProjects: (showAllProjects) =>
        set((state) => {
          state.showAllProjects = showAllProjects;
        }),

      setProjectSort: (projectSort) =>
        set((state) => {
          state.projectSort = projectSort;
        }),

      setArtifactOpen: (conversationId, isOpen) =>
        set((state) => {
          ensureArtifactState(state, conversationId).isOpen = isOpen;
        }),

      setArtifactTab: (conversationId, tab) =>
        set((state) => {
          const artifactState = ensureArtifactState(state, conversationId);
          artifactState.activeTab = tab;
          artifactState.isOpen = true;
        }),

      setTaskArtifactMode: (conversationId, mode) =>
        set((state) => {
          ensureArtifactState(state, conversationId).taskMode = mode;
        }),

      setRuntimeForConversation: (conversationId, projectId, runtime) =>
        set((state) => {
          state.runtimeByConversationId[conversationId] = runtime;
          state.lastRuntimeByProjectId[projectId] = runtime;
        }),
    })),
    {
      name: "ralphx-agent-session-store",
      partialize: (state) => ({
        focusedProjectId: state.focusedProjectId,
        selectedProjectId: state.selectedProjectId,
        selectedConversationId: state.selectedConversationId,
        expandedProjectIds: state.expandedProjectIds,
        showAllProjects: state.showAllProjects,
        projectSort: state.projectSort,
        artifactByConversationId: state.artifactByConversationId,
        runtimeByConversationId: state.runtimeByConversationId,
        lastRuntimeByProjectId: state.lastRuntimeByProjectId,
      }),
    }
  )
);

export function selectArtifactState(conversationId: string | null) {
  return (state: AgentSessionState): AgentArtifactState =>
    conversationId
      ? state.artifactByConversationId[conversationId] ?? DEFAULT_ARTIFACT_STATE
      : DEFAULT_ARTIFACT_STATE;
}

export function selectHasStoredArtifactState(conversationId: string | null) {
  return (state: AgentSessionState): boolean =>
    conversationId
      ? Object.prototype.hasOwnProperty.call(state.artifactByConversationId, conversationId)
      : false;
}
