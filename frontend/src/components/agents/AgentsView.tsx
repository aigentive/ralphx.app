import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ElementType,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  ClipboardList,
  FileText,
  GitPullRequestArrow,
  PanelRightOpen,
  PanelRightClose,
} from "lucide-react";
import { toast } from "sonner";

import { chatApi } from "@/api/chat";
import { executionApi } from "@/api/execution";
import { ideationApi } from "@/api/ideation";
import { projectsApi } from "@/api/projects";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ResizeHandle } from "@/components/ui/ResizeHandle";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { chatKeys, invalidateConversationDataQueries, useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { projectKeys, useProjects } from "@/hooks/useProjects";
import { getModelLabel } from "@/lib/model-utils";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import {
  selectArtifactState,
  selectHasStoredArtifactState,
  useAgentSessionStore,
  type AgentArtifactTab,
  type AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
import { AgentsArtifactPane } from "./AgentsArtifactPane";
import { AgentsSidebar } from "./AgentsSidebar";
import {
  getAgentConversationStoreKey,
  toProjectAgentConversation,
  type AgentConversation,
} from "./agentConversations";
import {
  deriveAgentTitleFromMessages,
  isDefaultAgentTitle,
} from "./agentTitle";
import {
  DEFAULT_AGENT_RUNTIME,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { AgentsStartComposer } from "./AgentsStartComposer";
import {
  agentConversationKeys,
  useProjectAgentConversations,
} from "./useProjectAgentConversations";
import { resolveAttachedIdeationSessionId } from "./attachedIdeationSession";
import { useAgentConversationTitleEvents } from "./useAgentConversationTitleEvents";
import { useProjectAgentBridgeEvents } from "./useProjectAgentBridgeEvents";

const HEADER_ARTIFACT_TABS: Array<{
  id: AgentArtifactTab;
  label: string;
  icon: ElementType;
}> = [
  { id: "plan", label: "Plan", icon: FileText },
  { id: "verification", label: "Verification", icon: CheckCircle2 },
  { id: "proposal", label: "Proposals", icon: GitPullRequestArrow },
  { id: "tasks", label: "Tasks", icon: ClipboardList },
];

const AGENTS_ARTIFACT_WIDTH_STORAGE_KEY = "ralphx-agents-artifact-width";
const AGENTS_ARTIFACT_MIN_WIDTH = 320;
const AGENTS_CHAT_MIN_WIDTH = 320;
const AGENTS_ARTIFACT_DEFAULT_WIDTH = "66.666667%";
const AGENTS_CHAT_CONTENT_WIDTH_CLASS = "max-w-[980px]";

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
  const [artifactPanelWidth, setArtifactPanelWidth] = useState<number | null>(() => {
    const saved = window.localStorage.getItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY);
    if (!saved) {
      return null;
    }
    const parsed = Number.parseInt(saved, 10);
    return Number.isFinite(parsed) && parsed >= AGENTS_ARTIFACT_MIN_WIDTH ? parsed : null;
  });
  const [isArtifactResizing, setIsArtifactResizing] = useState(false);
  const splitContainerRef = useRef<HTMLDivElement>(null);
  const autoTitleStateRef = useRef<
    Map<string, { messages: string[]; lastTitle: string | null }>
  >(new Map());
  const childArchiveSyncRef = useRef<Set<string>>(new Set());
  const syncedProjectIdRef = useRef<string | null>(null);
  const { data: projects = [], isLoading: isLoadingProjects } = useProjects();
  const setActiveConversation = useChatStore((s) => s.setActiveConversation);

  const focusedProjectId = useAgentSessionStore((s) => s.focusedProjectId);
  const selectedProjectId = useAgentSessionStore((s) => s.selectedProjectId);
  const selectedConversationId = useAgentSessionStore((s) => s.selectedConversationId);
  const runtimeByConversationId = useAgentSessionStore((s) => s.runtimeByConversationId);
  const lastRuntimeByProjectId = useAgentSessionStore((s) => s.lastRuntimeByProjectId);
  const setFocusedProject = useAgentSessionStore((s) => s.setFocusedProject);
  const selectConversation = useAgentSessionStore((s) => s.selectConversation);
  const clearSelection = useAgentSessionStore((s) => s.clearSelection);
  const setRuntimeForConversation = useAgentSessionStore((s) => s.setRuntimeForConversation);
  const setArtifactOpen = useAgentSessionStore((s) => s.setArtifactOpen);
  const setArtifactTab = useAgentSessionStore((s) => s.setArtifactTab);
  const setTaskArtifactMode = useAgentSessionStore((s) => s.setTaskArtifactMode);
  const artifactWidthCss = artifactPanelWidth
    ? `${artifactPanelWidth}px`
    : AGENTS_ARTIFACT_DEFAULT_WIDTH;

  const defaultProjectId = focusedProjectId || selectedProjectId || projectId || projects[0]?.id || null;
  const activeProjectId = selectedProjectId || defaultProjectId;
  const focusedConversations = useProjectAgentConversations(activeProjectId, showArchived);
  const artifactState = useAgentSessionStore(selectArtifactState(selectedConversationId));
  const hasStoredArtifactState = useAgentSessionStore(
    selectHasStoredArtifactState(selectedConversationId)
  );
  const selectedConversationQuery = useConversation(selectedConversationId, {
    enabled: !!selectedConversationId,
  });
  const selectedConversationData = selectedConversationQuery.data;
  const selectedConversationFallback = useMemo(() => {
    const conversation = selectedConversationData?.conversation;
    if (
      !conversation ||
      conversation.id !== selectedConversationId ||
      conversation.contextType !== "project" ||
      conversation.contextId !== activeProjectId ||
      (!showArchived && Boolean(conversation.archivedAt))
    ) {
      return null;
    }

    return toProjectAgentConversation(conversation);
  }, [
    activeProjectId,
    selectedConversationData,
    selectedConversationId,
    showArchived,
  ]);

  const activeConversation = useMemo(() => {
    if (!selectedConversationId) {
      return null;
    }
    return (
      focusedConversations.data?.find(
        (conversation) => conversation.id === selectedConversationId
      ) ?? selectedConversationFallback
    );
  }, [
    focusedConversations.data,
    selectedConversationFallback,
    selectedConversationId,
  ]);
  const selectedConversationMessages = useMemo(
    () =>
      selectedConversationData && selectedConversationData.conversation?.id === selectedConversationId
        ? selectedConversationData.messages
        : [],
    [selectedConversationData, selectedConversationId],
  );
  const attachedIdeationSessionId = useMemo(
    () => resolveAttachedIdeationSessionId(activeConversation, selectedConversationMessages),
    [activeConversation, selectedConversationMessages],
  );
  const attachedIdeationSessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedIdeationSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedIdeationSessionId!),
    enabled: !!attachedIdeationSessionId,
    staleTime: 5_000,
  });
  const attachedIdeationSessionData =
    attachedIdeationSessionId &&
    attachedIdeationSessionQuery.data?.session.id === attachedIdeationSessionId
      ? attachedIdeationSessionQuery.data
      : null;
  const hasAutoOpenArtifacts = useMemo(() => {
    if (!attachedIdeationSessionData) {
      return false;
    }

    const session = attachedIdeationSessionData.session;
    return Boolean(
      session.planArtifactId ||
        session.inheritedPlanArtifactId ||
        session.acceptanceStatus === "pending" ||
        session.verificationInProgress ||
        session.verificationStatus !== "unverified" ||
        attachedIdeationSessionData.proposals.length > 0
    );
  }, [attachedIdeationSessionData]);
  const artifactPaneOpen = hasStoredArtifactState
    ? artifactState.isOpen
    : hasAutoOpenArtifacts;
  useAgentConversationTitleEvents(activeProjectId);
  useProjectAgentBridgeEvents({
    conversation: activeConversation,
    attachedIdeationSessionId,
    projectId: activeProjectId,
  });

  const handleArtifactResizeStart = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    setIsArtifactResizing(true);
  }, []);

  const handleArtifactResizeReset = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    setArtifactPanelWidth(null);
  }, []);

  useEffect(() => {
    if (!isArtifactResizing) {
      return;
    }

    const handleMouseMove = (event: MouseEvent) => {
      const container = splitContainerRef.current;
      if (!container) {
        return;
      }
      const rect = container.getBoundingClientRect();
      const maxArtifactWidth = Math.max(
        AGENTS_ARTIFACT_MIN_WIDTH,
        rect.width - AGENTS_CHAT_MIN_WIDTH,
      );
      const nextWidth = rect.right - event.clientX;
      setArtifactPanelWidth(
        Math.max(AGENTS_ARTIFACT_MIN_WIDTH, Math.min(maxArtifactWidth, nextWidth)),
      );
    };

    const handleMouseUp = () => setIsArtifactResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isArtifactResizing]);

  useEffect(() => {
    if (artifactPanelWidth !== null) {
      window.localStorage.setItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY, String(artifactPanelWidth));
      return;
    }
    window.localStorage.removeItem(AGENTS_ARTIFACT_WIDTH_STORAGE_KEY);
  }, [artifactPanelWidth]);

  const activeRuntime = selectedConversationId
    ? runtimeByConversationId[selectedConversationId] ??
      runtimeFromConversation(activeConversation) ??
      null
    : null;
  const normalizedActiveRuntime = normalizeRuntimeSelection(activeRuntime);

  useEffect(() => {
    if (!projectId || syncedProjectIdRef.current === projectId) {
      return;
    }
    syncedProjectIdRef.current = projectId;
    setFocusedProject(projectId);
    clearSelection();
  }, [clearSelection, projectId, setFocusedProject]);

  useEffect(() => {
    if (
      !selectedConversationId ||
      !activeProjectId ||
      focusedConversations.isLoading ||
      selectedConversationQuery.isLoading
    ) {
      return;
    }
    const selectedStillExists = focusedConversations.data?.some(
      (conversation) => conversation.id === selectedConversationId
    );
    if (selectedStillExists === false && !selectedConversationFallback) {
      clearSelection();
    }
  }, [
    activeProjectId,
    clearSelection,
    focusedConversations.data,
    focusedConversations.isLoading,
    selectedConversationFallback,
    selectedConversationQuery.isLoading,
    selectedConversationId,
  ]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || !selectedConversationId) {
        return;
      }
      const activeElement = document.activeElement;
      if (
        activeElement instanceof HTMLInputElement ||
        activeElement instanceof HTMLTextAreaElement
      ) {
        return;
      }

      if (event.key === "\\") {
        event.preventDefault();
        setArtifactOpen(selectedConversationId, !artifactPaneOpen);
        return;
      }

      const tabByKey: Record<string, AgentArtifactTab> = {
        "1": "plan",
        "2": "verification",
        "3": "proposal",
        "4": "tasks",
      };
      const tab = tabByKey[event.key];
      if (tab) {
        event.preventDefault();
        setArtifactTab(selectedConversationId, tab);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    artifactPaneOpen,
    selectedConversationId,
    setArtifactOpen,
    setArtifactTab,
  ]);

  const handleSelectConversation = useCallback(
    (conversationProjectId: string, conversation: AgentConversation) => {
      if (
        selectedProjectId === conversationProjectId &&
        selectedConversationId === conversation.id
      ) {
        clearSelection();
        return;
      }

      selectConversation(conversationProjectId, conversation.id);
      setActiveConversation(
        getAgentConversationStoreKey(conversation),
        conversation.id
      );
    },
    [
      clearSelection,
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
      clearSelection();
    },
    [clearSelection, focusedProjectId, projectId, projects, selectedProjectId, setFocusedProject]
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
        queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() }),
      ]);
    },
    [queryClient]
  );

  const handleAutoManagedTitle = useCallback(
    ({
      content,
      conversationId,
      targetProjectId,
      shouldSpawnSessionNamer,
    }: {
      content: string;
      conversationId: string;
      targetProjectId: string;
      shouldSpawnSessionNamer: boolean;
    }) => {
      const conversation = findConversationById(conversationId);
      const titleIsAutoManaged =
        isDefaultAgentTitle(conversation?.title) ||
        autoTitleStateRef.current.get(conversationId)?.lastTitle === conversation?.title;
      if (!titleIsAutoManaged) {
        return;
      }

      const state = autoTitleStateRef.current.get(conversationId) ?? {
        messages: [],
        lastTitle: null,
      };
      const isFirstTrackedMessage = state.messages.length === 0;
      if (shouldSpawnSessionNamer && isFirstTrackedMessage) {
        void chatApi
          .spawnConversationSessionNamer(conversationId, content)
          .catch(() => {
            // Session namer is best-effort; local auto-titling remains as fallback.
          });
      }

      if (state.messages.length >= 3) {
        return;
      }

      state.messages = [...state.messages, content].slice(0, 3);
      const nextTitle = deriveAgentTitleFromMessages(state.messages);
      if (!nextTitle || nextTitle === conversation?.title || nextTitle === state.lastTitle) {
        autoTitleStateRef.current.set(conversationId, state);
        return;
      }

      state.lastTitle = nextTitle;
      autoTitleStateRef.current.set(conversationId, state);
      const titleUpdate =
        conversation?.contextType === "ideation"
          ? Promise.all([
              chatApi.updateConversationTitle(conversationId, nextTitle),
              ideationApi.sessions.updateTitle(conversation.contextId, nextTitle),
            ])
          : chatApi.updateConversationTitle(conversationId, nextTitle);
      void titleUpdate
        .then(() => {
          void invalidateProjectConversations(conversation?.projectId ?? targetProjectId);
        })
        .catch(() => {
          // Auto-titling is best-effort; manual title editing remains available.
        });
    },
    [findConversationById, invalidateProjectConversations]
  );

  const handleStartAgentConversation = useCallback(
    async ({
      projectId: targetProjectId,
      content,
      runtime,
    }: {
      projectId: string;
      content: string;
      runtime: AgentRuntimeSelection;
    }) => {
      const normalizedRuntime = normalizeRuntimeSelection(runtime);
      const result = await chatApi.sendAgentMessage(
        "project",
        targetProjectId,
        content,
        undefined,
        undefined,
        {
          providerHarness: normalizedRuntime.provider,
          modelId: normalizedRuntime.modelId,
        }
      );

      setFocusedProject(targetProjectId);
      setRuntimeForConversation(result.conversationId, targetProjectId, normalizedRuntime);
      selectConversation(targetProjectId, result.conversationId);
      setActiveConversation(
        getAgentConversationStoreKey({
          id: result.conversationId,
          contextType: "project",
          contextId: targetProjectId,
        }),
        result.conversationId
      );
      invalidateConversationDataQueries(queryClient, result.conversationId);
      await invalidateProjectConversations(targetProjectId);
      handleAutoManagedTitle({
        content,
        conversationId: result.conversationId,
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

  const handleQuickCreateAgent = useCallback((quickProjectId?: string) => {
    if (isStartingConversation) {
      return;
    }
    showStarterComposer(quickProjectId ?? null);
  }, [isStartingConversation, showStarterComposer]);

  const handleSelectArtifact = useCallback(
    (tab: AgentArtifactTab) => {
      if (!selectedConversationId) {
        return;
      }
      if (artifactPaneOpen && artifactState.activeTab === tab) {
        setArtifactOpen(selectedConversationId, false);
        return;
      }
      setArtifactTab(selectedConversationId, tab);
    },
    [
      artifactState.activeTab,
      artifactPaneOpen,
      selectedConversationId,
      setArtifactOpen,
      setArtifactTab,
    ]
  );

  useEffect(() => {
    if (
      activeConversation?.contextType !== "project" ||
      !attachedIdeationSessionData ||
      activeConversation.archivedAt ||
      childArchiveSyncRef.current.has(activeConversation.id)
    ) {
      return;
    }
    const session = attachedIdeationSessionData.session;
    const sessionArchived = session.status === "archived" || Boolean(session.archivedAt);
    if (!sessionArchived) {
      return;
    }
    childArchiveSyncRef.current.add(activeConversation.id);
    void chatApi.archiveConversation(activeConversation.id)
      .then(() => invalidateProjectConversations(activeConversation.projectId))
      .catch(() => {
        childArchiveSyncRef.current.delete(activeConversation.id);
        // Status sync is best-effort; manual archive remains available.
      });
  }, [
    activeConversation,
    attachedIdeationSessionData,
    invalidateProjectConversations,
  ]);

  const handleRemoveProject = useCallback(
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
          clearSelection();
        }
        await queryClient.invalidateQueries({ queryKey: projectKeys.list() });
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to remove project");
      }
    },
    [
      clearSelection,
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
          clearSelection();
        }
        await invalidateProjectConversations(conversation.projectId);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to archive session");
      }
    },
    [clearSelection, invalidateProjectConversations, selectedConversationId]
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
      autoTitleStateRef.current.delete(conversationId);
      await invalidateProjectConversations(conversation?.projectId ?? activeProjectId ?? projectId);
    },
    [activeProjectId, findConversationById, invalidateProjectConversations, projectId]
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

  const defaultRuntime =
    (defaultProjectId ? lastRuntimeByProjectId[defaultProjectId] : null) ??
    (selectedConversationId ? runtimeByConversationId[selectedConversationId] : null) ??
    DEFAULT_AGENT_RUNTIME;

  return (
    <TooltipProvider delayDuration={300}>
      <section
        className="h-full min-h-0 w-full flex overflow-hidden"
        style={{ background: "var(--bg-base)" }}
        data-testid="agents-view"
      >
        <AgentsSidebar
          projects={projects}
          focusedProjectId={focusedProjectId ?? defaultProjectId}
          selectedConversationId={selectedConversationId}
          onFocusProject={setFocusedProject}
          onSelectConversation={handleSelectConversation}
          onCreateAgent={() => showStarterComposer()}
          onCreateProject={onCreateProject}
          onQuickCreateAgent={handleQuickCreateAgent}
          onRemoveProject={handleRemoveProject}
          onArchiveConversation={handleArchiveConversation}
          onRestoreConversation={handleRestoreConversation}
          isCreatingAgent={isStartingConversation}
          showArchived={showArchived}
          onShowArchivedChange={setShowArchived}
        />

        <div
          ref={splitContainerRef}
          className="relative flex-1 min-w-0 h-full flex overflow-hidden"
          data-testid="agents-split-container"
        >
          {activeProjectId && selectedConversationId && activeConversation ? (
            <div className="flex-1 min-w-0 h-full">
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
                {...(activeConversation.contextType === "project" && attachedIdeationSessionId
                  ? { additionalQuestionSessionIds: [attachedIdeationSessionId] }
                  : {})}
                headerContent={
                  <AgentsChatHeader
                    conversation={activeConversation}
                    runtime={normalizedActiveRuntime}
                    artifactOpen={artifactPaneOpen}
                    activeArtifactTab={artifactState.activeTab}
                    onRenameConversation={handleRenameConversation}
                    onToggleArtifacts={() => setArtifactOpen(selectedConversationId, !artifactPaneOpen)}
                    onSelectArtifact={handleSelectArtifact}
                  />
                }
                emptyState={<div />}
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

          {selectedConversationId && artifactPaneOpen && activeConversation && (
            <>
              <div className="max-lg:hidden">
                <ResizeHandle
                  isResizing={isArtifactResizing}
                  onMouseDown={handleArtifactResizeStart}
                  onDoubleClick={handleArtifactResizeReset}
                  testId="agents-artifact-resize-handle"
                />
              </div>
              <div
                className="h-full shrink-0 max-lg:absolute max-lg:inset-y-0 max-lg:right-0 max-lg:z-20 max-lg:!w-[min(100%,420px)] max-lg:!min-w-0 max-lg:!max-w-none"
                style={{
                  width: artifactWidthCss,
                  minWidth: AGENTS_ARTIFACT_MIN_WIDTH,
                  maxWidth: `calc(100% - ${AGENTS_CHAT_MIN_WIDTH}px)`,
                  transition: isArtifactResizing ? "none" : "width 150ms ease-out",
                }}
                data-testid="agents-artifact-resizable-pane"
              >
                <AgentsArtifactPane
                  conversation={activeConversation}
                  activeTab={artifactState.activeTab}
                  taskMode={artifactState.taskMode}
                  onTabChange={handleSelectArtifact}
                  onTaskModeChange={(mode) => setTaskArtifactMode(selectedConversationId, mode)}
                  onClose={() => setArtifactOpen(selectedConversationId, false)}
                />
              </div>
            </>
          )}
        </div>

      </section>
    </TooltipProvider>
  );
}

interface AgentsChatHeaderProps {
  conversation: AgentConversation | null;
  runtime: AgentRuntimeSelection;
  artifactOpen: boolean;
  activeArtifactTab: AgentArtifactTab;
  onRenameConversation: (conversationId: string, title: string) => Promise<void>;
  onToggleArtifacts: () => void;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
}

export function AgentsChatHeader({
  conversation,
  runtime,
  artifactOpen,
  activeArtifactTab,
  onRenameConversation,
  onToggleArtifacts,
  onSelectArtifact,
}: AgentsChatHeaderProps) {
  const title = conversation?.title || "Untitled agent";
  const modelLabel = getModelLabel(runtime.modelId);
  const providerLabel = runtime.provider === "codex" ? "Codex" : "Claude";
  const modeLabel = runtime.provider === "codex" ? "Medium" : "Default";
  const [isEditing, setIsEditing] = useState(false);
  const [draftTitle, setDraftTitle] = useState(title);

  useEffect(() => {
    if (!isEditing) {
      setDraftTitle(title);
    }
  }, [isEditing, title]);

  const commitTitle = useCallback(async () => {
    if (!conversation) {
      setIsEditing(false);
      return;
    }
    const trimmed = draftTitle.trim();
    if (!trimmed || trimmed === title) {
      setDraftTitle(title);
      setIsEditing(false);
      return;
    }
    await onRenameConversation(conversation.id, trimmed);
    setIsEditing(false);
  }, [conversation, draftTitle, onRenameConversation, title]);

  return (
    <div className="flex w-full flex-1 items-center justify-between gap-3 min-w-0">
      <div className="min-w-0 shrink">
        {isEditing ? (
          <Input
            value={draftTitle}
            onChange={(event) => setDraftTitle(event.target.value)}
            onBlur={() => void commitTitle()}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void commitTitle();
              }
              if (event.key === "Escape") {
                event.preventDefault();
                setDraftTitle(title);
                setIsEditing(false);
              }
            }}
            className="h-7 max-w-[260px] text-sm font-semibold"
            autoFocus
            aria-label="Agent title"
          />
        ) : (
          <button
            type="button"
            className="block max-w-[420px] text-left text-sm font-semibold truncate"
            style={{ color: "var(--text-primary)" }}
            onClick={() => conversation && setIsEditing(true)}
            aria-label="Edit agent title"
            data-testid="agents-chat-title-button"
            data-theme-button-skip="true"
          >
            {title}
          </button>
        )}
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px] leading-none">
          <RuntimeMetaItem label="Provider" value={providerLabel} />
          <RuntimeMetaItem label="Model" value={modelLabel} />
          <RuntimeMetaItem label="Mode" value={modeLabel} />
        </div>
      </div>

      <div className="hidden md:flex items-center gap-1 ml-auto shrink-0">
        {!artifactOpen &&
          HEADER_ARTIFACT_TABS.map(({ id, label, icon: Icon }) => {
            const isActive = activeArtifactTab === id && artifactOpen;
            return (
              <Tooltip key={id}>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className={cn("h-8 w-8 p-0", isActive ? "" : "opacity-80")}
                    onClick={() => onSelectArtifact(id)}
                    style={{
                      color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
                      background: isActive ? withAlpha("var(--accent-primary)", 12) : "transparent",
                      border: isActive
                        ? "1px solid var(--accent-border)"
                        : "1px solid var(--overlay-faint)",
                      boxShadow: "none",
                    }}
                    aria-label={label}
                  >
                    <Icon className="w-4 h-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  {label}
                </TooltipContent>
              </Tooltip>
            );
          })}

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              onClick={onToggleArtifacts}
              aria-label={artifactOpen ? "Close artifacts" : "Open artifacts"}
            >
              {artifactOpen ? (
                <PanelRightClose className="w-4 h-4" />
              ) : (
                <PanelRightOpen className="w-4 h-4" />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="text-xs">
            {artifactOpen ? "Close artifacts" : "Open artifacts"}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}

function RuntimeMetaItem({ label, value }: { label: string; value: string }) {
  return (
    <span className="inline-flex min-w-0 items-baseline gap-1">
      <span className="text-[var(--text-muted)]">{label}</span>
      <span className="truncate font-medium text-[var(--text-secondary)]">{value}</span>
    </span>
  );
}

function runtimeFromConversation(
  conversation: AgentConversation | null
): AgentRuntimeSelection | null {
  if (!conversation?.providerHarness) {
    return null;
  }

  if (conversation.providerHarness === "claude") {
    return {
      provider: "claude",
      modelId: "sonnet",
    };
  }

  if (conversation.providerHarness === "codex") {
    return {
      provider: "codex",
      modelId: DEFAULT_AGENT_RUNTIME.modelId,
    };
  }

  return null;
}
