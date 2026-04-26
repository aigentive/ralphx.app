import {
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { AgentConversationWorkspace } from "@/api/chat";
import { useProjects } from "@/hooks/useProjects";
import { useResponsiveSidebarLayout } from "@/hooks/useResponsiveSidebarLayout";
import { useChatStore } from "@/stores/chatStore";
import { useAgentSessionStore } from "@/stores/agentSessionStore";
import type { AgentConversation } from "./agentConversations";
import { useAgentArtifactController } from "./useAgentArtifactController";
import { AgentsArtifactPaneRegion } from "./AgentsArtifactPaneRegion";
import { AgentsTerminalRegion } from "./AgentsTerminalRegion";
import { useAgentConversationTitleEvents } from "./useAgentConversationTitleEvents";
import { useProjectAgentBridgeEvents } from "./useProjectAgentBridgeEvents";
import { useAgentArtifactResize } from "./useAgentArtifactResize";
import { useAgentsSelectionModel } from "./useAgentsSelectionModel";
import { useAgentsWorkspaceModel } from "./useAgentsWorkspaceModel";
import { useAgentsAttachedIdeation } from "./useAgentsAttachedIdeation";
import { useAgentsAutoTitle } from "./useAgentsAutoTitle";
import { useAgentsActiveComposerControls } from "./useAgentsActiveComposerControls";
import { useAgentWorkspacePublisher } from "./useAgentWorkspacePublisher";
import { useStartAgentConversation } from "./useStartAgentConversation";
import { useAgentConversationLookup } from "./useAgentConversationLookup";
import { useAgentConversationActions } from "./useAgentConversationActions";
import { AgentsShellLayout } from "./AgentsShellLayout";
import { AgentsActiveConversationPanel } from "./AgentsActiveConversationPanel";
import { AgentsStartConversationPanel } from "./AgentsStartConversationPanel";
import { useAgentArtifactActions } from "./useAgentArtifactActions";
import { useAgentConversationInvalidation } from "./useAgentConversationInvalidation";

const AGENTS_SIDEBAR_COLLAPSE_STORAGE_KEY = "ralphx-agents-sidebar-collapsed";

interface AgentsViewProps {
  projectId: string;
  onCreateProject: () => void;
}

export function AgentsView({
  projectId,
  onCreateProject,
}: AgentsViewProps) {
  const queryClient = useQueryClient();
  const [showArchived, setShowArchived] = useState(false);
  const [optimisticConversationsById, setOptimisticConversationsById] = useState<
    Record<string, AgentConversation>
  >({});
  const [optimisticWorkspacesByConversationId, setOptimisticWorkspacesByConversationId] =
    useState<Record<string, AgentConversationWorkspace>>({});
  const [optimisticSelectedConversationId, setOptimisticSelectedConversationId] =
    useState<string | null>(null);
  const {
    artifactWidthCss,
    handleArtifactResizeReset,
    handleArtifactResizeStart,
    isArtifactResizing,
    splitContainerRef,
  } = useAgentArtifactResize();
  const syncedProjectIdRef = useRef<string | null>(null);
  const {
    sidebarWidth,
    isCollapsed: isSidebarCollapsed,
    isOverlayOpen: isSidebarOverlayOpen,
    toggleCollapse: toggleSidebarCollapse,
    closeOverlay: closeSidebarOverlay,
    suppressTransition: suppressSidebarTransition,
  } = useResponsiveSidebarLayout({
    storageKey: AGENTS_SIDEBAR_COLLAPSE_STORAGE_KEY,
    largeWidth: 340,
    mediumWidth: 276,
  });
  const { data: projects = [], isLoading: isLoadingProjects } = useProjects();
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

  const focusedProjectId = useAgentSessionStore((s) => s.focusedProjectId);
  const selectedProjectId = useAgentSessionStore((s) => s.selectedProjectId);
  const storedSelectedConversationId = useAgentSessionStore((s) => s.selectedConversationId);
  const runtimeByConversationId = useAgentSessionStore((s) => s.runtimeByConversationId);
  const lastRuntimeByProjectId = useAgentSessionStore((s) => s.lastRuntimeByProjectId);
  const setFocusedProject = useAgentSessionStore((s) => s.setFocusedProject);
  const selectConversation = useAgentSessionStore((s) => s.selectConversation);
  const clearSelection = useAgentSessionStore((s) => s.clearSelection);
  const setRuntimeForConversation = useAgentSessionStore((s) => s.setRuntimeForConversation);
  const clearAgentConversationSelection = useCallback(() => {
    setOptimisticSelectedConversationId(null);
    clearSelection();
  }, [clearSelection]);
  const [terminalChatDockElement, setTerminalChatDockElement] =
    useState<HTMLDivElement | null>(null);
  const [terminalPanelDockElement, setTerminalPanelDockElement] =
    useState<HTMLDivElement | null>(null);
  const {
    activeConversation,
    activeProjectId,
    defaultProjectId,
    focusedConversations,
    selectedConversationFallback,
    selectedConversationId,
    selectedConversationMessages,
  } = useAgentsSelectionModel({
    clearAgentConversationSelection,
    focusedProjectId,
    optimisticConversationsById,
    optimisticSelectedConversationId,
    projectId,
    projects,
    selectedProjectId,
    showArchived,
    storedSelectedConversationId,
  });
  const {
    activeConversationMode,
    activeConversationModeLocked,
    activeWorkspace,
    normalizedActiveRuntime,
    publishShortcutLabel,
    terminalUnavailableReason,
  } = useAgentsWorkspaceModel({
    activeConversation,
    optimisticWorkspacesByConversationId,
    runtimeByConversationId,
    selectedConversationId,
  });
  useAgentConversationTitleEvents(activeProjectId);

  useEffect(() => {
    if (!projectId || syncedProjectIdRef.current === projectId) {
      return;
    }
    syncedProjectIdRef.current = projectId;
    setFocusedProject(projectId);
  }, [projectId, setFocusedProject]);

  const findConversationById = useAgentConversationLookup({
    focusedConversations,
    selectedConversationFallback,
  });

  const invalidateProjectConversations = useAgentConversationInvalidation(queryClient);
  const {
    attachedIdeationSessionId,
    hasAutoOpenArtifacts,
  } = useAgentsAttachedIdeation({
    activeConversation,
    activeConversationMode,
    activeWorkspace,
    invalidateProjectConversations,
    selectedConversationMessages,
  });
  useProjectAgentBridgeEvents({
    conversation: activeConversation,
    attachedIdeationSessionId,
    projectId: activeProjectId,
  });
  const {
    openArtifactTab,
    scheduleArtifactPanePreload,
    setArtifactPaneVisibility,
    setArtifactTaskMode,
    toggleArtifactPaneVisibility,
  } = useAgentArtifactController({
    hasAutoOpenArtifacts,
    selectedConversationId,
  });

  const { clearAutoManagedTitle, handleAutoManagedTitle } = useAgentsAutoTitle({
    findConversationById,
    invalidateProjectConversations,
  });

  const handleStartAgentConversation = useStartAgentConversation({
    handleAutoManagedTitle,
    invalidateProjectConversations,
    queryClient,
    selectConversation,
    setActiveConversation,
    setFocusedProject,
    setOptimisticConversationsById,
    setOptimisticSelectedConversationId,
    setOptimisticWorkspacesByConversationId,
    setRuntimeForConversation,
  });

  const {
    handleArchiveConversation,
    handleArchiveProject,
    handleRenameConversation,
    handleRestoreConversation,
    handleSidebarCreateAgent,
    handleSidebarFocusProject,
    handleSidebarSelectConversation,
  } = useAgentConversationActions({
    activeProjectId,
    clearAgentConversationSelection,
    clearAutoManagedTitle,
    closeSidebarOverlay,
    findConversationById,
    focusedProjectId,
    invalidateProjectConversations,
    isSidebarOverlayOpen,
    projectId,
    projects,
    queryClient,
    selectConversation,
    selectedConversationId,
    selectedProjectId,
    setActiveConversation,
    setFocusedProject,
    setOptimisticSelectedConversationId,
  });

  const {
    handleOpenPublishPane,
    handlePreloadArtifacts,
    handleSelectArtifact,
  } = useAgentArtifactActions({
    hasAutoOpenArtifacts,
    openArtifactTab,
    scheduleArtifactPanePreload,
    selectedConversationId,
    setArtifactPaneVisibility,
  });

  const handleAgentUserMessageSent = useCallback(
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

  const { handlePublishWorkspace, publishingConversationId } =
    useAgentWorkspacePublisher({
      activeWorkspace,
      findConversationById,
      invalidateProjectConversations,
      optimisticWorkspacesByConversationId,
      queryClient,
      selectedConversationId,
    });

  const {
    activeProjectOptions,
    defaultRuntime,
    handleActiveConversationModeChange,
    handleActiveModelChange,
    switchingConversationModeId,
  } = useAgentsActiveComposerControls({
    activeConversation,
    activeConversationModeLocked,
    activeProjectId,
    activeWorkspace,
    defaultProjectId,
    invalidateProjectConversations,
    lastRuntimeByProjectId,
    normalizedActiveRuntime,
    projects,
    queryClient,
    runtimeByConversationId,
    selectedConversationId,
    setRuntimeForConversation,
  });

  const sidebarProps = {
    projects,
    focusedProjectId: focusedProjectId ?? defaultProjectId,
    selectedConversationId,
    pinnedConversation: selectedConversationFallback,
    onFocusProject: handleSidebarFocusProject,
    onSelectConversation: handleSidebarSelectConversation,
    onCreateAgent: handleSidebarCreateAgent,
    onCreateProject,
    onArchiveProject: handleArchiveProject,
    onRenameConversation: handleRenameConversation,
    onArchiveConversation: handleArchiveConversation,
    onRestoreConversation: handleRestoreConversation,
    showArchived,
    onShowArchivedChange: setShowArchived,
  } as const;

  return (
    <AgentsShellLayout
      isSidebarCollapsed={isSidebarCollapsed}
      isSidebarOverlayOpen={isSidebarOverlayOpen}
      onCloseSidebarOverlay={closeSidebarOverlay}
      onToggleSidebarCollapse={toggleSidebarCollapse}
      sidebarProps={sidebarProps}
      sidebarWidth={sidebarWidth}
      splitContainerRef={splitContainerRef}
      suppressSidebarTransition={suppressSidebarTransition}
    >
          {activeProjectId && selectedConversationId && activeConversation ? (
            <AgentsActiveConversationPanel
              activeConversation={activeConversation}
              activeConversationMode={activeConversationMode}
              activeConversationModeLocked={activeConversationModeLocked}
              activeProjectId={activeProjectId}
              activeProjectOptions={activeProjectOptions}
              activeWorkspace={activeWorkspace}
              attachedIdeationSessionId={attachedIdeationSessionId}
              hasAutoOpenArtifacts={hasAutoOpenArtifacts}
              normalizedActiveRuntime={normalizedActiveRuntime}
              onActiveConversationModeChange={handleActiveConversationModeChange}
              onActiveModelChange={handleActiveModelChange}
              onAgentUserMessageSent={handleAgentUserMessageSent}
              onOpenPublishPane={handleOpenPublishPane}
              onPreloadArtifacts={handlePreloadArtifacts}
              onPublishWorkspace={handlePublishWorkspace}
              onRenameConversation={handleRenameConversation}
              onSelectArtifact={handleSelectArtifact}
              onToggleArtifacts={toggleArtifactPaneVisibility}
              publishShortcutLabel={publishShortcutLabel}
              publishingConversationId={publishingConversationId}
              selectedConversationId={selectedConversationId}
              setTerminalChatDockElement={setTerminalChatDockElement}
              switchingConversationModeId={switchingConversationModeId}
              terminalUnavailableReason={terminalUnavailableReason}
            />
          ) : (
            <AgentsStartConversationPanel
              projects={projects}
              defaultProjectId={defaultProjectId}
              defaultRuntime={defaultRuntime}
              isLoadingProjects={isLoadingProjects}
              onCreateProject={onCreateProject}
              onStartAgentConversation={handleStartAgentConversation}
            />
          )}

          {selectedConversationId && activeConversation ? (
            <AgentsArtifactPaneRegion
              conversationId={selectedConversationId}
              conversation={activeConversation}
              workspace={activeWorkspace}
              hasAutoOpenArtifacts={hasAutoOpenArtifacts}
              artifactWidthCss={artifactWidthCss}
              isArtifactResizing={isArtifactResizing}
              onResizeStart={handleArtifactResizeStart}
              onResizeReset={handleArtifactResizeReset}
              onTabChange={handleSelectArtifact}
              onTaskModeChange={(mode) =>
                setArtifactTaskMode(selectedConversationId, mode)
              }
              onPublishWorkspace={handlePublishWorkspace}
              isPublishingWorkspace={publishingConversationId === selectedConversationId}
              onClose={() => setArtifactPaneVisibility(selectedConversationId, false)}
              terminalUnavailableReason={terminalUnavailableReason}
              setTerminalPanelDockElement={setTerminalPanelDockElement}
            />
          ) : null}
          <AgentsTerminalRegion
            conversationId={selectedConversationId}
            workspace={activeWorkspace}
            terminalUnavailableReason={terminalUnavailableReason}
            hasAutoOpenArtifacts={hasAutoOpenArtifacts}
            chatDockElement={terminalChatDockElement}
            panelDockElement={terminalPanelDockElement}
            onOpenArtifactTab={openArtifactTab}
          />
    </AgentsShellLayout>
  );
}
