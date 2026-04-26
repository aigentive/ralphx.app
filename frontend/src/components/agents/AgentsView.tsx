import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Menu } from "lucide-react";
import { toast } from "sonner";

import { chatApi } from "@/api/chat";
import type {
  AgentConversationBaseSelection,
  AgentConversationWorkspace,
  AgentConversationWorkspaceMode,
} from "@/api/chat";
import { executionApi } from "@/api/execution";
import { ideationApi } from "@/api/ideation";
import { projectsApi } from "@/api/projects";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { TooltipProvider } from "@/components/ui/tooltip";
import { chatKeys, invalidateConversationDataQueries } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { projectKeys, useProjects } from "@/hooks/useProjects";
import { useResponsiveSidebarLayout } from "@/hooks/useResponsiveSidebarLayout";
import { withAlpha } from "@/lib/theme-colors";
import { useChatStore } from "@/stores/chatStore";
import {
  useAgentSessionStore,
  type AgentArtifactTab,
  type AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import { AgentsSidebar } from "./AgentsSidebar";
import {
  getAgentConversationStoreKey,
  toProjectAgentConversation,
  type AgentConversation,
} from "./agentConversations";
import {
  DEFAULT_AGENT_RUNTIME,
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
  resolveConversationAgentMode,
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
import { uploadDraftAttachment } from "./chatAttachmentUpload";
import { useAgentArtifactResize } from "./useAgentArtifactResize";
import { useAgentsSelectionModel } from "./useAgentsSelectionModel";
import { useAgentsWorkspaceModel } from "./useAgentsWorkspaceModel";
import { useAgentsAttachedIdeation } from "./useAgentsAttachedIdeation";
import { useAgentsAutoTitle } from "./useAgentsAutoTitle";

const AGENTS_CHAT_CONTENT_WIDTH_CLASS = "max-w-[980px]";
const AGENTS_SIDEBAR_COLLAPSE_STORAGE_KEY = "ralphx-agents-sidebar-collapsed";
function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return fallback;
}

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
  const [publishingConversationId, setPublishingConversationId] = useState<string | null>(null);
  const [switchingConversationModeId, setSwitchingConversationModeId] = useState<string | null>(null);
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

  const handleSelectConversation = useCallback(
    (conversationProjectId: string, conversation: AgentConversation) => {
      if (
        selectedProjectId === conversationProjectId &&
        selectedConversationId === conversation.id
      ) {
        clearAgentConversationSelection();
        return;
      }

      setOptimisticSelectedConversationId(conversation.id);
      selectConversation(conversationProjectId, conversation.id);
      setActiveConversation(
        getAgentConversationStoreKey(conversation),
        conversation.id
      );
    },
    [
      clearAgentConversationSelection,
      selectConversation,
      selectedConversationId,
      selectedProjectId,
      setActiveConversation,
    ]
  );

  const findConversationById = useCallback(
    (conversationId: string) =>
      focusedConversations.data?.find((item) => item.id === conversationId) ??
      (selectedConversationFallback?.id === conversationId
        ? selectedConversationFallback
        : null),
    [focusedConversations.data, selectedConversationFallback]
  );

  const showStarterComposer = useCallback(
    (targetProjectId?: string | null) => {
      const nextProjectId =
        targetProjectId ??
        focusedProjectId ??
        selectedProjectId ??
        projectId ??
        projects[0]?.id ??
        null;
      if (nextProjectId) {
        setFocusedProject(nextProjectId);
      }
      clearAgentConversationSelection();
    },
    [
      clearAgentConversationSelection,
      focusedProjectId,
      projectId,
      projects,
      selectedProjectId,
      setFocusedProject,
    ]
  );

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

  const handleStartAgentConversation = useCallback(
    async ({
      projectId: targetProjectId,
      content,
      runtime,
      mode,
      base,
      files,
    }: {
      projectId: string;
      content: string;
      runtime: AgentRuntimeSelection;
      mode: AgentConversationWorkspaceMode;
      base: AgentConversationBaseSelection | null;
      files: File[];
    }) => {
      const normalizedRuntime = normalizeRuntimeSelection(runtime);
      let conversationIdOverride: string | null = null;

      if (files.length > 0) {
        const createdConversation = await chatApi.createConversation("project", targetProjectId);
        conversationIdOverride = createdConversation.id;
        await Promise.all(
          files.map((file) => uploadDraftAttachment(createdConversation.id, file))
        );
      }

      const result = await chatApi.startAgentConversation({
        projectId: targetProjectId,
        content,
        ...(conversationIdOverride ? { conversationId: conversationIdOverride } : {}),
        providerHarness: normalizedRuntime.provider,
        modelId: normalizedRuntime.modelId,
        mode,
        ...(base ? { base } : {}),
      });
      const resolvedConversationId = result.conversation.id;
      const optimisticConversation = toProjectAgentConversation(result.conversation);

      setOptimisticConversationsById((current) => ({
        ...current,
        [resolvedConversationId]: optimisticConversation,
      }));
      const optimisticWorkspace = result.workspace;
      if (optimisticWorkspace) {
        setOptimisticWorkspacesByConversationId((current) => ({
          ...current,
          [resolvedConversationId]: optimisticWorkspace,
        }));
      }
      queryClient.setQueryData(chatKeys.conversation(resolvedConversationId), {
        conversation: result.conversation,
        messages: [],
      });
      queryClient.setQueryData(
        ["agents", "conversation-workspace", resolvedConversationId],
        optimisticWorkspace ?? null,
      );
      setOptimisticSelectedConversationId(resolvedConversationId);
      setFocusedProject(targetProjectId);
      setRuntimeForConversation(resolvedConversationId, targetProjectId, normalizedRuntime);
      selectConversation(targetProjectId, resolvedConversationId);
      setActiveConversation(
        getAgentConversationStoreKey({
          id: resolvedConversationId,
          contextType: "project",
          contextId: targetProjectId,
        }),
        resolvedConversationId
      );
      invalidateConversationDataQueries(queryClient, resolvedConversationId);
      await invalidateProjectConversations(targetProjectId);
      handleAutoManagedTitle({
        content,
        conversationId: resolvedConversationId,
        targetProjectId,
        shouldSpawnSessionNamer: true,
      });
    },
    [
      handleAutoManagedTitle,
      invalidateProjectConversations,
      queryClient,
      selectConversation,
      setActiveConversation,
      setFocusedProject,
      setRuntimeForConversation,
    ]
  );

  const handleSidebarFocusProject = useCallback(
    (targetProjectId: string) => {
      setFocusedProject(targetProjectId);
      if (isSidebarOverlayOpen) {
        closeSidebarOverlay();
      }
    },
    [closeSidebarOverlay, isSidebarOverlayOpen, setFocusedProject]
  );

  const handleSidebarSelectConversation = useCallback(
    (conversationProjectId: string, conversation: AgentConversation) => {
      handleSelectConversation(conversationProjectId, conversation);
      if (isSidebarOverlayOpen) {
        closeSidebarOverlay();
      }
    },
    [closeSidebarOverlay, handleSelectConversation, isSidebarOverlayOpen]
  );

  const handleSidebarCreateAgent = useCallback(() => {
    showStarterComposer();
    if (isSidebarOverlayOpen) {
      closeSidebarOverlay();
    }
  }, [closeSidebarOverlay, isSidebarOverlayOpen, showStarterComposer]);

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

  const handleArchiveProject = useCallback(
    async (targetProjectId: string) => {
      try {
        try {
          await projectsApi.archive(targetProjectId);
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          if (!message.includes("currently active project")) {
            throw err;
          }
          await executionApi.setActiveProject(undefined);
          await projectsApi.archive(targetProjectId);
        }
        if (focusedProjectId === targetProjectId) {
          setFocusedProject(null);
        }
        if (selectedProjectId === targetProjectId) {
          clearAgentConversationSelection();
        }
        await queryClient.invalidateQueries({ queryKey: projectKeys.list() });
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to archive project");
      }
    },
    [
      clearAgentConversationSelection,
      focusedProjectId,
      queryClient,
      selectedProjectId,
      setFocusedProject,
    ]
  );

  const handleArchiveConversation = useCallback(
    async (conversation: AgentConversation) => {
      try {
        if (conversation.contextType === "ideation") {
          await ideationApi.sessions.archive(conversation.contextId);
        }
        await chatApi.archiveConversation(conversation.id);
        if (selectedConversationId === conversation.id) {
          clearAgentConversationSelection();
        }
        await invalidateProjectConversations(conversation.projectId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to archive session");
      }
    },
    [clearAgentConversationSelection, invalidateProjectConversations, selectedConversationId]
  );

  const handleRestoreConversation = useCallback(
    async (conversation: AgentConversation) => {
      try {
        if (conversation.contextType === "ideation") {
          await ideationApi.sessions.reopen(conversation.contextId);
        }
        await chatApi.restoreConversation(conversation.id);
        await invalidateProjectConversations(conversation.projectId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to restore session");
      }
    },
    [invalidateProjectConversations]
  );

  const handleRenameConversation = useCallback(
    async (conversationId: string, title: string) => {
      const trimmed = title.trim();
      if (!trimmed) {
        return;
      }
      const conversation = findConversationById(conversationId);
      if (conversation?.contextType === "ideation") {
        await Promise.all([
          chatApi.updateConversationTitle(conversationId, trimmed),
          ideationApi.sessions.updateTitle(conversation.contextId, trimmed),
        ]);
      } else {
        await chatApi.updateConversationTitle(conversationId, trimmed);
      }
      clearAutoManagedTitle(conversationId);
      await invalidateProjectConversations(conversation?.projectId ?? activeProjectId ?? projectId);
    },
    [
      activeProjectId,
      clearAutoManagedTitle,
      findConversationById,
      invalidateProjectConversations,
      projectId,
    ]
  );

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

  const handlePublishWorkspace = useCallback(
    async (conversationId: string) => {
      const conversation = findConversationById(conversationId);
      const workspace =
        selectedConversationId === conversationId
          ? activeWorkspace
          : optimisticWorkspacesByConversationId[conversationId] ?? null;
      setPublishingConversationId(conversationId);
      try {
        const result = await chatApi.publishAgentConversationWorkspace(conversationId);
        const prLabel = result.prNumber ? `#${result.prNumber}` : result.prUrl;
        toast.success(prLabel ? `Published ${prLabel}` : "Published branch");
        await Promise.all([
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace", conversationId],
          }),
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace-publication-events", conversationId],
          }),
          conversation?.projectId
            ? invalidateProjectConversations(conversation.projectId)
            : Promise.resolve(),
        ]);
      } catch (err) {
        const errorMessage = getErrorMessage(err, "Failed to publish branch");
        let refreshedWorkspace: AgentConversationWorkspace | null = null;
        try {
          refreshedWorkspace = await chatApi.getAgentConversationWorkspace(conversationId);
          void queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace-publication-events", conversationId],
          });
          if (refreshedWorkspace) {
            queryClient.setQueryData(
              ["agents", "conversation-workspace", conversationId],
              refreshedWorkspace
            );
          }
        } catch {
          refreshedWorkspace = null;
        }
        const publishFailureNeedsAgent =
          (refreshedWorkspace ?? workspace)?.publicationPushStatus === "needs_agent";

        if (publishFailureNeedsAgent) {
          toast.error("Publish failed. Sent the error to the agent to fix.");
          if (conversation?.projectId) {
            await invalidateProjectConversations(conversation.projectId);
          }
          invalidateConversationDataQueries(queryClient, conversationId);
        } else {
          toast.error(errorMessage);
        }
      } finally {
        setPublishingConversationId(null);
      }
    },
    [
      activeWorkspace,
      findConversationById,
      invalidateProjectConversations,
      optimisticWorkspacesByConversationId,
      queryClient,
      selectedConversationId,
    ]
  );

  const defaultRuntime =
    (defaultProjectId ? lastRuntimeByProjectId[defaultProjectId] : null) ??
    (selectedConversationId ? runtimeByConversationId[selectedConversationId] : null) ??
    DEFAULT_AGENT_RUNTIME;

  const activeProjectOptions = useMemo(
    () =>
      activeProjectId
        ? projects
            .filter((project) => project.id === activeProjectId)
            .map((project) => ({
              id: project.id,
              label: project.name,
              description: project.workingDirectory,
            }))
        : [],
    [activeProjectId, projects]
  );

  const handleActiveModelChange = useCallback(
    (modelId: string) => {
      if (!selectedConversationId || !activeProjectId) {
        return;
      }
      setRuntimeForConversation(selectedConversationId, activeProjectId, {
        provider: normalizedActiveRuntime.provider,
        modelId,
      });
    },
    [
      activeProjectId,
      normalizedActiveRuntime.provider,
      selectedConversationId,
      setRuntimeForConversation,
    ]
  );

  const handleActiveConversationModeChange = useCallback(
    async (mode: AgentConversationWorkspaceMode) => {
      if (
        !selectedConversationId ||
        !activeProjectId ||
        !activeConversation ||
        activeConversation.contextType !== "project" ||
        activeConversationModeLocked
      ) {
        return;
      }

      const currentMode = resolveConversationAgentMode(activeConversation, activeWorkspace);
      if (currentMode === mode) {
        return;
      }

      setSwitchingConversationModeId(selectedConversationId);
      try {
        await chatApi.switchAgentConversationMode({
          conversationId: selectedConversationId,
          mode,
        });
        await Promise.all([
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace", selectedConversationId],
          }),
          invalidateProjectConversations(activeProjectId),
          invalidateConversationDataQueries(queryClient, selectedConversationId),
        ]);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to change agent mode");
      } finally {
        setSwitchingConversationModeId(null);
      }
    },
    [
      activeConversation,
      activeConversationModeLocked,
      activeProjectId,
      activeWorkspace,
      invalidateProjectConversations,
      queryClient,
      selectedConversationId,
    ]
  );

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
    <TooltipProvider delayDuration={300}>
      <section
        className="h-full min-h-0 w-full flex overflow-hidden"
        style={{ background: "var(--bg-base)" }}
        data-testid="agents-view"
      >
        {isSidebarCollapsed && !isSidebarOverlayOpen && (
          <div
            role="button"
            aria-label="Open sidebar"
            tabIndex={0}
            data-testid="agents-sidebar-toggle-strip"
            onClick={toggleSidebarCollapse}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                toggleSidebarCollapse();
              }
            }}
            className="flex items-center justify-center shrink-0 cursor-pointer transition-colors duration-150"
            style={{
              width: 36,
              background: withAlpha("var(--bg-surface)", 50),
              borderRight: "1px solid var(--overlay-faint)",
              color: "var(--text-muted)",
            }}
            onMouseEnter={(event) => {
              event.currentTarget.style.background = "var(--overlay-weak)";
              event.currentTarget.style.color = "var(--text-primary)";
            }}
            onMouseLeave={(event) => {
              event.currentTarget.style.background = withAlpha("var(--bg-surface)", 50);
              event.currentTarget.style.color = "var(--text-muted)";
            }}
          >
            <Menu className="w-4 h-4" />
          </div>
        )}

        {isSidebarOverlayOpen && (
          <div
            aria-hidden="true"
            onClick={closeSidebarOverlay}
            data-testid="agents-sidebar-overlay-backdrop"
            style={{
              position: "fixed",
              inset: 0,
              top: 56,
              background: "var(--overlay-scrim)",
              zIndex: 34,
            }}
          />
        )}

        {!isSidebarOverlayOpen && (
          <div
            style={{
              width: isSidebarCollapsed ? 0 : sidebarWidth,
              minWidth: isSidebarCollapsed ? 0 : sidebarWidth,
              flexShrink: 0,
              overflow: "hidden",
              transition: suppressSidebarTransition.current ? "none" : "width 300ms ease",
              display: isSidebarCollapsed ? "none" : undefined,
            }}
            aria-hidden={isSidebarCollapsed ? "true" : undefined}
          >
            <AgentsSidebar {...sidebarProps} onCollapse={toggleSidebarCollapse} />
          </div>
        )}

        {isSidebarOverlayOpen && (
          <div
            className="plan-browser-slide-in"
            style={{
              position: "fixed",
              top: 56,
              left: 0,
              height: "calc(100vh - 56px)",
              width: sidebarWidth || 340,
              zIndex: 35,
            }}
          >
            <AgentsSidebar {...sidebarProps} onCollapse={closeSidebarOverlay} />
          </div>
        )}

        <div
          ref={splitContainerRef}
          className="relative flex-1 min-w-0 h-full flex overflow-hidden"
          data-testid="agents-split-container"
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
        </div>

      </section>
    </TooltipProvider>
  );
}
