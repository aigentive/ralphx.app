/**
 * PlanningView - Premium Planning Interface
 *
 * Design: macOS Tahoe Liquid Glass
 * - Flat translucent surfaces with backdrop-blur
 * - Subtle borders and single shadows
 * - Warm ambient orange glow in backgrounds
 * - Clean, minimal aesthetic
 */

import { useState, useCallback, useRef, useEffect, useLayoutEffect, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  MessageSquare,
  Archive,
  Loader2,
  Upload,
  Sparkles,
  RotateCcw,
  RefreshCw,
  ArrowLeft,
} from "lucide-react";
import { useEventBus } from "@/providers/EventProvider";
import { toast } from "sonner";
import type {
  IdeationSession,
  TaskProposal,
} from "@/types/ideation";
import type { ApplyProposalsInput, ApplyProposalsResultResponse } from "@/api/ideation.types";
import { Button } from "@/components/ui/button";
import { ResizeHandle, CHAT_PANEL_DEFAULT_WIDTH, CHAT_PANEL_MIN_WIDTH } from "@/components/ui/ResizeHandle";
import { PlanDisplay } from "./PlanDisplay";
import type { TeamMetadata } from "./PlanDisplay";
import { TeamResearchView } from "./TeamResearchView";
import { getTeamArtifacts } from "@/api/team";
import type { TeamArtifactSummary } from "@/api/team";
import { useTeamStore } from "@/stores/teamStore";
import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import { usePlanStore } from "@/stores/planStore";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import { AcceptModal } from "./AcceptModal";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { ConversationEmptyState } from "./EmptyStates";
import { animationStyles } from "./PlanningView.constants";
import { PlanBrowser } from "./PlanBrowser";
import { StartSessionPanel } from "./StartSessionPanel";
import { ProposalsToolbar } from "./ProposalsToolbar";
import { TieredProposalList } from "./TieredProposalList";
import type { ProposalDetailEnrichment } from "./ProposalDetailSheet";
import { ProactiveSyncNotificationBanner } from "./ProactiveSyncNotification";
import { ProposalsEmptyState } from "./ProposalsEmptyState";
import { AcceptedSessionBanner } from "./AcceptedSessionBanner";
import { useIdeationHandlers } from "./useIdeationHandlers";
import { useFileDrop } from "@/hooks/useFileDrop";
import { useDependencyGraph } from "@/hooks/useDependencyGraph";
import { DropZoneOverlay } from "./DropZoneOverlay";
import { ideationApi, type SessionWithDataResponse } from "@/api/ideation";
import { chatApi } from "@/api/chat";
import { ReopenSessionDialog } from "./ReopenSessionDialog";
import type { ReopenMode } from "./ReopenSessionDialog";
import { useReopenSession, useResetAndReaccept, ideationKeys } from "@/hooks/useIdeation";
import { useVerificationEvents } from "@/hooks/useVerificationEvents";

// ============================================================================
// Types
// ============================================================================

interface PlanningViewProps {
  session: IdeationSession | null;
  sessions: IdeationSession[];
  proposals: TaskProposal[];
  isSessionLoading?: boolean;
  onNewSession: () => void;
  onSelectSession: (sessionId: string) => void;
  onArchiveSession: (sessionId: string) => void;
  onDeleteSession?: (sessionId: string) => void;
  onEditProposal: (proposalId: string) => void;
  onRemoveProposal: (proposalId: string) => void;
  onReorderProposals: (proposalIds: string[]) => void;
  onApply: (options: ApplyProposalsInput) => Promise<ApplyProposalsResultResponse>;
  /** Callback to open proposal detail sheet */
  onViewProposal?: (proposalId: string, enrichment: ProposalDetailEnrichment) => void;
  /** ID of currently selected proposal (for highlight state in cards) */
  selectedProposalId?: string | null;
  /** Footer slot for execution controls — renders below left section */
  footer?: React.ReactNode;
}

// Empty States extracted to separate files

// Plan Browser extracted to PlanBrowser.tsx

// Start Session Panel extracted to StartSessionPanel.tsx

// Proposal Card extracted to ProposalCard.tsx

// Proactive Sync Notification extracted to ProactiveSyncNotification.tsx

// Proposals Toolbar extracted to ProposalsToolbar.tsx

// ============================================================================
// Analysis Banner
// ============================================================================

