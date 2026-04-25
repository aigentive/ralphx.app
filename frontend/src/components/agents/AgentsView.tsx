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
import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  ClipboardList,
  BrainCircuit,
  FileText,
  GitBranch,
  GitPullRequestArrow,
  Loader2,
  Menu,
  PanelRightOpen,
  PanelRightClose,
  Terminal as TerminalIcon,
} from "lucide-react";
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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ResizeHandle } from "@/components/ui/ResizeHandle";
import { BranchBasePicker } from "@/components/shared/BranchBasePicker";
import type { BranchBaseOption } from "@/components/shared/branchBaseOptions";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { chatKeys, invalidateConversationDataQueries, useConversation } from "@/hooks/useChat";
import { ideationKeys } from "@/hooks/useIdeation";
import { projectKeys, useProjects } from "@/hooks/useProjects";
import { useResponsiveSidebarLayout } from "@/hooks/useResponsiveSidebarLayout";
import { withAlpha } from "@/lib/theme-colors";
import { formatBranchDisplay } from "@/lib/branch-utils";
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
  AGENT_MODEL_OPTIONS,
  AGENT_PROVIDER_OPTIONS,
  normalizeRuntimeSelection,
} from "./agentOptions";
import { AgentComposerSurface } from "./AgentComposerSurface";
import { AgentTerminalDrawer } from "./AgentTerminalDrawer";
import { AgentsStartComposer } from "./AgentsStartComposer";
import {
  AGENT_TERMINAL_DEFAULT_HEIGHT,
  useAgentTerminalStore,
} from "./agentTerminalStore";
import {
  agentConversationKeys,
  useProjectAgentConversations,
} from "./useProjectAgentConversations";
import { archivedConversationCountKey } from "./useArchivedConversationCounts";
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
const AGENTS_SIDEBAR_COLLAPSE_STORAGE_KEY = "ralphx-agents-sidebar-collapsed";
const AGENT_CONVERSATION_MODE_OPTIONS: Array<{
  id: AgentConversationWorkspaceMode;
  label: string;
}> = [
  { id: "chat", label: "Chat" },
  { id: "edit", label: "Edit Agent" },
  { id: "ideation", label: "Ideation Mode" },
];

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
  const [manualArtifactVisibilityByConversationId, setManualArtifactVisibilityByConversationId] =
    useState<Record<string, boolean>>({});
  const splitContainerRef = useRef<HTMLDivElement>(null);
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
  const setArtifactOpen = useAgentSessionStore((s) => s.setArtifactOpen);
  const setArtifactTab = useAgentSessionStore((s) => s.setArtifactTab);
  const setTaskArtifactMode = useAgentSessionStore((s) => s.setTaskArtifactMode);
  const clearAgentConversationSelection = useCallback(() => {
    setOptimisticSelectedConversationId(null);
    clearSelection();
  }, [clearSelection]);
  const terminalOpenByConversationId = useAgentTerminalStore((s) => s.openByConversationId);
  const terminalHeightByConversationId = useAgentTerminalStore((s) => s.heightByConversationId);
  const setTerminalOpen = useAgentTerminalStore((s) => s.setOpen);
  const toggleTerminalOpen = useAgentTerminalStore((s) => s.toggleOpen);
  const setTerminalHeight = useAgentTerminalStore((s) => s.setHeight);
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
  const conversationWorkspaceQuery = useQuery({
    queryKey: ["agents", "conversation-workspace", selectedConversationId],
    queryFn: () => chatApi.getAgentConversationWorkspace(selectedConversationId!),
    enabled:
      !!selectedConversationId &&
      activeConversation?.contextType === "project",
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
  const hasManualArtifactVisibility =
    !!selectedConversationId &&
    Boolean(manualArtifactVisibilityByConversationId[selectedConversationId]);
  const artifactPaneOpen = hasAutoOpenArtifacts
    ? hasStoredArtifactState
      ? artifactState.isOpen
      : true
    : hasManualArtifactVisibility && artifactState.isOpen;
  useAgentConversationTitleEvents(activeProjectId);
  useProjectAgentBridgeEvents({
    conversation: activeConversation,
    attachedIdeationSessionId,
    projectId: activeProjectId,
  });

  const setArtifactPaneVisibility = useCallback(
    (conversationId: string, isOpen: boolean) => {
      setManualArtifactVisibilityByConversationId((current) => ({
        ...current,
        [conversationId]: true,
      }));
      setArtifactOpen(conversationId, isOpen);
    },
    [setArtifactOpen],
  );

  const toggleArtifactPaneVisibility = useCallback(
    (conversationId: string, isOpen: boolean) => {
      setManualArtifactVisibilityByConversationId((current) => ({
        ...current,
        [conversationId]: true,
      }));
      setArtifactOpen(conversationId, !isOpen);
    },
    [setArtifactOpen],
  );

  const openArtifactTab = useCallback(
    (conversationId: string, tab: AgentArtifactTab) => {
      setManualArtifactVisibilityByConversationId((current) => ({
        ...current,
        [conversationId]: true,
      }));
      setArtifactTab(conversationId, tab);
    },
    [setArtifactTab],
  );

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
  const activeWorkspace =
    conversationWorkspaceQuery.data ??
    (selectedConversationId
      ? optimisticWorkspacesByConversationId[selectedConversationId] ?? null
      : null);
  const activeConversationMode =
    activeConversation?.contextType === "project"
      ? resolveConversationAgentMode(activeConversation, activeWorkspace)
      : null;
  const activeConversationModeLocked =
    activeConversationMode === "ideation" || isWorkspaceModeLocked(activeWorkspace);
  const isTerminalOpen =
    selectedConversationId
      ? terminalOpenByConversationId[selectedConversationId] ?? false
      : false;
  const activeTerminalHeight =
    selectedConversationId
      ? terminalHeightByConversationId[selectedConversationId] ?? AGENT_TERMINAL_DEFAULT_HEIGHT
      : AGENT_TERMINAL_DEFAULT_HEIGHT;
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
    clearAgentConversationSelection();
  }, [clearAgentConversationSelection, projectId, setFocusedProject]);

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
        toggleArtifactPaneVisibility(selectedConversationId, artifactPaneOpen);
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
    artifactPaneOpen,
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
      if (artifactPaneOpen && artifactState.activeTab === tab) {
        setArtifactPaneVisibility(selectedConversationId, false);
        return;
      }
      openArtifactTab(selectedConversationId, tab);
    },
    [
      artifactState.activeTab,
      artifactPaneOpen,
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
      setPublishingConversationId(conversationId);
      try {
        const result = await chatApi.publishAgentConversationWorkspace(conversationId);
        const prLabel = result.prNumber ? `#${result.prNumber}` : result.prUrl;
        toast.success(prLabel ? `Published ${prLabel}` : "Published branch");
        await Promise.all([
          queryClient.invalidateQueries({
            queryKey: ["agents", "conversation-workspace", conversationId],
          }),
          conversation?.projectId
            ? invalidateProjectConversations(conversation.projectId)
            : Promise.resolve(),
        ]);
      } catch (err) {
        toast.error(err instanceof Error ? err.message : "Failed to publish branch");
      } finally {
        setPublishingConversationId(null);
      }
    },
    [findConversationById, invalidateProjectConversations, queryClient]
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
                        workspaceControls={
                          activeConversationMode ? (
                            <AgentConversationModeSelect
                              value={activeConversationMode}
                              onValueChange={handleActiveConversationModeChange}
                              disabled={
                                activeConversationModeLocked ||
                                composerProps.agentStatus !== "idle" ||
                                switchingConversationModeId === selectedConversationId
                              }
                            />
                          ) : undefined
                        }
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
                      <AgentConversationBaseLine
                        workspace={activeWorkspace}
                      />
                    </>
                  )}
                  {...(activeConversation.contextType === "project" && attachedIdeationSessionId
                    ? { additionalQuestionSessionIds: [attachedIdeationSessionId] }
                    : {})}
                  headerContent={
                    <AgentsChatHeader
                      conversation={activeConversation}
                      workspace={activeWorkspace}
                      artifactOpen={artifactPaneOpen}
                      activeArtifactTab={artifactState.activeTab}
                      terminalOpen={isTerminalOpen}
                      terminalUnavailableReason={terminalUnavailableReason}
                      onRenameConversation={handleRenameConversation}
                      onPublishWorkspace={handlePublishWorkspace}
                      onOpenPublishPane={handleOpenPublishPane}
                      isPublishingWorkspace={publishingConversationId === selectedConversationId}
                      onToggleTerminal={() => toggleTerminalOpen(selectedConversationId)}
                      onToggleArtifacts={() =>
                        toggleArtifactPaneVisibility(selectedConversationId, artifactPaneOpen)
                      }
                      onSelectArtifact={handleSelectArtifact}
                    />
                  }
                  emptyState={<div />}
                />
              </div>
              {isTerminalOpen && activeWorkspace && !terminalUnavailableReason && (
                <AgentTerminalDrawer
                  conversationId={selectedConversationId}
                  workspace={activeWorkspace}
                  height={activeTerminalHeight}
                  onHeightChange={(nextHeight) =>
                    setTerminalHeight(selectedConversationId, nextHeight)
                  }
                  onClose={() => setTerminalOpen(selectedConversationId, false)}
                />
              )}
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
                  workspace={activeWorkspace}
                  activeTab={artifactState.activeTab}
                  taskMode={artifactState.taskMode}
                  onTabChange={handleSelectArtifact}
                  onTaskModeChange={(mode) => setTaskArtifactMode(selectedConversationId, mode)}
                  onPublishWorkspace={handlePublishWorkspace}
                  isPublishingWorkspace={publishingConversationId === selectedConversationId}
                  onClose={() => setArtifactPaneVisibility(selectedConversationId, false)}
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
  workspace: AgentConversationWorkspace | null;
  artifactOpen: boolean;
  activeArtifactTab: AgentArtifactTab;
  terminalOpen?: boolean;
  terminalUnavailableReason?: string | null;
  onRenameConversation: (conversationId: string, title: string) => Promise<void>;
  onPublishWorkspace?: (conversationId: string) => Promise<void>;
  onOpenPublishPane?: () => void;
  isPublishingWorkspace?: boolean;
  onToggleTerminal?: () => void;
  onToggleArtifacts: () => void;
  onSelectArtifact: (tab: AgentArtifactTab) => void;
}

