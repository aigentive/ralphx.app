/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { useMemo, useState, useEffect, useCallback, useRef } from "react";
import { useShallow } from "zustand/react/shallow";
import { QueryClientProvider } from "@tanstack/react-query";
import { toast } from "sonner";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { ReviewsPanel } from "@/components/reviews/ReviewsPanel";
import { ExecutionControlBar } from "@/components/execution/ExecutionControlBar";
import { ChatPanel } from "@/components/Chat/ChatPanel";
import { KanbanSplitLayout, Navigation } from "@/components/layout";
import { PermissionDialog } from "@/components/PermissionDialog";
import { IdeationView, ProposalEditModal } from "@/components/Ideation";
import { ProposalDetailSheet } from "@/components/Ideation/ProposalDetailSheet";
import type { ProposalDetailEnrichment } from "@/components/Ideation/ProposalDetailSheet";
import { ExtensibilityView } from "@/components/ExtensibilityView";
import { ActivityView } from "@/components/activity";
import { SettingsView } from "@/components/settings";
import { InsightsView } from "@/components/views/InsightsView";
import { TeamSplitView } from "@/components/Team";
import { TaskGraphView } from "@/components/TaskGraph";
import { WelcomeScreen } from "@/components/WelcomeScreen";
import { UpdateChecker } from "@/components/UpdateChecker";
import { ProjectSelector } from "@/components/projects/ProjectSelector";
import { ProjectCreationWizard } from "@/components/projects/ProjectCreationWizard";
import { PlanQuickSwitcherPalette } from "@/components/plan/PlanQuickSwitcherPalette";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, selectTasksByStatus } from "@/stores/taskStore";
import { useChatStore } from "@/stores/chatStore";
import { useIdeationStore, selectActiveSession } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import { useProjectStore } from "@/stores/projectStore";
import type { ChatContext, ViewType } from "@/types/chat";
import type { ApplyProposalsInput } from "@/api/ideation.types";
import type { UpdateProposalInput } from "@/api/ideation";
import { toTaskProposal } from "@/api/ideation";
import type { CreateProject } from "@/types/project";
import { useTasksAwaitingReview } from "@/hooks/useReviews";
import { useReviewMutations } from "@/hooks/useReviewMutations";
import { useExecutionEvents } from "@/hooks/useExecutionEvents";
import { useExecutionStatus } from "@/hooks/useExecutionControl";
import { useRunningProcesses } from "@/hooks/useRunningProcesses";
import { useMergePipeline } from "@/hooks/useMergePipeline";
import { useProjects, projectKeys } from "@/hooks/useProjects";
import {
  useIdeationSession,
  useIdeationSessions,
  useArchiveIdeationSession,
} from "@/hooks/useIdeation";
import { useProposalMutations } from "@/hooks/useProposals";
import { useApplyProposals } from "@/hooks/useApplyProposals";
import { useAppKeyboardShortcuts } from "@/hooks/useAppKeyboardShortcuts";
import { useFeatureFlags, isViewEnabled } from "@/hooks/useFeatureFlags";
import { useNavCompactBreakpoint } from "@/hooks";
import { extractErrorMessage } from "@/lib/errors";
import { api, getGitBranches, getGitDefaultBranch } from "@/lib/tauri";
import { executionApi } from "@/api/execution";
import { tasksApi } from "@/api/tasks";
import type { SelectionSource } from "@/api/plan";
import type { ProjectSettings } from "@/types/settings";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  MessageSquare,
  CheckCircle,
  PanelRight,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Toaster } from "@/components/ui/sonner";
import { ScreenshotGalleryTestPage } from "@/test-pages/ScreenshotGalleryTest";

// Local storage key for persisting chat panel width
const CHAT_WIDTH_STORAGE_KEY = "ralphx-chat-panel-width";

const queryClient = getQueryClient();

/**
 * Test page router - checks URL params and returns test page if applicable
 * This is extracted to avoid hooks being called after conditional returns
 */
function getTestPage(): React.ReactElement | null {
  if (typeof window === "undefined") return null;

  const params = new URLSearchParams(window.location.search);
  const testPage = params.get("test");
  const scenario = params.get("scenario") || "default";

  if (testPage === "screenshot-gallery") {
    const scenarios: Record<string, React.ReactElement> = {
      default: <ScreenshotGalleryTestPage />,
      empty: <ScreenshotGalleryTestPage screenshots={[]} />,
      twoColumns: <ScreenshotGalleryTestPage columns={2} />,
      fourColumns: <ScreenshotGalleryTestPage columns={4} />,
    };
    return scenarios[scenario] ?? scenarios.default ?? null;
  }

  return null;
}

function FeatureDisabledPlaceholder({
  view,
  yamlKey,
  envVar,
}: {
  view: string;
  yamlKey: string;
  envVar: string;
}) {
  return (
    <div
      className="flex flex-col items-center justify-center h-full gap-4 p-8 text-center"
      data-testid={`feature-disabled-${view}`}
    >
      <p className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
        {view} page is disabled (dev mode)
      </p>
      <div className="text-xs font-mono rounded p-3 text-left" style={{ background: "var(--bg-surface)", color: "var(--text-secondary)" }}>
        <p className="mb-2 font-sans" style={{ color: "var(--text-muted)" }}>Enable via ralphx.yaml:</p>
        <pre>{`ui:\n  feature_flags:\n    ${yamlKey}: true`}</pre>
        <p className="mt-3 mb-1 font-sans" style={{ color: "var(--text-muted)" }}>Or via env var:</p>
        <pre>{`${envVar}=true`}</pre>
      </div>
    </div>
  );
}