/** Prominent banner shown below ProposalsToolbar while dependency analysis runs. */
export function AnalysisBanner() {
  return (
    <div
      data-testid="analysis-banner"
      className="flex items-center gap-2 px-4 py-2 shrink-0"
      style={{
        background: "hsla(14 100% 60% / 0.06)",
        borderBottom: "1px solid hsla(14 100% 60% / 0.15)",
      }}
    >
      <Loader2
        className="w-3.5 h-3.5 animate-spin shrink-0"
        style={{ color: "hsl(14 100% 60%)" }}
      />
      <span className="text-[12px]" style={{ color: "hsl(14 100% 65%)" }}>
        Analyzing dependencies — accept will be available when complete
      </span>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function PlanningView({
  session,
  sessions,
  proposals,
  isSessionLoading = false,
  onNewSession,
  onSelectSession,
  onArchiveSession,
  onDeleteSession,
  onEditProposal,
  onRemoveProposal,
  onReorderProposals,
  onApply,
  onViewProposal,
  selectedProposalId,
  footer,
}: PlanningViewProps) {
  const [chatPanelWidth, setChatPanelWidth] = useState(CHAT_PANEL_DEFAULT_WIDTH);
  const [isResizing, setIsResizing] = useState(false);
  const [isAcceptModalOpen, setIsAcceptModalOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const proposalsScrollRef = useRef<HTMLDivElement>(null);

  // Subscribe to backend verification state changes → invalidates TanStack Query caches
  useVerificationEvents();

  const queryClient = useQueryClient();

  const planArtifact = useIdeationStore((state) => state.planArtifact);

  // Fetch full verification data (currentRound, maxRounds, convergenceReason) beyond what session carries
  const { data: verificationData } = useQuery({
    queryKey: ["verification", session?.id],
    queryFn: () => ideationApi.verification.getStatus(session!.id),
    enabled: !!session?.id && !!planArtifact,
    staleTime: 30_000,
  });
  const ideationSettings = useIdeationStore((state) => state.ideationSettings);
  const fetchPlanArtifact = useIdeationStore((state) => state.fetchPlanArtifact);
  const showSyncNotification = useIdeationStore((state) => state.showSyncNotification);
  const syncNotification = useIdeationStore((state) => state.syncNotification);
  const dismissSyncNotification = useIdeationStore((state) => state.dismissSyncNotification);

  // Fetch dependency graph for the session
  const { data: dependencyGraph, isFetching: isDependencyUpdating } = useDependencyGraph(session?.id ?? "");

  // Build critical path set from the graph (TieredProposalList handles other computations)
  const criticalPathSet = useMemo(() => {
    if (!dependencyGraph) {
      return new Set<string>();
    }
    return new Set(dependencyGraph.criticalPath);
  }, [dependencyGraph]);

  // Dependency analysis loading state
  const [isAnalyzingDependencies, setIsAnalyzingDependencies] = useState(false);
  // Set to true after 90s frontend timeout fires or after an analysis_failed event.
  // Signals to the UI that the accept button should show "Accept without dependencies".
  const [analysisTimedOut, setAnalysisTimedOut] = useState(false);
  const analysisTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastDependencyFetchRef = useRef<boolean>(false);
  const lastDependencyToastAtRef = useRef<number | null>(null);
  const lastDependencyRefreshRequestedAt = useProposalStore((state) => state.lastDependencyRefreshRequestedAt);
  const lastProposalUpdatedAt = useProposalStore((state) => state.lastProposalUpdatedAt);
  const lastUpdatedProposalId = useProposalStore((state) => state.lastUpdatedProposalId);
  const autoOpenedPlanRef = useRef(false);
  const userOverrideRef = useRef(false);

  // Read-only mode: plans that are not active are read-only
  const isReadOnly = session?.status !== "active";

  // Reopen/Reset dialog state
  const [reopenDialogOpen, setReopenDialogOpen] = useState(false);
  const [reopenDialogMode, setReopenDialogMode] = useState<ReopenMode>("reopen");
  const reopenMutation = useReopenSession();
  const resetMutation = useResetAndReaccept();

  // Mount once near the root of the ideation feature tree to handle plan_verification:status_changed events
  useVerificationEvents();

  // Count tasks created from this session's proposals
  const sessionTaskCount = useMemo(
    () => proposals.filter((p) => p.createdTaskId != null).length,
    [proposals]
  );

  // Plan store actions
  const setActivePlan = usePlanStore((state) => state.setActivePlan);
  const clearActivePlan = usePlanStore((state) => state.clearActivePlan);
  const activePlanByProject = usePlanStore((state) => state.activePlanByProject);
  const activeProjectId = useProjectStore((state) => state.activeProjectId);

  const canReopen = isReadOnly && (session?.status === "accepted" || session?.status === "archived");
  const canResetReaccept = session?.status === "accepted";

  const handleOpenReopenDialog = useCallback((mode: ReopenMode) => {
    setReopenDialogMode(mode);
    setReopenDialogOpen(true);
  }, []);

  const handleConfirmReopen = useCallback(() => {
    if (!session) return;
    if (reopenDialogMode === "reopen") {
      reopenMutation.mutate(session.id, {
        onSuccess: async () => {
          setReopenDialogOpen(false);
          toast.success("Session reopened");

          // Clear active plan if this session was the active plan
          if (activeProjectId) {
            const activePlanId = activePlanByProject[activeProjectId];
            if (activePlanId === session.id) {
              try {
                await clearActivePlan(activeProjectId);
              } catch (err) {
                console.error("Failed to clear active plan:", err);
                toast.error("Failed to clear active plan");
              }
            }
          }
        },
        onError: (err) => toast.error(`Failed to reopen: ${err.message}`),
      });
    } else {
      resetMutation.mutate(
        { sessionId: session.id, proposalIds: proposals.map((p) => p.id) },
        {
          onSuccess: () => {
            setReopenDialogOpen(false);
            toast.success("Session reset and re-accepted");
          },
          onError: (err) => toast.error(`Failed to reset: ${err.message}`),
        }
      );
    }
  }, [session, reopenDialogMode, reopenMutation, resetMutation, proposals, activeProjectId, activePlanByProject, clearActivePlan]);

  // Get the event bus from context (TauriEventBus or MockEventBus)
  const eventBus = useEventBus();

  // Listen for dependency analysis events
  useEffect(() => {
    const sessionId = session?.id;
    if (!sessionId) return;

    // Listen for analysis started — start 90s safety timeout
    const unsubAnalysisStarted = eventBus.subscribe<{ sessionId: string }>(
      "dependencies:analysis_started",
      (payload) => {
        if (payload.sessionId === sessionId) {
          setIsAnalyzingDependencies(true);
          setAnalysisTimedOut(false);

          // 90-second frontend safety timeout: if no completion or failure event arrives,
          // reset the analyzing state so the UI doesn't stay stuck. The accept button will
          // show "Accept without dependencies" after this fires (escape hatch).
          if (analysisTimeoutRef.current) clearTimeout(analysisTimeoutRef.current);
          analysisTimeoutRef.current = setTimeout(() => {
            setIsAnalyzingDependencies(false);
            setAnalysisTimedOut(true);
            toast.warning("Dependency analysis is taking longer than expected. You can accept without dependencies.");
            analysisTimeoutRef.current = null;
          }, 90_000);
        }
      }
    );

    // Listen for suggestions applied — analysis succeeded
    const unsubSuggestionsApplied = eventBus.subscribe<{ sessionId: string; appliedCount: number }>(
      "dependencies:suggestions_applied",
      (payload) => {
        if (payload.sessionId === sessionId) {
          if (analysisTimeoutRef.current) {
            clearTimeout(analysisTimeoutRef.current);
            analysisTimeoutRef.current = null;
          }
          setIsAnalyzingDependencies(false);
          setAnalysisTimedOut(false);
          const count = payload.appliedCount;
          if (count > 0) {
            toast.success(`${count} ${count === 1 ? "dependency" : "dependencies"} added`);
          } else {
            toast.info("No new dependencies found");
          }
        }
      }
    );

    // Listen for analysis failed — backend timeout fired or agent crashed
    const unsubAnalysisFailed = eventBus.subscribe<{ sessionId: string; error: string }>(
      "dependencies:analysis_failed",
      (payload) => {
        if (payload.sessionId === sessionId) {
          if (analysisTimeoutRef.current) {
            clearTimeout(analysisTimeoutRef.current);
            analysisTimeoutRef.current = null;
          }
          setIsAnalyzingDependencies(false);
          setAnalysisTimedOut(true);
          toast.error("Dependency analysis failed", {
            action: {
              label: "Re-analyze",
              onClick: () => handleReanalyzeDependenciesRef.current?.(),
            },
          });
        }
      }
    );

    return () => {
      unsubAnalysisStarted();
      unsubSuggestionsApplied();
      unsubAnalysisFailed();
      if (analysisTimeoutRef.current) {
        clearTimeout(analysisTimeoutRef.current);
        analysisTimeoutRef.current = null;
      }
    };
  }, [eventBus, session?.id]);

  // Hydrate isAnalyzingDependencies from backend state on mount and after refetches.
  // The backend persists analysis state in the analyzing_dependencies HashSet and returns
  // it via analysis_in_progress in the dependency graph response. This survives page
  // refreshes and navigation — local useState(false) would reset on unmount.
  useEffect(() => {
    if (dependencyGraph?.analysisInProgress === true) {
      setIsAnalyzingDependencies(true);
    }
  }, [dependencyGraph?.analysisInProgress]);

  // Small UX hint when dependency graph refreshes automatically
  useEffect(() => {
    if (!session?.id || proposals.length === 0) {
      lastDependencyFetchRef.current = false;
      return;
    }

    if (
      !lastDependencyRefreshRequestedAt
      || lastDependencyRefreshRequestedAt === lastDependencyToastAtRef.current
    ) {
      lastDependencyFetchRef.current = false;
      return;
    }

    if (isDependencyUpdating) {
      lastDependencyFetchRef.current = true;
      return;
    }

    if (lastDependencyFetchRef.current) {
      toast.success("Dependencies updated");
      lastDependencyFetchRef.current = false;
      lastDependencyToastAtRef.current = lastDependencyRefreshRequestedAt;
    }
  }, [isDependencyUpdating, session?.id, proposals.length, lastDependencyRefreshRequestedAt]);

  // Manual re-trigger dependency analysis
  const handleReanalyzeDependencies = useCallback(async () => {
    if (!session || isAnalyzingDependencies || proposals.length < 2) return;
    setAnalysisTimedOut(false);
    try {
      await ideationApi.sessions.spawnDependencySuggester(session.id);
    } catch (err) {
      console.error("Failed to spawn dependency suggester:", err);
      toast.error("Failed to analyze dependencies");
    }
  }, [session, isAnalyzingDependencies, proposals.length]);

  // Stable ref so event subscription closure always calls latest handleReanalyzeDependencies
  const handleReanalyzeDependenciesRef = useRef(handleReanalyzeDependencies);
  useEffect(() => { handleReanalyzeDependenciesRef.current = handleReanalyzeDependencies; }, [handleReanalyzeDependencies]);

  // Stable ref for fetchPlanArtifact to avoid re-triggering the effect
  // when the Zustand action reference changes.
  const fetchPlanArtifactRef = useRef(fetchPlanArtifact);
  useEffect(() => { fetchPlanArtifactRef.current = fetchPlanArtifact; }, [fetchPlanArtifact]);

  const planArtifactId = planArtifact?.id ?? null;
  useEffect(() => {
    if (!session?.planArtifactId) return;
    if (planArtifactId !== session.planArtifactId) {
      fetchPlanArtifactRef.current(session.planArtifactId);
    }
  }, [session?.planArtifactId, planArtifactId]);

  // Fetch team artifact summaries for team-ideated sessions
  // Refetches when artifactVersion bumps (from team:artifact_created events)
  const [teamArtifacts, setTeamArtifacts] = useState<TeamArtifactSummary[]>([]);
  const artifactVersion = useTeamStore((s) => s.artifactVersion[session?.id ?? ""] ?? 0);
  useEffect(() => {
    const hasTeamMode = session?.teamMode && session.teamMode !== "solo";
    const hasArtifacts = artifactVersion > 0;
    if (!session?.id || (!hasTeamMode && !hasArtifacts)) {
      setTeamArtifacts([]);
      return;
    }

    let cancelled = false;
    getTeamArtifacts(session.id)
      .then((resp) => {
        if (cancelled) return;
        setTeamArtifacts(resp.artifacts);
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to fetch team artifacts:", err);
        setTeamArtifacts([]);
      });

    return () => { cancelled = true; };
  }, [session?.id, session?.teamMode, artifactVersion]);

  // Tab state for plan vs research content
  const [activeTab, setActiveTab] = useState<"plan" | "research">("plan");

  // Reset to plan tab when switching sessions
  useEffect(() => {
    setActiveTab("plan");
  }, [session?.id]);

  // Construct TeamMetadata when session is a team session (badge only — findings shown via chips)
  const teamMetadata = useMemo<TeamMetadata | undefined>(() => {
    if (!session?.teamMode || session.teamMode === "solo") return undefined;
    return {
      teamIdeated: true,
      teamMode: session.teamMode as "research" | "debate",
      teammateCount: session.teamConfig?.maxTeammates ?? teamArtifacts.length,
      findings: [],
    };
  }, [session?.teamMode, session?.teamConfig?.maxTeammates, teamArtifacts.length]);

  useEffect(() => {
    const unsubProposalsUpdate = eventBus.subscribe<{ artifact_id: string; proposal_ids: string[]; session_id?: string }>(
      "plan:proposals_may_need_update",
      (payload) => {
        // Only show notification for the active session
        if (payload.session_id && session?.id && payload.session_id !== session.id) {
          return;
        }

        const affectedProposals = proposals.filter((p) => payload.proposal_ids.includes(p.id));
        const previousStates: Record<string, unknown> = {};
        affectedProposals.forEach((p) => { previousStates[p.id] = { ...p }; });

        showSyncNotification({
          artifactId: payload.artifact_id,
          proposalIds: payload.proposal_ids,
          previousStates,
          timestamp: Date.now(),
        });
      }
    );

    return () => { unsubProposalsUpdate(); };
  }, [eventBus, proposals, showSyncNotification, session?.id]);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      // Chat panel is on the right, so width = container right edge - mouse position
      const newWidth = rect.right - e.clientX;
      setChatPanelWidth(Math.max(CHAT_PANEL_MIN_WIDTH, Math.min(600, newWidth)));
    };

    const handleMouseUp = () => setIsResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  // Navigate to task handler - switches to kanban view and selects the task
  const setCurrentView = useUiStore((state) => state.setCurrentView);
  const setSelectedTaskId = useUiStore((state) => state.setSelectedTaskId);

  // Get active project for feature branch setting
  const activeProject = useProjectStore(selectActiveProject);

  // Accept Plan - opens the confirmation modal (instead of applying directly)
  const handleAcceptPlan = useCallback(() => {
    setIsAcceptModalOpen(true);
  }, []);

  // Handle confirmed accept from modal - applies proposals with user-selected options
  const handleAcceptConfirm = useCallback(async (options: ApplyProposalsInput) => {
    if (!session) return;
    const projectId = activeProjectId || session.projectId;
    if (!projectId) return;

    // Close modal immediately
    setIsAcceptModalOpen(false);

    // Apply proposals to Kanban and capture executionPlanId from the response
    let executionPlanId: string | null | undefined;
    try {
      const applyResult = await onApply(options);
      executionPlanId = applyResult?.executionPlanId;
    } catch {
      return; // Apply failed, toast already shown, don't proceed to setActivePlan
    }

    // Set this session as the active plan — pass executionPlanId for atomic update
    try {
      await setActivePlan(projectId, session.id, "ideation", executionPlanId);
    } catch (error) {
      console.error("Failed to set active plan:", error);
      toast.error("Failed to set active plan");
    }
  }, [session, onApply, activeProjectId, setActivePlan]);

  const handleAcceptCancel = useCallback(() => {
    setIsAcceptModalOpen(false);
  }, []);

  const handleNavigateToTask = useCallback((taskId: string) => {
    setCurrentView("kanban");
    setSelectedTaskId(taskId);
  }, [setCurrentView, setSelectedTaskId]);

  const handleViewWork = useCallback(async () => {
    if (session) {
      const projectId = activeProjectId || session.projectId;
      if (projectId) {
        try {
          await setActivePlan(projectId, session.id, "ideation");
        } catch (error) {
          console.error("Failed to set active plan before navigating to graph:", error);
        }
      }
    }
    setCurrentView("graph");
  }, [session, activeProjectId, setActivePlan, setCurrentView]);

  const {
    highlightedProposalIds,
    isPlanExpanded,
    setIsPlanExpanded,
    importStatus,
    setImportStatus,
    fileInputRef,
    handleArchive,
    handleClearAll,
    handleReviewSync,
    handleUndoSync,
    handleDismissSync,
    handleImportPlan,
    handleFileSelected,
    handleFileDrop,
  } = useIdeationHandlers(
    session,
    proposals,
    onRemoveProposal,
    onReorderProposals,
    onArchiveSession,
    fetchPlanArtifact,
    dismissSyncNotification,
    syncNotification
  );


  // File drop hook for drag-and-drop markdown import
  const { isDragging, dropProps, error: fileDropError } = useFileDrop({
    acceptedExtensions: [".md"],
    onFileDrop: handleFileDrop,
    onError: (err) => setImportStatus({ type: "error", message: err.message }),
  });

  // Show file drop error in import status
  useEffect(() => {
    if (fileDropError) {
      setImportStatus({ type: "error", message: fileDropError.message });
      setTimeout(() => setImportStatus(null), 5000);
    }
  }, [fileDropError, setImportStatus]);

  // Auto-expand plan when there are no proposals
  useEffect(() => {
    if (userOverrideRef.current) return;
    if (isSessionLoading) return;
    if (planArtifact && proposals.length === 0 && !isPlanExpanded) {
      autoOpenedPlanRef.current = true;
      setIsPlanExpanded(true);
    }
  }, [planArtifact, proposals.length, isPlanExpanded, setIsPlanExpanded, isSessionLoading]);

  // Auto-collapse plan when new proposal arrives
  const lastProposalAddedAt = useProposalStore((state) => state.lastProposalAddedAt);
  const prevProposalAddedAtRef = useRef(lastProposalAddedAt);
  useEffect(() => {
    const changed = lastProposalAddedAt !== prevProposalAddedAtRef.current;
    prevProposalAddedAtRef.current = lastProposalAddedAt;
    if (!changed) return;
    if (userOverrideRef.current) return;
    if (lastProposalAddedAt !== null && isPlanExpanded) {
      autoOpenedPlanRef.current = false;
      setIsPlanExpanded(false);
    }
  }, [lastProposalAddedAt, isPlanExpanded, setIsPlanExpanded]);

  // If proposals load after an auto-open, collapse the plan
  useEffect(() => {
    if (userOverrideRef.current) return;
    if (isSessionLoading) return;
    if (proposals.length > 0 && isPlanExpanded && autoOpenedPlanRef.current) {
      autoOpenedPlanRef.current = false;
      setIsPlanExpanded(false);
    }
  }, [proposals.length, isPlanExpanded, isSessionLoading, setIsPlanExpanded]);

  // Reset plan expansion when switching sessions — default to expanded if session already has a plan
  const lastSessionIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (!session?.id) return;
    if (lastSessionIdRef.current !== session.id) {
      lastSessionIdRef.current = session.id;
      autoOpenedPlanRef.current = false;
      userOverrideRef.current = false;
      // Auto-expand on initial load if the session already has a plan
      setIsPlanExpanded(!!session.planArtifactId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.id, session?.planArtifactId]);

  const handlePlanExpandedChange = useCallback((expanded: boolean) => {
    autoOpenedPlanRef.current = false;
    userOverrideRef.current = true;
    setIsPlanExpanded(expanded);
  }, [setIsPlanExpanded]);

  // ── Verification action handlers ─────────────────────────────────────────

  // Trigger verification by sending 'verify' message to the ideation orchestrator
  const handleTriggerVerification = useCallback(async () => {
    if (!session) return;
    try {
      await chatApi.sendAgentMessage("ideation", session.id, "verify");
    } catch (err) {
      console.error("Failed to trigger verification:", err);
      toast.error("Failed to start verification");
    }
  }, [session]);

  const handleSkipVerification = useCallback(async () => {
    if (!session) return;
    // Optimistic update: immediately set verificationStatus to 'skipped' for instant accept button enablement
    queryClient.setQueryData<SessionWithDataResponse | null>(
      ideationKeys.sessionWithData(session.id),
      (old) => old ? { ...old, session: { ...old.session, verificationStatus: "skipped" } } : old
    );
    try {
      await ideationApi.verification.skip(session.id);
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessionWithData(session.id) });
      queryClient.invalidateQueries({ queryKey: ["verification", session.id] });
    } catch (err) {
      // Roll back optimistic update on failure
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessionWithData(session.id) });
      console.error("Failed to skip verification:", err);
      toast.error("Failed to skip verification");
    }
  }, [session, queryClient]);

  // planVersionBeforeVerification: not yet surfaced by the API — PlanDisplay hides the button until defined
  const planVersionBeforeVerification: string | undefined = undefined;

  const handleRevertAndSkip = useCallback(async () => {
    if (!session || !planVersionBeforeVerification) return;
    try {
      await ideationApi.verification.revertAndSkip(session.id, planVersionBeforeVerification);
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessionWithData(session.id) });
      queryClient.invalidateQueries({ queryKey: ["verification", session.id] });
    } catch (err) {
      console.error("Failed to revert and skip verification:", err);
      toast.error("Failed to revert plan");
    }
  }, [session, queryClient, planVersionBeforeVerification]);

  // ── End verification handlers ─────────────────────────────────────────────

  // Historical plan version state - set when user clicks "View plan as of proposal creation (vX)"
  const [historicalPlanVersion, setHistoricalPlanVersion] = useState<number | null>(null);

  const handleViewHistoricalPlan = useCallback((_artifactId: string, version: number) => {
    setHistoricalPlanVersion(version);
    userOverrideRef.current = true;
    setIsPlanExpanded(true);
  }, [setIsPlanExpanded]);

  // Auto-scroll to bottom only when a new proposal is added
  const lastScrollSessionIdRef = useRef<string | null>(null);
  const lastScrollProposalAddedAtRef = useRef<number | null>(null);
  const lastScrollProposalUpdatedAtRef = useRef<number | null>(null);
  const [recentlyUpdatedProposalId, setRecentlyUpdatedProposalId] = useState<string | null>(null);
  useLayoutEffect(() => {
    const currentSessionId = session?.id ?? null;
    if (lastScrollSessionIdRef.current !== currentSessionId) {
      lastScrollSessionIdRef.current = currentSessionId;
      lastScrollProposalAddedAtRef.current = null;
      if (proposalsScrollRef.current) {
        proposalsScrollRef.current.scrollTo({ top: 0, behavior: "auto" });
      }
      return;
    }

    if (!lastProposalAddedAt || lastProposalAddedAt === lastScrollProposalAddedAtRef.current) {
      return;
    }

    if (proposalsScrollRef.current) {
      proposalsScrollRef.current.scrollTo({
        top: proposalsScrollRef.current.scrollHeight,
        behavior: "smooth",
      });
    }
    lastScrollProposalAddedAtRef.current = lastProposalAddedAt;
  }, [lastProposalAddedAt, session?.id]);

  // Auto-scroll to updated proposal (from chat updates or edits)
  useLayoutEffect(() => {
    if (!lastProposalUpdatedAt || lastProposalUpdatedAt === lastScrollProposalUpdatedAtRef.current) {
      return;
    }

    if (!lastUpdatedProposalId) {
      return;
    }

    if (!proposals.some((p) => p.id === lastUpdatedProposalId)) {
      return;
    }

    if (proposalsScrollRef.current) {
      const target = proposalsScrollRef.current.querySelector(
        `[data-testid="proposal-card-${lastUpdatedProposalId}"]`
      );
      if (target instanceof HTMLElement) {
        target.scrollIntoView({ behavior: "smooth", block: "center" });
      }
    }

    setRecentlyUpdatedProposalId(lastUpdatedProposalId);
    lastScrollProposalUpdatedAtRef.current = lastProposalUpdatedAt;
  }, [lastProposalUpdatedAt, lastUpdatedProposalId, proposals]);

  useEffect(() => {
    if (!recentlyUpdatedProposalId) return;
    const timeout = setTimeout(() => setRecentlyUpdatedProposalId(null), 2400);
    return () => clearTimeout(timeout);
  }, [recentlyUpdatedProposalId]);

  const highlightedProposalIdsWithUpdates = useMemo(() => {
    if (!recentlyUpdatedProposalId) return highlightedProposalIds;
    const updated = new Set(highlightedProposalIds);
    updated.add(recentlyUpdatedProposalId);
    return updated;
  }, [highlightedProposalIds, recentlyUpdatedProposalId]);

  return (
    <>
      <style>{animationStyles}</style>
      <div
        ref={containerRef}
        data-testid="ideation-view"
        className="flex flex-col h-full relative"
        style={{ background: "hsl(220 10% 8%)" }}
        role="main"
      >
        <div className="flex flex-1 overflow-hidden">
          {/* Plan Browser Sidebar */}
          <PlanBrowser
            sessions={sessions}
            projectId={activeProjectId || session?.projectId || ""}
            currentPlanId={session?.id ?? null}
            onSelectPlan={onSelectSession}
            onNewPlan={onNewSession}
            {...(onDeleteSession !== undefined && { onDeletePlan: onDeleteSession })}
            onArchivePlan={onArchiveSession}
            onReopenPlan={(planId) => {
              onSelectSession(planId);
              handleOpenReopenDialog("reopen");
            }}
            onResetReacceptPlan={(planId) => {
              onSelectSession(planId);
              handleOpenReopenDialog("reset");
            }}
          />

          {/* Main Content - Column layout for session or no-session */}
          <div className="flex flex-col flex-1 overflow-hidden">
            {/* Header - Only shown when session is active */}
            {session && (
            <header
              data-testid="ideation-header"
              className="flex items-center justify-between h-11 px-4 border-b"
              style={{
                borderColor: "hsla(220 10% 100% / 0.06)",
                background: "hsla(220 10% 12% / 0.85)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
              }}
            >
              <div className="flex items-center gap-2">
                {/* Parent session breadcrumb */}
                {session.parentSessionId && (() => {
                  const parentSession = sessions.find((s) => s.id === session.parentSessionId);
                  if (parentSession) {
                    return (
                      <button
                        onClick={() => onSelectSession(session.parentSessionId!)}
                        className="flex items-center gap-1.5 px-2 py-1 rounded-md transition-colors duration-150"
                        style={{
                          background: "hsla(220 10% 100% / 0.04)",
                          border: "1px solid hsla(220 10% 100% / 0.08)",
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.background = "hsla(220 10% 100% / 0.08)";
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
                        }}
                      >
                        <ArrowLeft className="w-3 h-3" style={{ color: "hsl(220 10% 60%)" }} />
                        <span className="text-[11px]" style={{ color: "hsl(220 10% 70%)" }}>
                          {parentSession.title || "Untitled"}
                        </span>
                      </button>
                    );
                  }
                  return null;
                })()}
                <div
                  className="w-6 h-6 rounded-md flex items-center justify-center"
                  style={{
                    background: "hsla(14 100% 60% / 0.1)",
                    border: "1px solid hsla(14 100% 60% / 0.2)",
                  }}
                >
                  <Sparkles className="w-3 h-3" style={{ color: "hsl(14 100% 60%)" }} />
                </div>
                <div>
                  <h1
                    className="text-xs font-semibold tracking-tight"
                    style={{ color: "hsl(220 10% 90%)" }}
                  >
                    {session.title || "New Session"}
                  </h1>
                  <p
                    className="text-[10px]"
                    style={{ color: "hsl(220 10% 50%)" }}
                  >
                    {proposals.length} {proposals.length === 1 ? "proposal" : "proposals"}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-1.5">
                {canReopen && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleOpenReopenDialog("reopen")}
                    className="h-7 gap-1.5 text-xs transition-colors duration-150"
                    style={{ color: "hsl(220 10% 60%)" }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 90%)";
                      e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 60%)";
                      e.currentTarget.style.background = "transparent";
                    }}
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    Reopen
                  </Button>
                )}
                {canReopen && canResetReaccept && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleOpenReopenDialog("reset")}
                    className="h-7 gap-1.5 text-xs transition-colors duration-150"
                    style={{ color: "hsl(220 10% 60%)" }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 90%)";
                      e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 60%)";
                      e.currentTarget.style.background = "transparent";
                    }}
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                    Reset & Re-accept
                  </Button>
                )}
                {!isReadOnly && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleArchive}
                    className="h-7 gap-1.5 text-xs transition-colors duration-150"
                    style={{ color: "hsl(220 10% 60%)" }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 90%)";
                      e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = "hsl(220 10% 60%)";
                      e.currentTarget.style.background = "transparent";
                    }}
                  >
                    <Archive className="w-3.5 h-3.5" />
                    Archive
                  </Button>
                )}
              </div>
            </header>
          )}

          {/* Split Layout - Left section with footer support, Conversation right (matching Kanban pattern) */}
          <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
            {/* Left Section - Column layout with proposals content and optional footer */}
            <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
              {/* Main Content Area - Session or No-Session */}
              <div
                data-testid="proposals-panel"
                className="flex flex-col relative flex-1 min-h-0"
                style={{
                  minWidth: "360px",
                  borderRight: "1px solid hsla(220 10% 100% / 0.06)",
                  background: "hsl(220 10% 8%)",
                }}
                {...(session ? dropProps : {})}
              >
                {/* Drop zone overlay - shown during drag (active session only) */}
                {session && <DropZoneOverlay isVisible={isDragging} message="Drop to import plan" />}

                {/* No-Session State */}
                {!session && (
                  <StartSessionPanel onNewSession={onNewSession} />
                )}

                {/* Active-Session State */}
                {session && (
                  <>
                    {/* Proposals Toolbar (replaces panel header) */}
                    {proposals.length > 0 && (
                      <ProposalsToolbar
                        proposals={proposals}
                        graph={dependencyGraph}
                        isReadOnly={isReadOnly}
                        onClearAll={handleClearAll}
                        onAcceptPlan={handleAcceptPlan}
                        onAnalyzeDependencies={handleReanalyzeDependencies}
                        isAnalyzingDependencies={isAnalyzingDependencies}
                        analysisTimedOut={analysisTimedOut}
                        session={session}
                      />
                    )}

                    {/* Analysis Banner — shown below toolbar while dependency analysis runs */}
                    {proposals.length > 0 && isAnalyzingDependencies && (
                      <AnalysisBanner />
                    )}

                    {/* Tab bar — only when team artifacts exist */}
                    {teamArtifacts.length > 0 && (
                      <div
                        className="flex items-center gap-0 px-4 shrink-0"
                        style={{
                          height: "36px",
                          borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
                          background: "hsla(220 10% 12% / 0.6)",
                          backdropFilter: "blur(12px)",
                          WebkitBackdropFilter: "blur(12px)",
                        }}
                        data-testid="content-tab-bar"
                      >
                        <button
                          onClick={() => setActiveTab("plan")}
                          className="relative h-full px-3 text-[12px] font-medium transition-colors duration-150"
                          style={{
                            color: activeTab === "plan"
                              ? "hsl(220 10% 90%)"
                              : "hsl(220 10% 50%)",
                          }}
                          data-testid="tab-plan"
                        >
                          Plan & Proposals
                          {activeTab === "plan" && (
                            <span
                              className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                              style={{ background: "hsl(14 100% 60%)" }}
                            />
                          )}
                        </button>
                        <button
                          onClick={() => setActiveTab("research")}
                          className="relative h-full px-3 text-[12px] font-medium transition-colors duration-150 flex items-center gap-1.5"
                          style={{
                            color: activeTab === "research"
                              ? "hsl(220 10% 90%)"
                              : "hsl(220 10% 50%)",
                          }}
                          data-testid="tab-research"
                        >
                          Team Research
                          <span
                            className="text-[10px] font-semibold px-1.5 py-0.5 rounded-full"
                            style={{
                              background: activeTab === "research"
                                ? "hsla(14 100% 60% / 0.15)"
                                : "hsla(220 10% 100% / 0.06)",
                              color: activeTab === "research"
                                ? "hsl(14 100% 65%)"
                                : "hsl(220 10% 50%)",
                            }}
                          >
                            {teamArtifacts.length}
                          </span>
                          {activeTab === "research" && (
                            <span
                              className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                              style={{ background: "hsl(14 100% 60%)" }}
                            />
                          )}
                        </button>
                      </div>
                    )}

                    {/* Content area — switches on activeTab */}
                    {activeTab === "plan" ? (
                    <div ref={proposalsScrollRef} className="flex-1 overflow-y-auto p-4">
                      {session.status === "accepted" && (
                        <AcceptedSessionBanner
                          projectId={session.projectId}
                          proposals={proposals}
                          convertedAt={session.convertedAt}
                          onViewWork={handleViewWork}
                        />
                      )}

                      {importStatus && (
                        <div
                          className="mb-4 p-4 rounded-xl"
                          style={{
                            background: importStatus.type === "success"
                              ? "hsla(145 70% 40% / 0.1)"
                              : "hsla(0 70% 50% / 0.1)",
                            border: `1px solid ${importStatus.type === "success"
                              ? "hsla(145 70% 40% / 0.3)"
                              : "hsla(0 70% 50% / 0.3)"}`,
                          }}
                        >
                          <div className="flex items-center justify-between">
                            <p className="text-sm font-medium" style={{ color: "hsl(220 10% 90%)" }}>{importStatus.message}</p>
                            <Button variant="ghost" size="icon" onClick={() => setImportStatus(null)} className="h-7 w-7">×</Button>
                          </div>
                        </div>
                      )}

                      {syncNotification && (
                        <ProactiveSyncNotificationBanner
                          notification={syncNotification}
                          onDismiss={handleDismissSync}
                          onReview={handleReviewSync}
                          onUndo={handleUndoSync}
                        />
                      )}

                      {!planArtifact && proposals.length > 0 && (
                        <Button
                          variant="outline"
                          onClick={handleImportPlan}
                          className="w-full mb-4 gap-2 transition-colors duration-150"
                          style={{
                            border: "1px solid hsla(220 10% 100% / 0.1)",
                            background: "transparent",
                            color: "hsl(220 10% 70%)",
                          }}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.2)";
                            e.currentTarget.style.background = "hsla(220 10% 100% / 0.03)";
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.1)";
                            e.currentTarget.style.background = "transparent";
                          }}
                          data-testid="import-plan-button"
                        >
                          <Upload className="w-4 h-4" />
                          Import Implementation Plan
                        </Button>
                      )}

                      {planArtifact && (
                        <div className="mb-4">
                          <PlanDisplay
                            plan={planArtifact}
                            showApprove={ideationSettings?.requirePlanApproval ?? false}
                            linkedProposalsCount={proposals.filter((p) => p.planArtifactId === planArtifact.id).length}
                            onEdit={() => {}}
                            isExpanded={isPlanExpanded}
                            onExpandedChange={handlePlanExpandedChange}
                            {...(teamMetadata !== undefined && { teamMetadata })}
                            {...(historicalPlanVersion !== null && {
                              requestedVersion: historicalPlanVersion,
                              onVersionViewed: () => setHistoricalPlanVersion(null),
                            })}
                            verificationStatus={session?.verificationStatus ?? "unverified"}
                            verificationInProgress={session?.verificationInProgress ?? false}
                            {...(session?.gapScore != null && { gapScore: session.gapScore })}
                            {...(planVersionBeforeVerification !== undefined && { planVersionBeforeVerification })}
                            {...(verificationData?.currentRound !== undefined && { currentRound: verificationData.currentRound })}
                            {...(verificationData?.maxRounds !== undefined && { maxRounds: verificationData.maxRounds })}
                            {...(verificationData?.convergenceReason !== undefined && { convergenceReason: verificationData.convergenceReason })}
                            onVerifyFirst={handleTriggerVerification}
                            onSkipVerification={handleSkipVerification}
                            onRevertAndSkip={handleRevertAndSkip}
                            onRetryVerification={handleTriggerVerification}
                          />
                        </div>
                      )}

                      {!planArtifact && ideationSettings?.planMode === "required" && proposals.length === 0 && (
                        <div className="flex flex-col items-center justify-center h-full p-8">
                          <div className="relative">
                            <div
                              className="relative p-8 rounded-2xl text-center"
                              style={{
                                background: "hsla(220 10% 14% / 0.6)",
                                border: "1px solid hsla(220 10% 100% / 0.06)",
                              }}
                            >
                              <Loader2 className="w-10 h-10 mx-auto mb-4 animate-spin" style={{ color: "hsl(14 100% 60%)" }} />
                              <p className="font-medium" style={{ color: "hsl(220 10% 70%)" }}>Waiting for implementation plan...</p>
                              <p className="text-sm mt-1" style={{ color: "hsl(220 10% 50%)" }}>The orchestrator will create a plan first</p>
                            </div>
                          </div>
                        </div>
                      )}

                      {proposals.length === 0 && !(!planArtifact && ideationSettings?.planMode === "required") && <ProposalsEmptyState onBrowse={handleImportPlan} />}

                      {proposals.length > 0 && (
                        <TieredProposalList
                          proposals={proposals}
                          dependencyGraph={dependencyGraph}
                          highlightedIds={highlightedProposalIdsWithUpdates}
                          criticalPathIds={criticalPathSet}
                          onEdit={onEditProposal}
                          onRemove={onRemoveProposal}
                          {...(planArtifact?.metadata.version !== undefined && {
                            currentPlanVersion: planArtifact.metadata.version,
                          })}
                          {...(isReadOnly && { isReadOnly })}
                          onNavigateToTask={handleNavigateToTask}
                          onViewHistoricalPlan={handleViewHistoricalPlan}
                          {...(onViewProposal !== undefined && { onViewDetail: onViewProposal })}
                          {...(selectedProposalId != null && { selectedProposalId })}
                        />
                      )}
                    </div>
                    ) : (
                    <div className="flex-1 overflow-y-auto p-4">
                      <TeamResearchView artifacts={teamArtifacts} sessionId={session.id} />
                    </div>
                    )}
                  </>
                )}
              </div>

              {/* Footer Region - renders footer when provided */}
              {footer && (
                <div className="flex-shrink-0" data-testid="ideation-footer">
                  {footer}
                </div>
              )}
            </div>

            {/* Resize Handle - only when session active */}
            {session && (
            <ResizeHandle
              isResizing={isResizing}
              onMouseDown={handleResizeStart}
              testId="ideation-resize-handle"
            />
            )}

            {/* Conversation Panel (Right) - Only shown when session is active */}
            {session && (
            <div
              data-testid="conversation-panel"
              className="flex flex-col shrink-0"
              style={{ width: `${chatPanelWidth}px` }}
            >
              <IntegratedChatPanel
                key={session.id}
                projectId={session.projectId}
                ideationSessionId={session.id}
                emptyState={<ConversationEmptyState />}
                showHelperTextAlways={true}
                headerContent={
                  <div className="flex items-center gap-2 min-w-0 flex-1">
                    <MessageSquare className="w-3.5 h-3.5 shrink-0" style={{ color: "hsl(220 10% 50%)" }} />
                    <span className="text-[13px] font-medium" style={{ color: "hsl(220 10% 90%)" }}>Conversation</span>
                  </div>
                }
              />
            </div>
            )}
          </div>
          </div>
        </div>

        <input
          ref={fileInputRef}
          type="file"
          accept=".md"
          onChange={handleFileSelected}
          className="hidden"
          data-testid="plan-import-file-input"
        />
      </div>

      <ReopenSessionDialog
        open={reopenDialogOpen}
        onOpenChange={setReopenDialogOpen}
        mode={reopenDialogMode}
        sessionTitle={session?.title || "Untitled"}
        taskCount={sessionTaskCount}
        onConfirm={handleConfirmReopen}
        isLoading={reopenMutation.isPending || resetMutation.isPending}
      />

      {/* Accept Plan Confirmation Modal */}
      {session && dependencyGraph && (
        <AcceptModal
          isOpen={isAcceptModalOpen}
          proposals={proposals}
          dependencyGraph={dependencyGraph}
          sessionId={session.id}
          onAccept={handleAcceptConfirm}
          onCancel={handleAcceptCancel}
          isAnalyzingDependencies={isAnalyzingDependencies}
          defaultUseFeatureBranch={activeProject?.useFeatureBranches ?? false}
          session={session}
        />
      )}
    </>
  );
}

// Backward compatibility alias
export { PlanningView as IdeationView };