export function AgentsChatHeader({
  conversation,
  workspace,
  artifactOpen,
  activeArtifactTab,
  terminalOpen = false,
  terminalUnavailableReason = null,
  onRenameConversation,
  onPublishWorkspace,
  onOpenPublishPane,
  isPublishingWorkspace = false,
  onToggleTerminal,
  onToggleArtifacts,
  onSelectArtifact,
}: AgentsChatHeaderProps) {
  const title = conversation?.title || "Untitled agent";
  const conversationMode = conversation ? resolveConversationAgentMode(conversation, workspace) : null;
  const showIdeationArtifacts = conversationMode === "ideation";
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
      <div className="flex min-w-0 shrink items-center gap-2">
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
        </div>
        {workspace && <AgentsWorkspaceStatusPill workspace={workspace} />}
      </div>

      <div className="hidden md:flex items-center gap-1 ml-auto shrink-0">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 w-8 p-0"
              onClick={onToggleTerminal}
              disabled={!onToggleTerminal || Boolean(terminalUnavailableReason)}
              aria-label={terminalOpen ? "Close terminal" : "Open terminal"}
              data-testid="agents-terminal-toggle"
            >
              <TerminalIcon className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom" className="max-w-[280px] text-xs">
            {terminalUnavailableReason ??
              (terminalOpen ? "Close terminal" : "Open terminal")}
          </TooltipContent>
        </Tooltip>

        {conversation && workspace?.mode === "edit" && !workspace.linkedPlanBranchId && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 gap-1.5 px-2.5 text-xs"
                onClick={onOpenPublishPane}
                disabled={
                  !onPublishWorkspace ||
                  !onOpenPublishPane ||
                  isPublishingWorkspace ||
                  workspace.status === "missing"
                }
                aria-label="Commit and publish branch"
                data-testid="agents-publish-workspace"
              >
                {isPublishingWorkspace ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <GitPullRequestArrow className="h-3.5 w-3.5" />
                )}
                <span>Commit & Publish</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              Commit changes, push the branch, and create or update its PR
            </TooltipContent>
          </Tooltip>
        )}

        {showIdeationArtifacts && !artifactOpen &&
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

        {showIdeationArtifacts || artifactOpen ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-8 w-8 p-0"
                onClick={onToggleArtifacts}
                aria-label={artifactOpen ? "Close panel" : "Open artifacts"}
              >
                {artifactOpen ? (
                  <PanelRightClose className="w-4 h-4" />
                ) : (
                  <PanelRightOpen className="w-4 h-4" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              {artifactOpen ? "Close panel" : "Open artifacts"}
            </TooltipContent>
          </Tooltip>
        ) : null}
      </div>
    </div>
  );
}

