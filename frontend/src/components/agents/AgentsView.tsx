import {
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import type {
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
} from "@/api/chat";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { chatKeys } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { useProjects } from "@/hooks/useProjects";
import { useResponsiveSidebarLayout } from "@/hooks/useResponsiveSidebarLayout";
import { useChatStore } from "@/stores/chatStore";
import {
  useAgentSessionStore,
  type AgentArtifactTab,
} from "@/stores/agentSessionStore";
import {
  getAgentConversationStoreKey,
  type AgentConversation,
} from "./agentConversations";
import {
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { getAgentArtifactStateSnapshot } from "./agentArtifactState";
import { useAgentArtifactController } from "./useAgentArtifactController";
import { AgentComposerProjectLine, AgentComposerSurface } from "./AgentComposerSurface";
import { AgentConversationBaseLine } from "./AgentConversationBaseLine";
import { AgentsArtifactPaneRegion } from "./AgentsArtifactPaneRegion";
import { AgentsChatHeaderController } from "./AgentsChatHeaderController";
import {
  AGENT_CONVERSATION_MODE_OPTIONS,
} from "./agentConversationMode";
import { AgentsStartComposer } from "./AgentsStartComposer";
import {
  AgentsTerminalDockHost,
  AgentsTerminalRegion,
} from "./AgentsTerminalRegion";
import {
  agentConversationKeys,
} from "./useProjectAgentConversations";
import { archivedConversationCountKey } from "./useArchivedConversationCounts";
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

const AGENTS_CHAT_CONTENT_WIDTH_CLASS = "max-w-[980px]";
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
  const [isStartingConversation, setIsStartingConversation] = useState(false);
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

  const invalidateProjectConversations = useCallback(
    async (targetProjectId: string) => {
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: agentConversationKeys.project(targetProjectId),
        }),
        queryClient.invalidateQueries({
          queryKey: chatKeys.conversationList("project", targetProjectId),
        }),
        queryClient.invalidateQueries({
          queryKey: archivedConversationCountKey(targetProjectId),
          refetchType: "active",
        }),
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() }),
      ]);
    },
    [queryClient]
  );
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
            <div className="flex-1 min-w-0 h-full flex flex-col">
              <div className="min-h-0 flex-1">
                <IntegratedChatPanel
                  key={selectedConversationId}
                  projectId={activeProjectId}
                  {...(activeConversation.contextType === "ideation"
                    ? { ideationSessionId: activeConversation.contextId }
                    : {})}
                  conversationIdOverride={selectedConversationId}
                  selectedTaskIdOverride={null}
                  storeContextKeyOverride={getAgentConversationStoreKey(activeConversation)}
                  agentProcessContextIdOverride={
                    activeConversation.contextType === "project"
                      ? selectedConversationId
                      : undefined
                  }
                  sendOptions={{
                    conversationId: selectedConversationId,
                    providerHarness: normalizedActiveRuntime.provider,
                    modelId: normalizedActiveRuntime.modelId,
                  }}
                  onUserMessageSent={handleAgentUserMessageSent}
                  hideHeaderSessionControls
                  hideSessionToolbar
                  surfaceBackground="var(--bg-base)"
                  contentWidthClassName={AGENTS_CHAT_CONTENT_WIDTH_CLASS}
                  inputContainerClassName="shrink-0 bg-transparent px-4 pb-4 pt-3"
                  renderComposer={(composerProps) => (
                    <>
                      <AgentComposerSurface
                        dataTestId="agents-conversation-composer"
                        actionTestId="agents-conversation-submit"
                        onSend={composerProps.onSend}
                        onStop={composerProps.onStop}
                        agentStatus={composerProps.agentStatus}
                        isSubmitting={composerProps.isSending}
                        isReadOnly={composerProps.isReadOnly}
                        autoFocus={composerProps.autoFocus}
                        placeholder="Ask the agent to plan, build, debug, or review something"
                        showHelperText={false}
                        hasQueuedMessages={composerProps.hasQueuedMessages}
                        onEditLastQueued={composerProps.onEditLastQueued}
                        attachments={composerProps.attachments}
                        enableAttachments={composerProps.enableAttachments}
                        onFilesSelected={composerProps.onFilesSelected}
                        onRemoveAttachment={composerProps.onRemoveAttachment}
                        attachmentsUploading={composerProps.attachmentsUploading}
                        {...(composerProps.value !== undefined
                          ? {
                              value: composerProps.value,
                              onChange: composerProps.onChange,
                            }
                          : {})}
                        {...(composerProps.questionMode !== undefined
                          ? { questionMode: composerProps.questionMode }
                          : {})}
                        submitLabel="Send"
                        {...(activeConversationMode
                          ? {
                              mode: {
                                value: activeConversationMode,
                                onValueChange: (value: string) =>
                                  handleActiveConversationModeChange(value as AgentConversationWorkspaceMode),
                                options: AGENT_CONVERSATION_MODE_OPTIONS,
                                disabled:
                                  activeConversationModeLocked ||
                                  composerProps.agentStatus !== "idle" ||
                                  switchingConversationModeId === selectedConversationId,
                              },
                            }
                          : {})}
                        project={{
                          value: activeProjectId,
                          onValueChange: () => undefined,
                          options: activeProjectOptions,
                          placeholder: "Current project",
                          disabled: true,
                        }}
                        provider={{
                          value: normalizedActiveRuntime.provider,
                          onValueChange: () => undefined,
                          options: AGENT_PROVIDER_OPTIONS,
                          disabled: true,
                        }}
                        model={{
                          value: normalizedActiveRuntime.modelId,
                          onValueChange: handleActiveModelChange,
                          options: AGENT_MODEL_OPTIONS[normalizedActiveRuntime.provider],
                        }}
                      />
                      <div className="mt-2 flex w-full flex-wrap items-center justify-between gap-2 px-2">
                        <AgentComposerProjectLine
                          value={activeProjectId}
                          onValueChange={() => undefined}
                          options={activeProjectOptions}
                          placeholder="Current project"
                          disabled
                        />
                        <AgentConversationBaseLine
                          workspace={activeWorkspace}
                        />
                      </div>
                    </>
                  )}
                  {...(activeConversation.contextType === "project" && attachedIdeationSessionId
                    ? { additionalQuestionSessionIds: [attachedIdeationSessionId] }
                    : {})}
                  headerContent={
                    <AgentsChatHeaderController
                      conversation={activeConversation}
                      workspace={activeWorkspace}
                      hasAutoOpenArtifacts={hasAutoOpenArtifacts}
                      terminalUnavailableReason={terminalUnavailableReason}
                      onRenameConversation={handleRenameConversation}
                      onPublishWorkspace={handlePublishWorkspace}
                      onOpenPublishPane={handleOpenPublishPane}
                      onPreloadArtifacts={handlePreloadArtifacts}
                      publishShortcutLabel={publishShortcutLabel}
                      isPublishingWorkspace={publishingConversationId === selectedConversationId}
                      onToggleArtifacts={toggleArtifactPaneVisibility}
                      onSelectArtifact={handleSelectArtifact}
                    />
                  }
                  emptyState={<div />}
                />
              </div>
              <AgentsTerminalDockHost
                dock="chat"
                conversationId={selectedConversationId}
                workspace={activeWorkspace}
                terminalUnavailableReason={terminalUnavailableReason}
                hasAutoOpenArtifacts={hasAutoOpenArtifacts}
                setDockElement={setTerminalChatDockElement}
              />
            </div>
          ) : (
            <div className="flex-1 min-w-0 h-full">
              <AgentsStartComposer
                projects={projects}
                defaultProjectId={defaultProjectId}
                defaultRuntime={normalizeRuntimeSelection(defaultRuntime)}
                isLoadingProjects={isLoadingProjects}
                isSubmitting={isStartingConversation}
                onCreateProject={onCreateProject}
                onSubmit={async (input) => {
                  try {
                    setIsStartingConversation(true);
                    await handleStartAgentConversation(input);
                  } catch (err) {
                    toast.error(
                      err instanceof Error
                        ? err.message
                        : "Failed to start agent conversation",
                    );
                    throw err;
                  } finally {
                    setIsStartingConversation(false);
                  }
                }}
              />
            </div>
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
