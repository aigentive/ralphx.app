import {
  lazy,
  memo,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
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
import { ResizeHandle } from "@/components/ui/ResizeHandle";
import { BranchBasePicker } from "@/components/shared/BranchBasePicker";
import type { BranchBaseOption } from "@/components/shared/branchBaseOptions";
import { TooltipProvider } from "@/components/ui/tooltip";
import { chatKeys, invalidateConversationDataQueries, useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { projectKeys, useProjects } from "@/hooks/useProjects";
import { useResponsiveSidebarLayout } from "@/hooks/useResponsiveSidebarLayout";
import { withAlpha } from "@/lib/theme-colors";
import { cn } from "@/lib/utils";
import { useChatStore } from "@/stores/chatStore";
import {
  useAgentSessionStore,
  type AgentArtifactState,
  type AgentArtifactTab,
  type AgentTaskArtifactMode,
  type AgentRuntimeSelection,
} from "@/stores/agentSessionStore";
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
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { useAgentArtifactUiStore } from "./agentArtifactUiStore";
import {
  getAgentArtifactStateSnapshot,
  useResolvedAgentArtifactState,
} from "./agentArtifactState";
import { AgentComposerProjectLine, AgentComposerSurface } from "./AgentComposerSurface";
import {
  AgentsChatHeader,
  type AgentsChatHeaderProps,
} from "./AgentsChatHeader";
import { preloadAgentsArtifactPane } from "./agentArtifactPanePreload";
import {
  isWorkspaceModeLocked,
  resolveConversationAgentMode,
} from "./agentConversationMode";
import {
  cancelDeferredFrameJob,
  scheduleDeferredFrameJob,
  useAfterPaintMounted,
  type DeferredFrameJob,
} from "./agentDeferredFrame";
import { preloadAgentTerminalExperience } from "./agentTerminalPreload";
import { AgentsStartComposer } from "./AgentsStartComposer";
import { useAgentTerminalStore } from "./agentTerminalStore";
import {
  AgentsTerminalDockHost,
  AgentsTerminalRegion,
} from "./AgentsTerminalRegion";
import {
  agentConversationKeys,
  useProjectAgentConversations,
} from "./useProjectAgentConversations";
import { archivedConversationCountKey } from "./useArchivedConversationCounts";
import { resolveAttachedIdeationSessionId } from "./attachedIdeationSession";
import { useAgentConversationTitleEvents } from "./useAgentConversationTitleEvents";
import { useProjectAgentBridgeEvents } from "./useProjectAgentBridgeEvents";
import { useDeferredAgentHydration } from "./useDeferredAgentHydration";

const LazyAgentsArtifactPane = lazy(() =>
  preloadAgentsArtifactPane().then((module) => ({ default: module.AgentsArtifactPane })),
);

const AGENTS_ARTIFACT_WIDTH_STORAGE_KEY = "ralphx-agents-artifact-width";
const AGENTS_ARTIFACT_MIN_WIDTH = 320;
const AGENTS_CHAT_MIN_WIDTH = 320;
const AGENTS_ARTIFACT_DEFAULT_WIDTH = "66.666667%";
const AGENTS_CHAT_CONTENT_WIDTH_CLASS = "max-w-[980px]";
const AGENTS_SIDEBAR_COLLAPSE_STORAGE_KEY = "ralphx-agents-sidebar-collapsed";
const AGENT_CONVERSATION_MODE_OPTIONS: Array<{
  id: AgentConversationWorkspaceMode;
  label: string;
  description: string;
}> = [
  { id: "chat", label: "Chat", description: "Ask read-only questions about the project." },
  { id: "edit", label: "Agent", description: "Build, change, and review code in a branch." },
  { id: "ideation", label: "Ideation", description: "Plan work before creating tasks." },
];

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return fallback;
}

function AgentArtifactPaneLoadingShell() {
  return (
    <div
      className="flex h-full min-h-[220px] items-center justify-center p-6 text-center text-sm font-medium text-[var(--text-primary)]"
      data-testid="agents-artifact-pane-loading"
    >
      Loading panel...
    </div>
  );
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
  const artifactResizeFrameRef = useRef<number | null>(null);
  const pendingArtifactWidthRef = useRef<number | null>(null);
  const artifactResizeBoundsRef = useRef<{ right: number; maxWidth: number } | null>(null);
  const artifactPersistenceJobsRef = useRef<
    Map<string, { frame: number | null; timer: number | null; state: AgentArtifactState }>
  >(new Map());
  const artifactPanePreloadJobRef = useRef<DeferredFrameJob | null>(null);
  const autoTitleStateRef = useRef<
    Map<string, { messages: string[]; lastTitle: string | null }>
  >(new Map());
  const childArchiveSyncRef = useRef<Set<string>>(new Set());
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
  const selectedConversationId = storedSelectedConversationId ?? optimisticSelectedConversationId;
  const runtimeByConversationId = useAgentSessionStore((s) => s.runtimeByConversationId);
  const lastRuntimeByProjectId = useAgentSessionStore((s) => s.lastRuntimeByProjectId);
  const setFocusedProject = useAgentSessionStore((s) => s.setFocusedProject);
  const selectConversation = useAgentSessionStore((s) => s.selectConversation);
  const clearSelection = useAgentSessionStore((s) => s.clearSelection);
  const setRuntimeForConversation = useAgentSessionStore((s) => s.setRuntimeForConversation);
  const setArtifactState = useAgentSessionStore((s) => s.setArtifactState);
  const clearAgentConversationSelection = useCallback(() => {
    setOptimisticSelectedConversationId(null);
    clearSelection();
  }, [clearSelection]);
  const [terminalChatDockElement, setTerminalChatDockElement] =
    useState<HTMLDivElement | null>(null);
  const [terminalPanelDockElement, setTerminalPanelDockElement] =
    useState<HTMLDivElement | null>(null);
  const artifactWidthCss = artifactPanelWidth
    ? `${artifactPanelWidth}px`
    : AGENTS_ARTIFACT_DEFAULT_WIDTH;

  const defaultProjectId = focusedProjectId || selectedProjectId || projectId || projects[0]?.id || null;
  const activeProjectId = selectedProjectId || defaultProjectId;
  const focusedConversations = useProjectAgentConversations(activeProjectId, showArchived);
  const selectedConversationQuery = useConversation(selectedConversationId, {
    enabled: !!selectedConversationId,
  });
  const selectedConversationData = selectedConversationQuery.data;
  const selectedConversationFallback = useMemo(() => {
    const conversation = selectedConversationData?.conversation;
    const isArchivedConversation = Boolean(conversation?.archivedAt);
    if (conversation) {
      if (
        conversation.id !== selectedConversationId ||
        conversation.contextType !== "project" ||
        conversation.contextId !== activeProjectId ||
        (showArchived ? !isArchivedConversation : isArchivedConversation)
      ) {
        return null;
      }

      return toProjectAgentConversation(conversation);
    }

    const optimisticConversation = selectedConversationId
      ? optimisticConversationsById[selectedConversationId]
      : null;
    if (
      !optimisticConversation ||
      optimisticConversation.contextType !== "project" ||
      optimisticConversation.contextId !== activeProjectId ||
      (showArchived
        ? !optimisticConversation.archivedAt
        : Boolean(optimisticConversation.archivedAt))
    ) {
      return null;
    }

    return optimisticConversation;
  }, [
    activeProjectId,
    optimisticConversationsById,
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
  const shouldHydrateAttachedIdeation =
    activeConversation?.contextType === "ideation" ||
    (activeConversation?.contextType === "project" &&
      (activeConversationMode === "ideation" ||
        Boolean(activeWorkspace?.linkedIdeationSessionId || activeWorkspace?.linkedPlanBranchId)));
  const selectedConversationMessages = useMemo(
    () =>
      selectedConversationData && selectedConversationData.conversation?.id === selectedConversationId
        ? selectedConversationData.messages
        : [],
    [selectedConversationData, selectedConversationId],
  );
  const attachedIdeationSessionId = useMemo(
    () =>
      shouldHydrateAttachedIdeation
        ? resolveAttachedIdeationSessionId(activeConversation, selectedConversationMessages)
        : null,
    [activeConversation, selectedConversationMessages, shouldHydrateAttachedIdeation],
  );
  const attachedIdeationSessionQuery = useQuery({
    queryKey: ideationKeys.sessionWithData(attachedIdeationSessionId ?? ""),
    queryFn: () => ideationApi.sessions.getWithData(attachedIdeationSessionId!),
    enabled: shouldHydrateAttachedIdeation && !!attachedIdeationSessionId,
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
  useAgentConversationTitleEvents(activeProjectId);
  useProjectAgentBridgeEvents({
    conversation: activeConversation,
    attachedIdeationSessionId,
    projectId: activeProjectId,
  });

  const cancelArtifactPersistenceJob = useCallback((conversationId: string) => {
    const job = artifactPersistenceJobsRef.current.get(conversationId);
    if (!job) {
      return;
    }
    if (job.frame !== null) {
      window.cancelAnimationFrame(job.frame);
    }
    if (job.timer !== null) {
      window.clearTimeout(job.timer);
    }
    artifactPersistenceJobsRef.current.delete(conversationId);
  }, []);

  const flushArtifactPersistenceJobs = useCallback(() => {
    for (const [conversationId, job] of Array.from(artifactPersistenceJobsRef.current)) {
      if (job.frame !== null) {
        window.cancelAnimationFrame(job.frame);
      }
      if (job.timer !== null) {
        window.clearTimeout(job.timer);
      }
      artifactPersistenceJobsRef.current.delete(conversationId);
      setArtifactState(conversationId, job.state);
    }
  }, [setArtifactState]);

  const cancelArtifactPanePreloadJob = useCallback(() => {
    const job = artifactPanePreloadJobRef.current;
    if (!job) {
      return;
    }
    if (job.frame !== null) {
      window.cancelAnimationFrame(job.frame);
    }
    if (job.timer !== null) {
      window.clearTimeout(job.timer);
    }
    artifactPanePreloadJobRef.current = null;
  }, []);

  const scheduleArtifactPanePreload = useCallback(() => {
    if (artifactPanePreloadJobRef.current) {
      return;
    }
    const job: DeferredFrameJob = {
      frame: null,
      timer: null,
    };
    job.frame = window.requestAnimationFrame(() => {
      job.frame = null;
      job.timer = window.setTimeout(() => {
        job.timer = null;
        artifactPanePreloadJobRef.current = null;
        void preloadAgentsArtifactPane().catch(() => undefined);
      }, 0);
    });
    artifactPanePreloadJobRef.current = job;
  }, []);

  const scheduleArtifactStatePersistence = useCallback(
    (conversationId: string, nextState: AgentArtifactState) => {
      cancelArtifactPersistenceJob(conversationId);
      const job: { frame: number | null; timer: number | null; state: AgentArtifactState } = {
        frame: null,
        timer: null,
        state: nextState,
      };
      job.frame = window.requestAnimationFrame(() => {
        job.frame = null;
        job.timer = window.setTimeout(() => {
          job.timer = null;
          artifactPersistenceJobsRef.current.delete(conversationId);
          setArtifactState(conversationId, nextState);
        }, 0);
      });
      artifactPersistenceJobsRef.current.set(conversationId, job);
    },
    [cancelArtifactPersistenceJob, setArtifactState],
  );

  useEffect(
    () => () => flushArtifactPersistenceJobs(),
    [flushArtifactPersistenceJobs],
  );

  useEffect(
    () => () => cancelArtifactPanePreloadJob(),
    [cancelArtifactPanePreloadJob],
  );

  const updateArtifactState = useCallback(
    (
      conversationId: string,
      updater: (current: AgentArtifactState) => AgentArtifactState,
    ) => {
      const currentState = getAgentArtifactStateSnapshot(conversationId, hasAutoOpenArtifacts);
      const nextState = updater(currentState);
      useAgentArtifactUiStore.getState().setArtifactState(conversationId, nextState);
      scheduleArtifactStatePersistence(conversationId, nextState);
    },
    [hasAutoOpenArtifacts, scheduleArtifactStatePersistence],
  );

  const setArtifactPaneVisibility = useCallback(
    (conversationId: string, isOpen: boolean) => {
      updateArtifactState(conversationId, (current) => ({
        ...current,
        isOpen,
      }));
    },
    [updateArtifactState],
  );

  const toggleArtifactPaneVisibility = useCallback(
    (conversationId: string) => {
      const currentState = getAgentArtifactStateSnapshot(
        conversationId,
        hasAutoOpenArtifacts,
      );
      setArtifactPaneVisibility(conversationId, !currentState.isOpen);
    },
    [hasAutoOpenArtifacts, setArtifactPaneVisibility],
  );

  const openArtifactTab = useCallback(
    (conversationId: string, tab: AgentArtifactTab) => {
      updateArtifactState(conversationId, (current) => ({
        ...current,
        activeTab: tab,
        isOpen: true,
      }));
    },
    [updateArtifactState],
  );

  const setArtifactTaskMode = useCallback(
    (conversationId: string, mode: AgentTaskArtifactMode) => {
      updateArtifactState(conversationId, (current) => ({
        ...current,
        taskMode: mode,
      }));
    },
    [updateArtifactState],
  );

  const handleArtifactResizeStart = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    const container = splitContainerRef.current;
    if (container) {
      const rect = container.getBoundingClientRect();
      artifactResizeBoundsRef.current = {
        right: rect.right,
        maxWidth: Math.max(
          AGENTS_ARTIFACT_MIN_WIDTH,
          rect.width - AGENTS_CHAT_MIN_WIDTH,
        ),
      };
    } else {
      artifactResizeBoundsRef.current = null;
    }
    pendingArtifactWidthRef.current = null;
    setIsArtifactResizing(true);
  }, []);

  const handleArtifactResizeReset = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    if (artifactResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(artifactResizeFrameRef.current);
      artifactResizeFrameRef.current = null;
    }
    pendingArtifactWidthRef.current = null;
    artifactResizeBoundsRef.current = null;
    setArtifactPanelWidth(null);
  }, []);

  const flushPendingArtifactWidth = useCallback(() => {
    if (artifactResizeFrameRef.current !== null) {
      window.cancelAnimationFrame(artifactResizeFrameRef.current);
      artifactResizeFrameRef.current = null;
    }
    const pendingWidth = pendingArtifactWidthRef.current;
    pendingArtifactWidthRef.current = null;
    if (pendingWidth !== null) {
      setArtifactPanelWidth(pendingWidth);
    }
  }, []);

  const scheduleArtifactWidth = useCallback((nextWidth: number) => {
    pendingArtifactWidthRef.current = nextWidth;
    if (artifactResizeFrameRef.current !== null) {
      return;
    }
    artifactResizeFrameRef.current = window.requestAnimationFrame(() => {
      artifactResizeFrameRef.current = null;
      const pendingWidth = pendingArtifactWidthRef.current;
      pendingArtifactWidthRef.current = null;
      if (pendingWidth !== null) {
        setArtifactPanelWidth(pendingWidth);
      }
    });
  }, []);

  useEffect(
    () => () => {
      if (artifactResizeFrameRef.current !== null) {
        window.cancelAnimationFrame(artifactResizeFrameRef.current);
      }
    },
    [],
  );

  useEffect(() => {
    if (!isArtifactResizing) {
      return;
    }

    const handleMouseMove = (event: MouseEvent) => {
      const container = splitContainerRef.current;
      if (!container) {
        return;
      }
      const bounds =
        artifactResizeBoundsRef.current ??
        (() => {
          const rect = container.getBoundingClientRect();
          const nextBounds = {
            right: rect.right,
            maxWidth: Math.max(
              AGENTS_ARTIFACT_MIN_WIDTH,
              rect.width - AGENTS_CHAT_MIN_WIDTH,
            ),
          };
          artifactResizeBoundsRef.current = nextBounds;
          return nextBounds;
        })();
      const nextWidth = bounds.right - event.clientX;
      scheduleArtifactWidth(
        Math.max(AGENTS_ARTIFACT_MIN_WIDTH, Math.min(bounds.maxWidth, nextWidth)),
      );
    };

    const handleMouseUp = () => {
      flushPendingArtifactWidth();
      artifactResizeBoundsRef.current = null;
      setIsArtifactResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [flushPendingArtifactWidth, isArtifactResizing, scheduleArtifactWidth]);

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

  useEffect(() => {
    if (!projectId || syncedProjectIdRef.current === projectId) {
      return;
    }
    syncedProjectIdRef.current = projectId;
    setFocusedProject(projectId);
  }, [projectId, setFocusedProject]);

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
      clearAgentConversationSelection();
    }
  }, [
    activeProjectId,
    clearAgentConversationSelection,
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
        toggleArtifactPaneVisibility(selectedConversationId);
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
        openArtifactTab(selectedConversationId, tab);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    openArtifactTab,
    selectedConversationId,
    toggleArtifactPaneVisibility,
  ]);

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

interface AgentsArtifactPaneRegionProps {
  conversationId: string;
  conversation: AgentConversation;
  workspace: AgentConversationWorkspace | null;
  hasAutoOpenArtifacts: boolean;
  artifactWidthCss: string;
  isArtifactResizing: boolean;
  onResizeStart: (event: ReactMouseEvent) => void;
  onResizeReset: (event: ReactMouseEvent) => void;
  onTabChange: (tab: AgentArtifactTab) => void;
  onTaskModeChange: (mode: AgentTaskArtifactMode) => void;
  onPublishWorkspace: (conversationId: string) => Promise<void>;
  isPublishingWorkspace: boolean;
  onClose: () => void;
  terminalUnavailableReason: string | null;
  setTerminalPanelDockElement: (element: HTMLDivElement | null) => void;
}

function AgentsArtifactPaneRegion({
  conversationId,
  conversation,
  workspace,
  hasAutoOpenArtifacts,
  artifactWidthCss,
  isArtifactResizing,
  onResizeStart,
  onResizeReset,
  onTabChange,
  onTaskModeChange,
  onPublishWorkspace,
  isPublishingWorkspace,
  onClose,
  terminalUnavailableReason,
  setTerminalPanelDockElement,
}: AgentsArtifactPaneRegionProps) {
  const { artifactState, artifactPaneOpen } = useResolvedAgentArtifactState(
    conversationId,
    hasAutoOpenArtifacts,
  );
  const contentMounted = useAfterPaintMounted(artifactPaneOpen);

  return (
    <>
      {artifactPaneOpen ? (
        <div className="max-lg:hidden">
          <ResizeHandle
            isResizing={isArtifactResizing}
            onMouseDown={onResizeStart}
            onDoubleClick={onResizeReset}
            testId="agents-artifact-resize-handle"
          />
        </div>
      ) : null}
      <div
        className={cn(
          "h-full shrink-0 overflow-hidden",
          artifactPaneOpen &&
            "max-lg:absolute max-lg:inset-y-0 max-lg:right-0 max-lg:z-20 max-lg:!w-[min(100%,420px)] max-lg:!min-w-0 max-lg:!max-w-none",
        )}
        style={{
          width: artifactPaneOpen ? artifactWidthCss : "0px",
          minWidth: artifactPaneOpen ? AGENTS_ARTIFACT_MIN_WIDTH : 0,
          maxWidth: artifactPaneOpen
            ? `calc(100% - ${AGENTS_CHAT_MIN_WIDTH}px)`
            : 0,
          opacity: artifactPaneOpen ? 1 : 0,
          pointerEvents: artifactPaneOpen ? "auto" : "none",
          transition: "none",
        }}
        data-testid="agents-artifact-resizable-pane"
      >
        <div className="flex h-full min-h-0 flex-col">
          {artifactPaneOpen ? (
            <div className="min-h-0 flex-1">
              {contentMounted ? (
                <Suspense fallback={<AgentArtifactPaneLoadingShell />}>
                  <LazyAgentsArtifactPane
                    conversation={conversation}
                    workspace={workspace}
                    activeTab={artifactState.activeTab}
                    taskMode={artifactState.taskMode}
                    onTabChange={onTabChange}
                    onTaskModeChange={onTaskModeChange}
                    onPublishWorkspace={onPublishWorkspace}
                    isPublishingWorkspace={isPublishingWorkspace}
                    onClose={onClose}
                  />
                </Suspense>
              ) : (
                <AgentArtifactPaneLoadingShell />
              )}
            </div>
          ) : null}
          <AgentsTerminalDockHost
            dock="panel"
            conversationId={conversationId}
            workspace={workspace}
            terminalUnavailableReason={terminalUnavailableReason}
            hasAutoOpenArtifacts={hasAutoOpenArtifacts}
            setDockElement={setTerminalPanelDockElement}
          />
        </div>
      </div>
    </>
  );
}

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

function AgentsChatHeaderController({
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

const AgentConversationBaseLine = memo(function AgentConversationBaseLine({
  workspace,
}: {
  workspace: AgentConversationWorkspace | null;
}) {
  if (!workspace) {
    return null;
  }

  const baseLabel = workspace.baseDisplayName ?? workspace.baseRef;
  const option: BranchBaseOption = {
    key: `${workspace.baseRefKind}:${workspace.baseRef}`,
    label: baseLabel,
    detail: workspace.baseDisplayName ? workspace.baseRef : undefined,
    source: "local",
    selection: {
      kind:
        workspace.baseRefKind === "project_default" ||
        workspace.baseRefKind === "current_branch" ||
        workspace.baseRefKind === "local_branch"
          ? workspace.baseRefKind
          : "local_branch",
      ref: workspace.baseRef,
      displayName: baseLabel,
    },
  };

  return (
    <div
      className="flex min-w-0 justify-end"
      data-testid="agents-conversation-base"
    >
      <BranchBasePicker
        value={option.key}
        onValueChange={() => undefined}
        options={[option]}
        placeholder="Base branch"
        readOnly
      />
    </div>
  );
});

function getAgentTerminalUnavailableReason(
  conversation: AgentConversation | null,
  workspace: AgentConversationWorkspace | null,
): string | null {
  if (!conversation) {
    return "Select an agent conversation";
  }
  if (conversation.contextType !== "project") {
    return "Terminal is available for project conversations";
  }
  if (!workspace) {
    return "Terminal requires a workspace-backed conversation";
  }
  if (workspace.status === "missing") {
    return "Terminal unavailable because the workspace is missing";
  }
  if (workspace.linkedIdeationSessionId || workspace.linkedPlanBranchId) {
    return "Terminal disabled while ideation or execution owns this workspace";
  }
  return null;
}

async function uploadDraftAttachment(conversationId: string, file: File): Promise<{ id: string }> {
  const arrayBuffer = await file.arrayBuffer();
  const fileData = Array.from(new Uint8Array(arrayBuffer));

  return invoke<{ id: string }>("upload_chat_attachment", {
    input: {
      conversationId,
      fileName: file.name,
      fileData,
      mimeType: file.type || undefined,
    },
  });
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
