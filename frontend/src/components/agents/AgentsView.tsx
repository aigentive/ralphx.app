import { useQueryClient } from "@tanstack/react-query";

import { useProjects } from "@/hooks/useProjects";
import { useAgentArtifactController } from "./useAgentArtifactController";
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
import { AgentsConversationMainRegion } from "./AgentsConversationMainRegion";
import { useAgentArtifactActions } from "./useAgentArtifactActions";
import { useAgentConversationInvalidation } from "./useAgentConversationInvalidation";
import { useAgentUserMessageAutoTitle } from "./useAgentUserMessageAutoTitle";
import { AgentsConversationSideRegions } from "./AgentsConversationSideRegions";
import { useAgentsSessionBindings } from "./useAgentsSessionBindings";
import { useSyncedAgentProjectFocus } from "./useSyncedAgentProjectFocus";
import { useAgentsOptimisticState } from "./useAgentsOptimisticState";
import { useAgentsTerminalDocks } from "./useAgentsTerminalDocks";
import { useAgentsSidebarState } from "./useAgentsSidebarState";

interface AgentsViewProps {
  projectId: string;
  onCreateProject: () => void;
}

export function AgentsView({
  projectId,
  onCreateProject,
}: AgentsViewProps) {
  const queryClient = useQueryClient();
  const {
    closeSidebarOverlay,
    isSidebarCollapsed,
    isSidebarOverlayOpen,
    setShowArchived,
    showArchived,
    sidebarWidth,
    suppressSidebarTransition,
    toggleSidebarCollapse,
  } = useAgentsSidebarState();
  const {
    optimisticConversationsById,
    optimisticSelectedConversationId,
    optimisticWorkspacesByConversationId,
    setOptimisticConversationsById,
    setOptimisticSelectedConversationId,
    setOptimisticWorkspacesByConversationId,
  } = useAgentsOptimisticState();
  const {
    artifactWidthCss,
    handleArtifactResizeReset,
    handleArtifactResizeStart,
    isArtifactResizing,
    splitContainerRef,
  } = useAgentArtifactResize();
  const { data: projects = [], isLoading: isLoadingProjects } = useProjects();
  const {
    clearAgentConversationSelection,
    focusedProjectId,
    lastRuntimeByProjectId,
    runtimeByConversationId,
    selectConversation,
    selectedProjectId,
    setActiveConversation,
    setFocusedProject,
    setRuntimeForConversation,
    storedSelectedConversationId,
  } = useAgentsSessionBindings({
    setOptimisticSelectedConversationId,
  });
  const {
    setTerminalChatDockElement,
    setTerminalPanelDockElement,
    terminalChatDockElement,
    terminalPanelDockElement,
  } = useAgentsTerminalDocks();
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
  useSyncedAgentProjectFocus(projectId, setFocusedProject);

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

  const handleAgentUserMessageSent = useAgentUserMessageAutoTitle({
    activeProjectId,
    findConversationById,
    handleAutoManagedTitle,
    selectedConversationId,
  });

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
          <AgentsConversationMainRegion
            activeConversation={activeConversation}
            activeConversationMode={activeConversationMode}
            activeConversationModeLocked={activeConversationModeLocked}
            activeProjectId={activeProjectId}
            activeProjectOptions={activeProjectOptions}
            activeWorkspace={activeWorkspace}
            attachedIdeationSessionId={attachedIdeationSessionId}
            defaultProjectId={defaultProjectId}
            defaultRuntime={defaultRuntime}
            hasAutoOpenArtifacts={hasAutoOpenArtifacts}
            isLoadingProjects={isLoadingProjects}
            normalizedActiveRuntime={normalizedActiveRuntime}
            onActiveConversationModeChange={handleActiveConversationModeChange}
            onActiveModelChange={handleActiveModelChange}
            onAgentUserMessageSent={handleAgentUserMessageSent}
            onCreateProject={onCreateProject}
            onOpenPublishPane={handleOpenPublishPane}
            onPreloadArtifacts={handlePreloadArtifacts}
            onPublishWorkspace={handlePublishWorkspace}
            onRenameConversation={handleRenameConversation}
            onSelectArtifact={handleSelectArtifact}
            onStartAgentConversation={handleStartAgentConversation}
            onToggleArtifacts={toggleArtifactPaneVisibility}
            projects={projects}
            publishShortcutLabel={publishShortcutLabel}
            publishingConversationId={publishingConversationId}
            selectedConversationId={selectedConversationId}
            setTerminalChatDockElement={setTerminalChatDockElement}
            switchingConversationModeId={switchingConversationModeId}
            terminalUnavailableReason={terminalUnavailableReason}
          />

          <AgentsConversationSideRegions
            activeConversation={activeConversation}
            activeWorkspace={activeWorkspace}
            artifactWidthCss={artifactWidthCss}
            chatDockElement={terminalChatDockElement}
            hasAutoOpenArtifacts={hasAutoOpenArtifacts}
            isArtifactResizing={isArtifactResizing}
            openArtifactTab={openArtifactTab}
            panelDockElement={terminalPanelDockElement}
            publishingConversationId={publishingConversationId}
            selectedConversationId={selectedConversationId}
            setArtifactPaneVisibility={setArtifactPaneVisibility}
            setArtifactTaskMode={setArtifactTaskMode}
            setTerminalPanelDockElement={setTerminalPanelDockElement}
            terminalUnavailableReason={terminalUnavailableReason}
            onPublishWorkspace={handlePublishWorkspace}
            onResizeReset={handleArtifactResizeReset}
            onResizeStart={handleArtifactResizeStart}
            onSelectArtifact={handleSelectArtifact}
          />
    </AgentsShellLayout>
  );
}