function AgentConversationModeSelect({
  value,
  onValueChange,
  disabled,
}: {
  value: AgentConversationWorkspaceMode;
  onValueChange: (value: AgentConversationWorkspaceMode) => void;
  disabled: boolean;
}) {
  return (
    <div
      className="inline-flex min-h-10 max-w-[178px] flex-none items-center gap-2 rounded-[12px] border px-2.5 py-1.5"
      style={{
        background: "color-mix(in srgb, var(--bg-base) 24%, var(--bg-surface) 76%)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <div
        className="flex h-[24px] w-[24px] shrink-0 items-center justify-center rounded-full"
        style={{ color: "var(--text-secondary)" }}
      >
        <BrainCircuit className="h-[13px] w-[13px]" />
      </div>
      <div className="min-w-0">
        <div
          className="mb-0.5 text-[8px] font-medium uppercase tracking-[0.16em]"
          style={{ color: "var(--text-muted)" }}
        >
          Mode
        </div>
        <Select
          value={value}
          onValueChange={(nextValue) =>
            onValueChange(nextValue as AgentConversationWorkspaceMode)
          }
          disabled={disabled}
        >
          <SelectTrigger
            className="h-auto w-auto min-w-0 border-0 bg-transparent px-0 py-0 text-[12px] font-medium shadow-none outline-none ring-0 focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 [&>span]:max-w-full"
            style={{
              color: "var(--text-primary)",
              boxShadow: "none",
              outline: "none",
              WebkitAppearance: "none",
              appearance: "none",
            }}
            data-testid="agents-conversation-mode"
            data-theme-button-skip="true"
            aria-label="Agent mode"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {AGENT_CONVERSATION_MODE_OPTIONS.map((option) => (
              <SelectItem key={option.id} value={option.id}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  );
}

function AgentsWorkspaceStatusPill({
  workspace,
}: {
  workspace: AgentConversationWorkspace;
}) {
  const branch = formatBranchDisplay(workspace.branchName);
  const status =
    workspace.publicationPrStatus ?? workspace.publicationPushStatus ?? workspace.status;
  const statusLabel = status.replace(/_/g, " ");
  const baseLabel = workspace.baseDisplayName ?? workspace.baseRef;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          tabIndex={0}
          className="inline-flex min-w-0 max-w-[180px] items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium sm:max-w-[300px]"
          style={{
            color: "var(--text-secondary)",
            background: "var(--bg-surface)",
            borderColor: "var(--overlay-weak)",
          }}
          data-testid="agents-workspace-status"
        >
          <GitBranch className="h-3.5 w-3.5 shrink-0" />
          <span className="truncate font-mono">{branch.short}</span>
          <span
            className="h-1 w-1 shrink-0 rounded-full"
            style={{ background: "var(--accent-primary)" }}
          />
          <span className="shrink-0 capitalize">{statusLabel}</span>
        </div>
      </TooltipTrigger>
      <TooltipContent side="bottom" className="max-w-[360px] text-xs">
        <div className="space-y-1">
          <div>Branch: {branch.full}</div>
          <div>Base: {baseLabel}</div>
          {workspace.publicationPrUrl && (
            <div>
              PR:{" "}
              {workspace.publicationPrNumber
                ? `#${workspace.publicationPrNumber}`
                : workspace.publicationPrUrl}
            </div>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
}

function AgentConversationBaseLine({
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
      className="mt-2 flex w-full justify-end px-2"
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
}

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

function resolveConversationAgentMode(
  conversation: AgentConversation,
  workspace: AgentConversationWorkspace | null
): AgentConversationWorkspaceMode {
  return conversation.agentMode ?? workspace?.mode ?? "chat";
}

function isWorkspaceModeLocked(workspace: AgentConversationWorkspace | null): boolean {
  return Boolean(workspace?.linkedIdeationSessionId || workspace?.linkedPlanBranchId);
}