function AppContent() {
  // Check for test page first (must happen before any hooks for ESLint compliance)
  const testPage = useMemo(() => getTestPage(), []);

  const reviewsPanelOpen = useUiStore((s) => s.reviewsPanelOpen);
  const toggleReviewsPanel = useUiStore((s) => s.toggleReviewsPanel);
  const setReviewsPanelOpen = useUiStore((s) => s.setReviewsPanelOpen);
  const executionStatus = useUiStore((s) => s.executionStatus);
  const setExecutionStatus = useUiStore((s) => s.setExecutionStatus);
  const currentView = useUiStore((s) => s.currentView);
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);
  // Unified chat visibility per view (replaces chatCollapsed and chatStore.isOpen)
  const chatVisibleByView = useUiStore((s) => s.chatVisibleByView);
  const toggleChatVisible = useUiStore((s) => s.toggleChatVisible);
  const graphRightPanelUserOpen = useUiStore((s) => s.graphRightPanelUserOpen);
  const graphRightPanelCompactOpen = useUiStore((s) => s.graphRightPanelCompactOpen);
  const toggleGraphRightPanelUserOpen = useUiStore((s) => s.toggleGraphRightPanel);
  const toggleGraphRightPanelCompactOpen = useUiStore(
    (s) => s.toggleGraphRightPanelCompactOpen
  );
  const battleModeActive = useUiStore((s) => s.battleModeActive);
  const enterBattleMode = useUiStore((s) => s.enterBattleMode);
  const exitBattleMode = useUiStore((s) => s.exitBattleMode);
  const { isNavCompact } = useNavCompactBreakpoint();
  const { data: featureFlags } = useFeatureFlags();

  // Redirect to kanban in production when the current view is disabled via feature flags
  useEffect(() => {
    if (!import.meta.env.DEV && !isViewEnabled(currentView, featureFlags)) {
      setCurrentView("kanban");
    }
  }, [currentView, featureFlags, setCurrentView]);

  // Welcome screen overlay state
  const showWelcomeOverlay = useUiStore((s) => s.showWelcomeOverlay);
  const welcomeOverlayReturnView = useUiStore((s) => s.welcomeOverlayReturnView);
  const openWelcomeOverlay = useUiStore((s) => s.openWelcomeOverlay);
  const closeWelcomeOverlay = useUiStore((s) => s.closeWelcomeOverlay);
  // Activity filter state (for context-aware navigation from StatusActivityBadge)
  const activityFilter = useUiStore((s) => s.activityFilter);

  // Chat panel state (width + message management)
  const chatWidth = useChatStore((s) => s.width);
  const setChatWidth = useChatStore((s) => s.setWidth);
  const clearMessages = useChatStore((s) => s.clearMessages);

  const switchToProject = useUiStore((s) => s.switchToProject);

  // Project state
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const setProjects = useProjectStore((s) => s.setProjects);
  const addProject = useProjectStore((s) => s.addProject);
  const selectProject = useProjectStore((s) => s.selectProject);

  const prevProjectIdRef = useRef<string | null>(activeProjectId);

  // Fetch projects from backend
  const { data: fetchedProjects, isLoading: isLoadingProjects } = useProjects();

  // Project creation wizard state
  const [isProjectWizardOpen, setIsProjectWizardOpen] = useState(false);
  const [isCreatingProject, setIsCreatingProject] = useState(false);
  const [projectCreationError, setProjectCreationError] = useState<string | null>(null);

  // Plan quick switcher state
  const [isPlanQuickSwitcherOpen, setIsPlanQuickSwitcherOpen] = useState(false);
  const [planQuickSwitcherSource, setPlanQuickSwitcherSource] =
    useState<SelectionSource>("quick_switcher");

  // Ideation state
  const activeSession = useIdeationStore(selectActiveSession);
  const setActiveSession = useIdeationStore((s) => s.setActiveSession);
  const selectSession = useIdeationStore((s) => s.selectSession);
  const archiveSessionInStore = useIdeationStore((s) => s.archiveSession);
  const activeSessionId = activeSession?.id ?? "";
  // Get raw proposals from store and memoize the filtered/sorted version
  const allProposals = useProposalStore((s) => s.proposals);
  const setProposals = useProposalStore((s) => s.setProposals);
  const proposals = useMemo(() => {
    if (!activeSessionId) return [];
    return Object.values(allProposals)
      .filter((p) => p.sessionId === activeSessionId)
      .sort((a, b) => a.sortOrder - b.sortOrder);
  }, [allProposals, activeSessionId]);
  const [editingProposalId, setEditingProposalId] = useState<string | null>(null);
  const editingProposal = editingProposalId
    ? allProposals[editingProposalId] ?? null
    : null;

  const [viewingProposalId, setViewingProposalId] = useState<string | null>(null);
  const [viewingEnrichment, setViewingEnrichment] = useState<ProposalDetailEnrichment | undefined>(undefined);
  const viewingProposal = viewingProposalId
    ? allProposals[viewingProposalId] ?? null
    : null;

  const [isExecutionLoading, setIsExecutionLoading] = useState(false);

  // Execution settings state (persisted to database)
  const [executionSettings, setExecutionSettings] = useState<ProjectSettings | null>(null);

  // Running processes data for popover
  const { data: runningProcessesData } = useRunningProcesses(activeProjectId ?? undefined);
  const [isLoadingSettings, setIsLoadingSettings] = useState(true);
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Check if we should show the empty state (no projects)
  // Use TanStack Query data directly — the Zustand store sync via useEffect
  // can lag behind, causing a brief flash where store.projects is {} while
  // fetchedProjects already has data.
  const hasNoProjects = !isLoadingProjects && (!fetchedProjects || fetchedProjects.length === 0);

  // Use active project ID (queries are disabled when null)
  const currentProjectId = activeProjectId ?? "";

  const { totalCount: pendingReviewCount } = useTasksAwaitingReview(currentProjectId);

  // Real-time execution status updates via Tauri events
  useExecutionEvents();
  // Fetch initial execution status and poll every 30s as fallback
  // Pass currentProjectId for per-project execution status scoping
  useExecutionStatus(currentProjectId || undefined);
  const { isApproving, isRequestingChanges } = useReviewMutations();

  // Merge pipeline data
  const { data: mergePipelineData } = useMergePipeline(activeProjectId ?? undefined);
  const mergingCount = useMemo(() => {
    if (!mergePipelineData) return 0;
    return mergePipelineData.active.length + mergePipelineData.waiting.length + mergePipelineData.needsAttention.length;
  }, [mergePipelineData]);
  const hasAttentionMerges = useMemo(() => {
    return (mergePipelineData?.needsAttention.length ?? 0) > 0;
  }, [mergePipelineData]);

  // Paused tasks (provider errors)
  // useShallow prevents infinite re-renders: selectTasksByStatus returns a new array
  // on every call via .filter(), and Zustand's default Object.is sees new !== old.
  const pausedTasks = useTaskStore(useShallow(selectTasksByStatus("paused")));
  const pausedCount = pausedTasks.length;

  // Ideation hooks
  const { data: sessionData, isLoading: isSessionLoading } = useIdeationSession(activeSession?.id ?? "");
  const { data: allSessions = [] } = useIdeationSessions(currentProjectId);
  const archiveSession = useArchiveIdeationSession();
  const { deleteProposal, reorder, updateProposal } = useProposalMutations();
  const { apply: applyProposalsMutation } = useApplyProposals();

  /**
   * Resolved session for IdeationView.
   *
   * When switching sessions, there's a race between:
   * 1. Zustand updating activeSession (instant)
   * 2. TanStack Query fetching the new session data (async)
   *
   * During this window, sessionData contains stale data from the previous session.
   * We only use sessionData.session when its ID matches the active session,
   * otherwise we fall back to activeSession from the store.
   *
   * This prevents "flash" where the old session briefly appears before the new one loads.
   */
  const resolvedSession = useMemo(() => {
    const fetchedSession = sessionData?.session;
    const isFetchedSessionCurrent = fetchedSession?.id === activeSession?.id;

    // Base: prefer fetched data when current, fallback to store session
    const base = (isFetchedSessionCurrent && fetchedSession) ? fetchedSession : activeSession;

    // Merge store verification fields when events have fired (seq > 0).
    // This ensures real-time Tauri events take precedence over stale React Query data.
    // Seq resets to 0 on session switch (new session has no events yet).
    if (base && activeSession && activeSession.id === base.id && (activeSession.verificationUpdateSeq ?? 0) > 0) {
      return {
        ...base,
        verificationStatus: activeSession.verificationStatus ?? base.verificationStatus,
        verificationInProgress: activeSession.verificationInProgress ?? base.verificationInProgress,
        gapScore: activeSession.gapScore !== undefined ? activeSession.gapScore : base.gapScore,
      };
    }

    return base;
  }, [sessionData?.session, activeSession]);

  // Mirror PlanningView's isReadOnly: sessions that are not "active" are read-only
  const isIdeationReadOnly = resolvedSession?.status !== "active";

  // Sync proposals from sessionData to the store
  useEffect(() => {
    if (sessionData?.proposals) {
      // Convert API response to store type using proper mapping function
      setProposals(sessionData.proposals.map(toTaskProposal));
    }
  }, [sessionData?.proposals, setProposals]);


  // Sync fetched projects to store and auto-select first project
  useEffect(() => {
    if (fetchedProjects && fetchedProjects.length > 0) {
      setProjects(fetchedProjects);
      // Auto-select first project if none is selected
      if (!activeProjectId) {
        const firstProject = fetchedProjects[0];
        if (firstProject) {
          selectProject(firstProject.id);
        }
      }
    }
  }, [fetchedProjects, setProjects, activeProjectId, selectProject]);

  // Phase 82: Notify backend of active project changes for scoped execution
  // Only send when we have an actual project ID — skip the initial null
  // that occurs before Zustand persist hydration from localStorage.
  useEffect(() => {
    if (activeProjectId) {
      executionApi.setActiveProject(activeProjectId).catch((err) => {
        console.error("Failed to set active project:", err);
      });
    }
  }, [activeProjectId]);

  // Project switch: save/restore per-project view + ideation session
  // Runs AFTER the setActiveProject backend sync effect (order matters in React)
  useEffect(() => {
    const prevId = prevProjectIdRef.current;
    prevProjectIdRef.current = activeProjectId;

    if (prevId !== activeProjectId && activeProjectId) {
      // Atomic view state save/clean/restore
      switchToProject(prevId, activeProjectId);

      // Restore ideation session (separate store, same synchronous tick)
      const sessionByProject = useUiStore.getState().sessionByProject;
      const restoredSessionId = sessionByProject[activeProjectId] ?? null;

      if (restoredSessionId) {
        const sessions = useIdeationStore.getState().sessions;
        if (sessions[restoredSessionId]) {
          setActiveSession(restoredSessionId);
        } else {
          // Session was deleted/not yet loaded — don't restore stale ID
          setActiveSession(null);
        }
      } else {
        setActiveSession(null);
      }
    }
  }, [activeProjectId, switchToProject, setActiveSession]);

  // Load persisted chat width from localStorage on mount
  useEffect(() => {
    const savedWidth = localStorage.getItem(CHAT_WIDTH_STORAGE_KEY);
    if (savedWidth) {
      const width = parseInt(savedWidth, 10);
      if (!isNaN(width)) {
        setChatWidth(width);
      }
    }
  }, [setChatWidth]);

  // Persist chat width to localStorage when it changes
  useEffect(() => {
    localStorage.setItem(CHAT_WIDTH_STORAGE_KEY, chatWidth.toString());
  }, [chatWidth]);

  // Load execution settings from database when project changes
  useEffect(() => {
    async function loadSettings() {
      try {
        setIsLoadingSettings(true);
        setSettingsError(null);
        // Phase 82: Pass currentProjectId for per-project settings
        const response = await executionApi.getSettings(currentProjectId || undefined);
        // Map API response (camelCase) to settings type (snake_case)
        setExecutionSettings({
          ...DEFAULT_PROJECT_SETTINGS,
          execution: {
            ...DEFAULT_PROJECT_SETTINGS.execution,
            max_concurrent_tasks: response.maxConcurrentTasks,
            project_ideation_max: response.projectIdeationMax,
            auto_commit: response.autoCommit,
            pause_on_failure: response.pauseOnFailure,
          },
        });
      } catch (err) {
        console.error("Failed to load execution settings:", err);
        setSettingsError(err instanceof Error ? err.message : "Failed to load settings");
        // Fall back to defaults
        setExecutionSettings(DEFAULT_PROJECT_SETTINGS);
      } finally {
        setIsLoadingSettings(false);
      }
    }
    loadSettings();
  }, [currentProjectId]);

  // Debounced handler for execution settings changes (300ms)
  const handleSettingsChange = useCallback((newSettings: ProjectSettings) => {
    // Update local state immediately for responsive UI
    setExecutionSettings(newSettings);
    setSettingsError(null);

    // Clear any pending save
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }

    // Debounce the API call
    saveTimeoutRef.current = setTimeout(async () => {
      try {
        setIsSavingSettings(true);
        // Phase 82: Pass currentProjectId for per-project settings
        await executionApi.updateSettings({
          maxConcurrentTasks: newSettings.execution.max_concurrent_tasks,
          projectIdeationMax: newSettings.execution.project_ideation_max,
          autoCommit: newSettings.execution.auto_commit,
          pauseOnFailure: newSettings.execution.pause_on_failure,
        }, currentProjectId || undefined);
      } catch (err) {
        console.error("Failed to save execution settings:", err);
        setSettingsError(err instanceof Error ? err.message : "Failed to save settings");
      } finally {
        setIsSavingSettings(false);
      }
    }, 300);
  }, [currentProjectId]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, []);

  // Build chat context based on current view
  const chatContext: ChatContext = useMemo(() => {
    if (currentView === "ideation") {
      if (activeSession) {
        return {
          view: "ideation",
          projectId: currentProjectId,
          ideationSessionId: activeSession.id,
        };
      }
      // No session yet - fall back to project context for chat
      return {
        view: "kanban",
        projectId: currentProjectId,
      };
    }
    return {
      view: currentView,
      projectId: currentProjectId,
    };
  }, [currentView, activeSession, currentProjectId]);

  // Phase 82: Pass currentProjectId to execution API calls for per-project scoping
  const handlePauseToggle = async () => {
    if (executionStatus.haltMode === "stopped") {
      toast.error("Execution was stopped. Restart tasks manually.");
      return;
    }
    setIsExecutionLoading(true);
    try {
      const response = executionStatus.isPaused
        ? await api.execution.resume(currentProjectId || undefined)
        : await api.execution.pause(currentProjectId || undefined);
      setExecutionStatus(response.status);
    } catch {
      toast.error(
        executionStatus.isPaused
          ? "Failed to resume execution"
          : "Failed to pause execution"
      );
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleStop = async () => {
    setIsExecutionLoading(true);
    try {
      const response = await api.execution.stop(currentProjectId || undefined);
      setExecutionStatus(response.status);
    } catch {
      toast.error("Failed to stop execution");
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handlePauseProcess = async (taskId: string) => {
    try {
      await tasksApi.pause(taskId);
      toast.success("Task paused");
    } catch {
      toast.error("Failed to pause task");
    }
  };

  const handleStopProcess = async (taskId: string) => {
    try {
      await tasksApi.stop(taskId);
      toast.success("Task stopped");
    } catch {
      toast.error("Failed to stop task");
    }
  };

  const handleOpenSettings = () => {
    setCurrentView("settings");
  };

  const handleBattleModeToggle = useCallback(() => {
    if (battleModeActive) {
      exitBattleMode();
      return;
    }
    enterBattleMode();
  }, [battleModeActive, enterBattleMode, exitBattleMode]);

  useEffect(() => {
    if (currentView !== "graph" && battleModeActive) {
      exitBattleMode();
    }
  }, [battleModeActive, currentView, exitBattleMode]);

  // Ideation handlers
  const handleNewSession = useCallback(() => {
    // Clear active session to show StartSessionPanel with mode selector
    setActiveSession(null);
  }, [setActiveSession]);

  const handleArchiveSession = useCallback(async (sessionId: string) => {
    try {
      await archiveSession.mutateAsync(sessionId);
      // Clean up stores to free memory
      archiveSessionInStore(sessionId);
      clearMessages(`session:${sessionId}`);
      setActiveSession(null);
    } catch {
      toast.error("Failed to archive session");
    }
  }, [archiveSession, setActiveSession, archiveSessionInStore, clearMessages]);

  const handleSelectSession = useCallback((sessionId: string) => {
    // Find the session in allSessions and select it atomically
    const session = allSessions.find((s) => s.id === sessionId);
    if (session) {
      selectSession(session);
    }
  }, [allSessions, selectSession]);

  const handleEditProposal = useCallback((proposalId: string) => {
    setEditingProposalId(proposalId);
  }, []);

  const handleViewProposal = useCallback((proposalId: string, enrichment: ProposalDetailEnrichment) => {
    setViewingProposalId(proposalId);
    setViewingEnrichment(enrichment);
  }, []);

  const handleNavigateToTaskFromSheet = useCallback((taskId: string) => {
    setCurrentView("kanban");
    setSelectedTaskId(taskId);
  }, [setCurrentView, setSelectedTaskId]);

  const handleSaveProposal = useCallback(
    async (proposalId: string, data: UpdateProposalInput) => {
      try {
        await updateProposal.mutateAsync({ proposalId, changes: data });
        setEditingProposalId(null);
        toast.success("Proposal updated");
      } catch {
        toast.error("Failed to update proposal");
      }
    },
    [updateProposal]
  );

  const handleRemoveProposal = useCallback((proposalId: string) => {
    deleteProposal.mutate(proposalId);
  }, [deleteProposal]);

  const handleReorderProposals = useCallback((proposalIds: string[]) => {
    if (activeSession) {
      reorder.mutate({ sessionId: activeSession.id, proposalIds });
    }
  }, [activeSession, reorder]);

  const handleApplyProposals = useCallback(async (options: ApplyProposalsInput) => {
    try {
      const result = await applyProposalsMutation.mutateAsync(options);
      if (result.sessionConverted) {
        const count = result.createdTaskIds.length;
        toast.success(`Plan accepted — ${count} ${count === 1 ? "task" : "tasks"} created`, {
          action: {
            label: "View Work",
            onClick: () => setCurrentView("graph"),
          },
          duration: 6000,
        });
      }
      return result;
    } catch (error) {
      toast.error(extractErrorMessage(error, "Failed to apply proposals"));
      throw error;
    }
  }, [applyProposalsMutation, setCurrentView]);

  // Project wizard handlers
  const handleOpenProjectWizard = useCallback(() => {
    setProjectCreationError(null);
    setIsProjectWizardOpen(true);
  }, []);

  const handleCloseProjectWizard = useCallback(() => {
    setIsProjectWizardOpen(false);
    setProjectCreationError(null);
  }, []);

  const handleCreateProject = useCallback(async (projectData: CreateProject) => {
    setIsCreatingProject(true);
    setProjectCreationError(null);
    try {
      // Call Tauri backend to create project
      const newProject = await api.projects.create(projectData);
      // Invalidate the projects query so the useEffect sync doesn't overwrite with stale data
      await queryClient.invalidateQueries({ queryKey: projectKeys.list() });
      addProject(newProject);
      selectProject(newProject.id);
      setIsProjectWizardOpen(false);
    } catch (error) {
      setProjectCreationError(error instanceof Error ? error.message : "Failed to create project");
    } finally {
      setIsCreatingProject(false);
    }
  }, [addProject, selectProject]);

  const handleBrowseFolder = useCallback(async (): Promise<string | null> => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Select Project Folder",
      });
      // selected is string | string[] | null for directories
      if (typeof selected === "string") {
        return selected;
      }
      return null;
    } catch {
      return null;
    }
  }, []);

  const handleFetchBranches = useCallback(async (workingDirectory: string): Promise<string[]> => {
    try {
      const branches = await getGitBranches(workingDirectory);
      return branches;
    } catch {
      return [];
    }
  }, []);

  const handleDetectDefaultBranch = useCallback(async (workingDirectory: string): Promise<string> => {
    // Use backend detection with fallback chain (origin/HEAD -> main -> master -> first branch)
    return getGitDefaultBranch(workingDirectory);
  }, []);

  // Handler for closing manually-opened welcome screen
  const handleCloseWelcomeOverlay = useCallback(() => {
    if (welcomeOverlayReturnView) {
      setCurrentView(welcomeOverlayReturnView);
    }
    closeWelcomeOverlay();
  }, [welcomeOverlayReturnView, setCurrentView, closeWelcomeOverlay]);

  // Handler for view changes - clears task selection to reset state
  const handleViewChange = useCallback((view: ViewType) => {
    // Close any open task detail panel when switching views
    setSelectedTaskId(null);
    setCurrentView(view);
  }, [setSelectedTaskId, setCurrentView]);

  // Keyboard shortcuts for view switching, chat toggle, reviews toggle, and project creation
  const handleToggleGraphRightPanel = useCallback(() => {
    if (isNavCompact) {
      toggleGraphRightPanelCompactOpen();
    } else {
      toggleGraphRightPanelUserOpen();
    }
  }, [isNavCompact, toggleGraphRightPanelCompactOpen, toggleGraphRightPanelUserOpen]);

  const handleOpenPlanQuickSwitcher = useCallback(
    (source: SelectionSource = "quick_switcher") => {
      setPlanQuickSwitcherSource(source);
      setIsPlanQuickSwitcherOpen(true);
    },
    []
  );

  useAppKeyboardShortcuts({
    currentView,
    setCurrentView: handleViewChange,
    toggleChatVisible,
    toggleReviewsPanel,
    toggleGraphRightPanel: handleToggleGraphRightPanel,
    openProjectWizard: handleOpenProjectWizard,
    hasProjects: !hasNoProjects,
    showWelcomeOverlay,
    openWelcomeOverlay,
    closeWelcomeOverlay,
    welcomeOverlayReturnView,
    openPlanQuickSwitcher: handleOpenPlanQuickSwitcher,
    onBattleModeToggle: handleBattleModeToggle,
    featureFlags,
  });

  // Global click handler to close quick switcher when clicking outside
  useEffect(() => {
    if (!isPlanQuickSwitcherOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      // Check if click is outside the quick switcher panel
      const target = e.target as HTMLElement;
      const quickSwitcherPanel = target.closest('[data-quick-switcher-panel]');

      if (!quickSwitcherPanel) {
        setIsPlanQuickSwitcherOpen(false);
      }
    };

    // Use capture phase to handle clicks before they bubble
    document.addEventListener('click', handleClickOutside, true);
    return () => document.removeEventListener('click', handleClickOutside, true);
  }, [isPlanQuickSwitcherOpen]);

  // Test page routing - return early if on a test page
  if (testPage) {
    return testPage;
  }

  const toastBottomOffset = (currentView === "kanban" || currentView === "graph") ? "92px" : "16px";
  const quickSwitcherAnchorSelector =
    currentView === "kanban"
      ? '[data-testid="kanban-split-left"]'
      : currentView === "graph"
        ? '[data-testid="graph-split-left"]'
        : undefined;

  return (
    <main
      className="h-screen flex flex-col overflow-hidden"
      style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}
    >
      {/* Update checker - runs on mount, shows toast if update available */}
      <UpdateChecker />

      {/* Header - macOS Tahoe Liquid Glass */}
      <TooltipProvider delayDuration={300}>
        <header
          className="fixed top-0 left-0 right-0 h-14 flex items-center justify-between pr-4 pl-24 border-b z-50 select-none"
          style={{
            background: "hsla(220 10% 8% / 0.85)",
            backdropFilter: "blur(24px)",
            WebkitBackdropFilter: "blur(24px)",
            borderColor: "hsla(220 10% 100% / 0.06)",
            boxShadow: "0 1px 0 hsla(220 10% 100% / 0.03)",
          }}
          data-tauri-drag-region
          data-testid="app-header"
        >
          {/* Left Section: Branding + Navigation */}
          <div className="flex items-center gap-6">
            {/* App Branding */}
            <h1
              className="text-xl font-bold tracking-tight select-none"
              style={{ color: "var(--text-primary)" }}
            >
              Ralph
              <span
                style={{
                  color: "#ff6b35",
                  textShadow: "0 0 12px rgba(255, 107, 53, 0.5)",
                }}
              >
                X
              </span>
            </h1>

            {/* View Navigation */}
            <Navigation currentView={currentView} onViewChange={handleViewChange} />
          </div>

          {/* Spacer */}
          <div className="flex-1" />

          {/* Right Section: Project Selector + Panel Toggles */}
          <div
            className="flex items-center gap-2"
            style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
          >
            {/* Project selector */}
            <div className="mr-2">
              <ProjectSelector onNewProject={handleOpenProjectWizard} align="end" />
            </div>
            {/* Chat Panel Toggle - hidden on ideation (has built-in chat) */}
            {currentView !== "ideation" && (() => {
              // Unified per-view visibility - same logic for all views
              const isExpanded = chatVisibleByView[currentView];
              const handleToggle = () => toggleChatVisible(currentView);

              return (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleToggle}
                      className={cn(
                        "gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                        isNavCompact ? "px-2" : isExpanded ? "px-3" : "px-2 xl:px-3"
                      )}
                      style={{
                        background: isExpanded
                          ? "rgba(255,107,53,0.1)"
                          : "transparent",
                        border: isExpanded ? "1px solid rgba(255,107,53,0.15)" : "1px solid transparent",
                        color: isExpanded ? "#ff6b35" : "rgba(255,255,255,0.5)",
                      }}
                      data-testid="chat-toggle"
                    >
                      <MessageSquare className="w-[18px] h-[18px] flex-shrink-0" />
                      <span className={cn(
                        "text-sm font-medium whitespace-nowrap",
                        isNavCompact ? "hidden" : isExpanded ? "inline" : "hidden xl:inline"
                      )}>
                        Chat
                      </span>
                      <kbd
                        className={cn(
                          "ml-1 px-1.5 py-0.5 text-xs rounded",
                          isNavCompact ? "hidden" : isExpanded ? "inline" : "hidden xl:inline"
                        )}
                        style={{
                          backgroundColor: "rgba(255,255,255,0.05)",
                          color: "rgba(255,255,255,0.4)",
                        }}
                      >
                        ⌘K
                      </kbd>
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom" className="text-xs">
                    Toggle Chat <kbd className="ml-1 opacity-70">⌘K</kbd>
                  </TooltipContent>
                </Tooltip>
              );
            })()}

            {/* Reviews Panel Toggle */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={toggleReviewsPanel}
                  className={cn(
                    "relative gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                    reviewsPanelOpen ? "px-3" : "px-2 xl:px-3"
                  )}
                  style={{
                    background: reviewsPanelOpen
                      ? "rgba(255,107,53,0.1)"
                      : "transparent",
                    border: reviewsPanelOpen ? "1px solid rgba(255,107,53,0.15)" : "1px solid transparent",
                    color: reviewsPanelOpen ? "#ff6b35" : "rgba(255,255,255,0.5)",
                  }}
                  data-testid="reviews-toggle"
                >
                  <CheckCircle className="w-[18px] h-[18px] flex-shrink-0" />
                  <span className={cn(
                    "text-sm font-medium whitespace-nowrap",
                    reviewsPanelOpen ? "inline" : "hidden xl:inline"
                  )}>
                    Reviews
                  </span>
                  {/* Badge with pending count */}
                  {pendingReviewCount > 0 && (
                    <span
                      className="absolute -top-1 -right-1 flex items-center justify-center min-w-[18px] h-[18px] px-1 text-xs font-bold rounded-full animate-badge-pop"
                      style={{
                        backgroundColor: "var(--status-warning)",
                        color: "white",
                      }}
                      data-testid="reviews-badge"
                    >
                      {pendingReviewCount > 9 ? "9+" : pendingReviewCount}
                    </span>
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                Toggle Reviews <kbd className="ml-1 opacity-70">⌘⇧R</kbd>
              </TooltipContent>
            </Tooltip>

            {/* Graph Right Panel Toggle (graph view only) */}
            {currentView === "graph" && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleToggleGraphRightPanel}
                    disabled={battleModeActive}
                    className="h-8 w-8 p-0 transition-all duration-150 active:scale-[0.98]"
                    style={{
                      background: (isNavCompact ? graphRightPanelCompactOpen : graphRightPanelUserOpen)
                        ? "rgba(255,107,53,0.1)"
                        : "transparent",
                      border: (isNavCompact ? graphRightPanelCompactOpen : graphRightPanelUserOpen)
                        ? "1px solid rgba(255,107,53,0.15)"
                        : "1px solid transparent",
                      color: (isNavCompact ? graphRightPanelCompactOpen : graphRightPanelUserOpen)
                        ? "#ff6b35"
                        : "rgba(255,255,255,0.5)",
                      opacity: battleModeActive ? 0.45 : 1,
                    }}
                    data-testid="graph-panel-toggle"
                  >
                    <PanelRight className="w-[18px] h-[18px]" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom" className="text-xs">
                  {battleModeActive
                    ? "Disabled during Battle Mode"
                    : <>Toggle Graph Panel <kbd className="ml-1 opacity-70">⌘L</kbd></>}
                </TooltipContent>
              </Tooltip>
            )}
          </div>
        </header>
      </TooltipProvider>

      {/* Spacer for fixed header */}
      <div className="h-14 flex-shrink-0" />

      {/* Main content area - shows WelcomeScreen or normal content */}
      {(hasNoProjects || showWelcomeOverlay) ? (
        /* Empty state or manual overlay: animated welcome screen */
        <WelcomeScreen
          onCreateProject={handleOpenProjectWizard}
          onClose={showWelcomeOverlay ? handleCloseWelcomeOverlay : undefined}
        />
      ) : (
        /* Normal content with view-specific content and optional panels */
        <div className="flex-1 flex overflow-hidden">
          {/* Main view area */}
          <div className="flex-1 flex flex-col overflow-hidden">
            <div className="flex-1 overflow-auto h-full">
              {currentView === "kanban" && (
                <KanbanSplitLayout
                  projectId={currentProjectId}
                  footer={
                    <ExecutionControlBar
                      projectId={currentProjectId}
                      runningCount={executionStatus.runningCount}
                      maxConcurrent={executionStatus.maxConcurrent}
                      queuedCount={executionStatus.queuedCount}
                      queuedMessageCount={executionStatus.queuedMessageCount ?? 0}
                      pausedCount={pausedCount}
                      pausedTasks={pausedTasks}
                      mergingCount={mergingCount}
                      hasAttentionMerges={hasAttentionMerges}
                      mergePipelineData={mergePipelineData ?? null}
                      isPaused={executionStatus.isPaused}
                      haltMode={executionStatus.haltMode}
                      isLoading={isExecutionLoading}
                      onPauseToggle={handlePauseToggle}
                      onStop={handleStop}
                      runningProcesses={runningProcessesData?.processes ?? []}
                      ideationSessions={runningProcessesData?.ideationSessions ?? []}
                      onPauseProcess={handlePauseProcess}
                      onStopProcess={handleStopProcess}
                      onOpenSettings={handleOpenSettings}
                    />
                  }
                >
                  <TaskBoard
                    projectId={currentProjectId}
                    onOpenPlanQuickSwitcher={handleOpenPlanQuickSwitcher}
                  />
                </KanbanSplitLayout>
              )}
              {currentView === "graph" && (
                <TaskGraphView
                  projectId={currentProjectId}
                  onOpenPlanQuickSwitcher={handleOpenPlanQuickSwitcher}
                  footer={
                    <ExecutionControlBar
                      projectId={currentProjectId}
                      runningCount={executionStatus.runningCount}
                      maxConcurrent={executionStatus.maxConcurrent}
                      queuedCount={executionStatus.queuedCount}
                      queuedMessageCount={executionStatus.queuedMessageCount ?? 0}
                      pausedCount={pausedCount}
                      pausedTasks={pausedTasks}
                      mergingCount={mergingCount}
                      hasAttentionMerges={hasAttentionMerges}
                      mergePipelineData={mergePipelineData ?? null}
                      isPaused={executionStatus.isPaused}
                      haltMode={executionStatus.haltMode}
                      isLoading={isExecutionLoading}
                      onPauseToggle={handlePauseToggle}
                      onStop={handleStop}
                      runningProcesses={runningProcessesData?.processes ?? []}
                      ideationSessions={runningProcessesData?.ideationSessions ?? []}
                      onPauseProcess={handlePauseProcess}
                      onStopProcess={handleStopProcess}
                      onOpenSettings={handleOpenSettings}
                    />
                  }
                />
              )}
              {currentView === "ideation" && (
                <IdeationView
                  session={resolvedSession}
                  proposals={proposals}
                  isSessionLoading={isSessionLoading}
                  onNewSession={handleNewSession}
                  onSelectSession={handleSelectSession}
                  onArchiveSession={handleArchiveSession}
                  onEditProposal={handleEditProposal}
                  onViewProposal={handleViewProposal}
                  selectedProposalId={viewingProposalId}
                  onRemoveProposal={handleRemoveProposal}
                  onReorderProposals={handleReorderProposals}
                  onApply={handleApplyProposals}
                  footer={
                    <ExecutionControlBar
                      projectId={currentProjectId}
                      runningCount={executionStatus.runningCount}
                      maxConcurrent={executionStatus.maxConcurrent}
                      queuedCount={executionStatus.queuedCount}
                      queuedMessageCount={executionStatus.queuedMessageCount ?? 0}
                      pausedCount={pausedCount}
                      pausedTasks={pausedTasks}
                      mergingCount={mergingCount}
                      hasAttentionMerges={hasAttentionMerges}
                      mergePipelineData={mergePipelineData ?? null}
                      isPaused={executionStatus.isPaused}
                      haltMode={executionStatus.haltMode}
                      isLoading={isExecutionLoading}
                      onPauseToggle={handlePauseToggle}
                      onStop={handleStop}
                      runningProcesses={runningProcessesData?.processes ?? []}
                      ideationSessions={runningProcessesData?.ideationSessions ?? []}
                      onPauseProcess={handlePauseProcess}
                      onStopProcess={handleStopProcess}
                      onOpenSettings={handleOpenSettings}
                    />
                  }
                />
              )}
              {currentView === "extensibility" && (
                isViewEnabled("extensibility", featureFlags)
                  ? <ExtensibilityView />
                  : import.meta.env.DEV
                    ? <FeatureDisabledPlaceholder view="extensibility" yamlKey="extensibility_page" envVar="RALPHX_UI_EXTENSIBILITY_PAGE" />
                    : null
              )}
              {currentView === "activity" && (
                isViewEnabled("activity", featureFlags)
                  ? (
                    <ActivityView
                      showHeader
                      {...(activityFilter.taskId && { taskId: activityFilter.taskId })}
                      {...(activityFilter.sessionId && { sessionId: activityFilter.sessionId })}
                    />
                  )
                  : import.meta.env.DEV
                    ? <FeatureDisabledPlaceholder view="activity" yamlKey="activity_page" envVar="RALPHX_UI_ACTIVITY_PAGE" />
                    : null
              )}
              {currentView === "insights" && <InsightsView />}
              {currentView === "settings" && (
                <SettingsView
                  {...(executionSettings && { initialSettings: executionSettings })}
                  isLoading={isLoadingSettings}
                  isSaving={isSavingSettings}
                  error={settingsError}
                  onSettingsChange={handleSettingsChange}
                />
              )}
              {currentView === "team" && <TeamSplitView />}
            </div>
        </div>

          {/* ReviewsPanel - floating overlay with Tahoe glass panel */}
          {reviewsPanelOpen && (
            <div
              className="fixed top-14 right-0 w-[400px] z-40 flex flex-col animate-[slide-in-right_200ms_ease-out]"
              style={{ bottom: "76px" }}
            >
              {/* Floating panel inner container */}
              <div
                className="flex flex-col flex-1 rounded-[10px] overflow-hidden"
                style={{
                  margin: "8px",
                  background: "hsla(220 10% 10% / 0.92)",
                  backdropFilter: "blur(20px) saturate(180%)",
                  WebkitBackdropFilter: "blur(20px) saturate(180%)",
                  border: "1px solid hsla(220 20% 100% / 0.08)",
                  boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
                }}
              >
                <ReviewsPanel
                  projectId={currentProjectId}
                  onClose={() => setReviewsPanelOpen(false)}
                  isApproving={isApproving}
                  isRequestingChanges={isRequestingChanges}
                />
              </div>
            </div>
          )}

          {/* ChatPanel - resizable side panel with Cmd+K toggle (not on kanban or ideation) */}
          {currentView !== "kanban" && currentView !== "ideation" && <ChatPanel context={chatContext} />}
        </div>
      )}

      {/* Project Creation Wizard */}
      <ProjectCreationWizard
        isOpen={isProjectWizardOpen}
        onClose={handleCloseProjectWizard}
        onCreate={handleCreateProject}
        onBrowseFolder={handleBrowseFolder}
        onFetchBranches={handleFetchBranches}
        onDetectDefaultBranch={handleDetectDefaultBranch}
        isCreating={isCreatingProject}
        error={projectCreationError}
        isFirstRun={hasNoProjects}
      />

      {/* Permission Dialog - Global UI-based permission approval */}
      <PermissionDialog />

      {/* Proposal Edit Modal - Edit ideation proposals */}
      <ProposalEditModal
        proposal={editingProposal}
        onSave={handleSaveProposal}
        onCancel={() => setEditingProposalId(null)}
        isSaving={updateProposal.isPending}
      />

      {/* Proposal Detail Sheet - Read-only detail view */}
      <ProposalDetailSheet
        proposal={viewingProposal}
        {...(viewingEnrichment !== undefined && { enrichment: viewingEnrichment })}
        isReadOnly={isIdeationReadOnly}
        onClose={() => { setViewingProposalId(null); setViewingEnrichment(undefined); }}
        onEdit={handleEditProposal}
        onDelete={handleRemoveProposal}
        onNavigateToTask={handleNavigateToTaskFromSheet}
      />

      {/* Plan Quick Switcher */}
      {!hasNoProjects && (
        <PlanQuickSwitcherPalette
          projectId={currentProjectId}
          isOpen={isPlanQuickSwitcherOpen}
          onClose={() => setIsPlanQuickSwitcherOpen(false)}
          selectionSource={planQuickSwitcherSource}
          {...(quickSwitcherAnchorSelector
            ? { anchorSelector: quickSwitcherAnchorSelector }
            : {})}
        />
      )}

      {/* Toast notifications */}
      <Toaster position="bottom-left" offset={toastBottomOffset} />
    </main>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <AppContent />
      </EventProvider>
    </QueryClientProvider>
  );
}

export default App;
