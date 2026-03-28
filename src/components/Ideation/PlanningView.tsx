/**
 * PlanningView - Premium Planning Interface
 *
 * Design: macOS Tahoe Liquid Glass
 * - Flat translucent surfaces with backdrop-blur
 * - Subtle borders and single shadows
 * - Warm ambient orange glow in backgrounds
 * - Clean, minimal aesthetic
 */

import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import {
  MessageSquare,
  Archive,
  Loader2,
  Sparkles,
  RotateCcw,
  RefreshCw,
  ArrowLeft,
  CheckCircle,
  AlertTriangle,
  ShieldCheck,
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
import { ConversationEmptyState, WaitingForCapacityState } from "./EmptyStates";
import { useChildSessionStatus } from "@/hooks/useChildSessionStatus";
import { animationStyles } from "./PlanningView.constants";
import { PlanBrowser } from "./PlanBrowser";
import { StartSessionPanel } from "./StartSessionPanel";
import type { TeamMetadata } from "./PlanDisplay";
import type { ProposalDetailEnrichment } from "./ProposalDetailSheet";
import { useIdeationHandlers } from "./useIdeationHandlers";
import { useFileDrop } from "@/hooks/useFileDrop";
import { useDependencyGraph } from "@/hooks/useDependencyGraph";
import { DropZoneOverlay } from "./DropZoneOverlay";
import { ReopenSessionDialog } from "./ReopenSessionDialog";
import type { ReopenMode } from "./ReopenSessionDialog";
import { useReopenSession, useResetAndReaccept, useIdeationSessions } from "@/hooks/useIdeation";
import { ideationApi } from "@/api/ideation";
import { useQuery } from "@tanstack/react-query";
import { planBranchApi } from "@/api/plan-branch";
import { PlanTabContent } from "./PlanTabContent";
import { ProposalsTabContent } from "./ProposalsTabContent";
import { TeamResearchTabContent } from "./TeamResearchTabContent";
import { VerificationPanel } from "./VerificationPanel";

// ============================================================================
// Types
// ============================================================================

interface PlanningViewProps {
  session: IdeationSession | null;
  proposals: TaskProposal[];
  isSessionLoading?: boolean;
  onNewSession: () => void;
  onSelectSession: (sessionId: string) => void;
  onArchiveSession: (sessionId: string) => void;
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
  proposals,
  isSessionLoading = false,
  onNewSession,
  onSelectSession,
  onArchiveSession,
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

  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const fetchPlanArtifact = useIdeationStore((state) => state.fetchPlanArtifact);
  const showSyncNotification = useIdeationStore((state) => state.showSyncNotification);
  const dismissSyncNotification = useIdeationStore((state) => state.dismissSyncNotification);
  const syncNotification = useIdeationStore((state) => state.syncNotification);

  // Fetch dependency graph for the session
  const { data: dependencyGraph, isFetching: isDependencyUpdating } = useDependencyGraph(session?.id ?? "");

  // Build critical path set from the graph
  const criticalPathSet = useMemo(() => {
    if (!dependencyGraph) return new Set<string>();
    return new Set(dependencyGraph.criticalPath);
  }, [dependencyGraph]);

  const lastDependencyFetchRef = useRef<boolean>(false);
  const lastDependencyToastAtRef = useRef<number | null>(null);
  const lastDependencyRefreshRequestedAt = useProposalStore((state) => state.lastDependencyRefreshRequestedAt);

  // Read-only mode: plans that are not active are read-only
  const isReadOnly = session?.status !== "active";

  // Reopen/Reset dialog state
  const [reopenDialogOpen, setReopenDialogOpen] = useState(false);
  const [reopenDialogMode, setReopenDialogMode] = useState<ReopenMode>("reopen");
  const reopenMutation = useReopenSession();
  const resetMutation = useResetAndReaccept();

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

  // Sessions list for breadcrumb parent resolution
  const projectIdForSessions = activeProjectId || session?.projectId || "";
  const { data: allSessionsForBreadcrumb = [] } = useIdeationSessions(projectIdForSessions);

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

  // Stable ref for fetchPlanArtifact (must stay in PlanningView — always mounted)
  const fetchPlanArtifactRef = useRef(fetchPlanArtifact);
  useEffect(() => { fetchPlanArtifactRef.current = fetchPlanArtifact; }, [fetchPlanArtifact]);

  const planArtifactId = planArtifact?.id ?? null;
  useEffect(() => {
    if (!session?.planArtifactId) return;
    if (planArtifactId !== session.planArtifactId) {
      fetchPlanArtifactRef.current(session.planArtifactId);
    }
  }, [session?.planArtifactId, planArtifactId]);

  // Fetch plan branch to show preserved branch config in reopen dialog
  const { data: planBranch } = useQuery({
    queryKey: ["plan-branch", planArtifactId],
    queryFn: () => planBranchApi.getByPlan(planArtifactId!),
    enabled: planArtifactId != null,
  });

  // Fetch team artifact summaries for team-ideated sessions
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

  // Tab state managed in Zustand (enables cross-component tab switching from notifications)
  const setActiveIdeationTab = useIdeationStore((s) => s.setActiveIdeationTab);
  const setActiveVerificationChildId = useIdeationStore((s) => s.setActiveVerificationChildId);
  const setLastVerificationChildId = useIdeationStore((s) => s.setLastVerificationChildId);
  const activeTab = useIdeationStore(
    (s) => s.activeIdeationTab[session?.id ?? ''] ?? 'plan'
  );
  const activeVerificationChildId = useIdeationStore(
    (s) => s.activeVerificationChildId[session?.id ?? ''] ?? null
  );
  const lastVerificationChildId = useIdeationStore(
    (s) => s.lastVerificationChildId[session?.id ?? ''] ?? null
  );

  // Poll status for verification child and direct child session views to detect pending_initial_prompt
  const { data: verificationChildStatus } = useChildSessionStatus(lastVerificationChildId);
  // Only poll for the current session when it's a child (has parentSessionId) to detect pending prompt
  const { data: currentSessionStatus } = useChildSessionStatus(
    session?.parentSessionId ? session.id : null
  );

  // Eagerly fetch verification child sessions so lastVerificationChildId is populated
  // before the user clicks the Verification tab (eliminates cold-start flash of parent chat)
  const { data: verificationChildren } = useQuery({
    queryKey: ["childSessions", session?.id, "verification"],
    queryFn: () => ideationApi.sessions.getChildren(session!.id, "verification"),
    enabled: !!session?.id && session?.sessionPurpose !== "verification",
    staleTime: 30_000,
  });

  // Pre-populate lastVerificationChildId from eager query result.
  // Only sets if store field is null (avoids overwriting event-driven updates).
  useEffect(() => {
    if (!session?.id || !verificationChildren?.length) return;
    if (lastVerificationChildId) return;

    const sorted = [...verificationChildren].sort(
      (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    );
    const latestId = sorted[0]?.id;
    if (latestId) {
      setLastVerificationChildId(session.id, latestId);
    }
  }, [session?.id, verificationChildren, lastVerificationChildId, setLastVerificationChildId]);

  // Reset to plan tab when switching sessions
  const prevSessionIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (!session?.id) return;
    if (prevSessionIdRef.current !== null && prevSessionIdRef.current !== session.id) {
      // Session changed — reset new session to plan tab
      setActiveIdeationTab(session.id, 'plan');
      setActiveVerificationChildId(session.id, null);
      setLastVerificationChildId(session.id, null);
    }
    prevSessionIdRef.current = session.id;
  }, [session?.id, setActiveIdeationTab, setActiveVerificationChildId, setLastVerificationChildId]);

  const isVerificationTabActive = activeTab === 'verification';

  // Construct TeamMetadata when session is a team session
  const teamMetadata = useMemo<TeamMetadata | undefined>(() => {
    if (!session?.teamMode || session.teamMode === "solo") return undefined;
    return {
      teamIdeated: true,
      teamMode: session.teamMode as "research" | "debate",
      teammateCount: session.teamConfig?.maxTeammates ?? teamArtifacts.length,
      findings: [],
    };
  }, [session?.teamMode, session?.teamConfig?.maxTeammates, teamArtifacts.length]);

  // Verification tab visibility + badge state
  const verificationStatus = session?.verificationStatus ?? "unverified";
  const showVerificationTab = Boolean(
    verificationStatus !== "unverified" || planArtifact
  );
  const verificationBadge: "in_progress" | "verified" | "warning" | null = (() => {
    if (!session) return null;
    if (session.verificationInProgress) return "in_progress";
    if (verificationStatus === "verified" || verificationStatus === "imported_verified") return "verified";
    if (verificationStatus === "needs_revision") return "warning";
    return null;
  })();

  // Subscribe to plan:proposals_may_need_update — must stay mounted (not inside a conditional tab)
  useEffect(() => {
    const unsubProposalsUpdate = eventBus.subscribe<{ artifact_id: string; proposal_ids: string[]; session_id?: string }>(
      "plan:proposals_may_need_update",
      (payload) => {
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

  // Navigate to task handler
  const setCurrentView = useUiStore((state) => state.setCurrentView);
  const setSelectedTaskId = useUiStore((state) => state.setSelectedTaskId);

  // Get active project for feature branch setting
  const activeProject = useProjectStore(selectActiveProject);

  // Accept Plan — opens the confirmation modal
  const handleAcceptPlan = useCallback(() => {
    setIsAcceptModalOpen(true);
  }, []);

  // Handle confirmed accept from modal
  const handleAcceptConfirm = useCallback(async (options: ApplyProposalsInput) => {
    if (!session) return;
    const projectId = activeProjectId || session.projectId;
    if (!projectId) return;

    let executionPlanId: string | null | undefined;
    try {
      const applyResult = await onApply(options);
      if (!applyResult?.sessionConverted) {
        setIsAcceptModalOpen(false);
        return;
      }
      executionPlanId = applyResult?.executionPlanId;
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to apply proposals";
      toast.error(message);
      return;
    }

    try {
      await setActivePlan(projectId, session.id, "ideation", executionPlanId);
    } catch (error) {
      console.error("Failed to set active plan:", error);
      toast.error("Failed to set active plan");
    }

    setIsAcceptModalOpen(false);
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
  const autoOpenedPlanRef = useRef(false);
  const userOverrideRef = useRef(false);

  useEffect(() => {
    if (userOverrideRef.current) return;
    if (isSessionLoading) return;
    if (planArtifact && proposals.length === 0 && !isPlanExpanded) {
      autoOpenedPlanRef.current = true;
      setIsPlanExpanded(true);
    }
  }, [planArtifact, proposals.length, isPlanExpanded, setIsPlanExpanded, isSessionLoading]);

  // Switch to Proposals tab when new proposal arrives
  const lastProposalAddedAt = useProposalStore((state) => state.lastProposalAddedAt);
  const prevProposalAddedAtRef = useRef(lastProposalAddedAt);
  useEffect(() => {
    const changed = lastProposalAddedAt !== prevProposalAddedAtRef.current;
    prevProposalAddedAtRef.current = lastProposalAddedAt;
    if (!changed) return;
    if (userOverrideRef.current) return;
    if (lastProposalAddedAt !== null && isPlanExpanded && session) {
      autoOpenedPlanRef.current = false;
      setActiveIdeationTab(session.id, 'proposals');
    }
  }, [lastProposalAddedAt, isPlanExpanded, session, setActiveIdeationTab]);

  // If proposals load after an auto-open, switch to Proposals tab
  useEffect(() => {
    if (userOverrideRef.current) return;
    if (isSessionLoading) return;
    if (proposals.length > 0 && isPlanExpanded && autoOpenedPlanRef.current && session) {
      autoOpenedPlanRef.current = false;
      setActiveIdeationTab(session.id, 'proposals');
    }
  }, [proposals.length, isPlanExpanded, isSessionLoading, session, setActiveIdeationTab]);

  // Reset plan expansion when switching sessions
  const lastSessionIdRef = useRef<string | null>(null);
  useEffect(() => {
    if (!session?.id) return;
    if (lastSessionIdRef.current !== session.id) {
      lastSessionIdRef.current = session.id;
      autoOpenedPlanRef.current = false;
      userOverrideRef.current = false;
      setIsPlanExpanded(!!session.planArtifactId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session?.id, session?.planArtifactId]);

  const handlePlanExpandedChange = useCallback((expanded: boolean) => {
    autoOpenedPlanRef.current = false;
    userOverrideRef.current = true;
    setIsPlanExpanded(expanded);
  }, [setIsPlanExpanded]);

  // Historical plan version — set when user clicks "View plan as of proposal creation (vX)"
  const [requestedHistoricalVersion, setRequestedHistoricalVersion] = useState<number | null>(null);

  const handleViewHistoricalPlan = useCallback((_artifactId: string, version: number) => {
    setRequestedHistoricalVersion(version);
    if (session) setActiveIdeationTab(session.id, 'plan');
    userOverrideRef.current = true;
    setIsPlanExpanded(true);
  }, [session, setActiveIdeationTab, setIsPlanExpanded]);

  const handleTabChange = useCallback((tab: 'plan' | 'proposals' | 'research') => {
    if (!session) return;
    setActiveIdeationTab(session.id, tab);
  }, [session, setActiveIdeationTab]);

  // Fall back to plan tab if proposals tab is active but all proposals are deleted
  useEffect(() => {
    if (activeTab === 'proposals' && proposals.length === 0 && session) {
      handleTabChange('plan');
    }
  }, [proposals.length, activeTab, session, handleTabChange]);

  const handleVerificationTabClick = useCallback(async () => {
    if (!session) return;
    setActiveIdeationTab(session.id, 'verification');

    // If child ID already preloaded (set by event handler), skip async fetch
    if (activeVerificationChildId || lastVerificationChildId) return;

    // Fetch the latest verification child session
    try {
      const children = await ideationApi.sessions.getChildren(session.id, 'verification');
      if (children.length > 0) {
        // Sort by createdAt descending, take the most recent
        const sorted = [...children].sort(
          (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
        );
        const latest = sorted[0];
        if (latest) {
          setActiveVerificationChildId(session.id, latest.id);
          setLastVerificationChildId(session.id, latest.id);
        }
      }
    } catch (err) {
      console.error('Verification tab: failed to fetch child sessions', err);
      // Tab switches regardless — child panel stays hidden until child exists
    }
  }, [session, activeVerificationChildId, lastVerificationChildId, setActiveIdeationTab, setActiveVerificationChildId, setLastVerificationChildId]);

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
            projectId={activeProjectId || session?.projectId || ""}
            currentPlanId={session?.id ?? null}
            onSelectPlan={onSelectSession}
            onNewPlan={onNewSession}
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
                  const parentSession = allSessionsForBreadcrumb.find((s) => s.id === session.parentSessionId);
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

          {/* Split Layout - Left section with footer support, Conversation right */}
          <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
            {/* Left Section - Column layout with tab content and optional footer */}
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
                    {/* Tab bar — Plan | Proposals | Team Research (conditional) */}
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
                        onClick={() => handleTabChange('plan')}
                        className="relative h-full px-3 text-[12px] font-medium transition-colors duration-150"
                        style={{
                          color: activeTab === "plan"
                            ? "hsl(220 10% 90%)"
                            : "hsl(220 10% 50%)",
                        }}
                        data-testid="tab-plan"
                      >
                        Plan
                        {activeTab === "plan" && (
                          <span
                            className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                            style={{ background: "hsl(14 100% 60%)" }}
                          />
                        )}
                      </button>
                      {showVerificationTab && (
                        <button
                          onClick={handleVerificationTabClick}
                          className="relative h-full px-3 text-[12px] font-medium transition-colors duration-150 flex items-center gap-1.5"
                          style={{
                            color: activeTab === "verification"
                              ? "hsl(220 10% 90%)"
                              : "hsl(220 10% 50%)",
                          }}
                          data-testid="tab-verification"
                        >
                          Verification
                          {verificationBadge === "in_progress" && (
                            <span
                              data-testid="verification-badge-in-progress"
                              className="w-2 h-2 rounded-full animate-pulse shrink-0"
                              style={{ background: "hsl(38 100% 60%)" }}
                            />
                          )}
                          {verificationBadge === "verified" && (
                            <CheckCircle
                              data-testid="verification-badge-verified"
                              className="w-3 h-3 shrink-0"
                              style={{ color: "hsl(142 71% 45%)" }}
                            />
                          )}
                          {verificationBadge === "warning" && (
                            <AlertTriangle
                              data-testid="verification-badge-warning"
                              className="w-3 h-3 shrink-0"
                              style={{ color: "hsl(38 100% 60%)" }}
                            />
                          )}
                          {activeTab === "verification" && (
                            <span
                              className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                              style={{ background: "hsl(14 100% 60%)" }}
                            />
                          )}
                        </button>
                      )}
                      {proposals.length > 0 && (
                        <button
                          onClick={() => handleTabChange('proposals')}
                          className="relative h-full px-3 text-[12px] font-medium transition-colors duration-150 flex items-center gap-1.5"
                          style={{
                            color: activeTab === "proposals"
                              ? "hsl(220 10% 90%)"
                              : "hsl(220 10% 50%)",
                          }}
                          data-testid="tab-proposals"
                        >
                          Proposals
                          <span
                            className="text-[10px] font-semibold px-1.5 py-0.5 rounded-full"
                            style={{
                              background: activeTab === "proposals"
                                ? "hsla(14 100% 60% / 0.15)"
                                : "hsla(220 10% 100% / 0.06)",
                              color: activeTab === "proposals"
                                ? "hsl(14 100% 65%)"
                                : "hsl(220 10% 50%)",
                            }}
                          >
                            {proposals.length}
                          </span>
                          {activeTab === "proposals" && (
                            <span
                              className="absolute bottom-0 left-3 right-3 h-[2px] rounded-full"
                              style={{ background: "hsl(14 100% 60%)" }}
                            />
                          )}
                        </button>
                      )}
                      {teamArtifacts.length > 0 && (
                        <button
                          onClick={() => handleTabChange('research')}
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
                      )}
                    </div>

                    {/* Tab content — delegates to extracted components */}
                    {activeTab === "plan" && (
                      <PlanTabContent
                        session={session}
                        proposals={proposals}
                        {...(teamMetadata !== undefined && { teamMetadata })}
                        importStatus={importStatus}
                        onImportStatusChange={setImportStatus}
                        onImportPlan={handleImportPlan}
                        onViewWork={handleViewWork}
                        isPlanExpanded={isPlanExpanded}
                        onExpandedChange={handlePlanExpandedChange}
                        requestedHistoricalVersion={requestedHistoricalVersion}
                        onHistoricalVersionViewed={() => setRequestedHistoricalVersion(null)}
                      />
                    )}

                    {activeTab === "verification" && (
                      <div
                        data-testid="verification-tab-content"
                        className="flex flex-col flex-1 min-h-0"
                      >
                        <VerificationPanel session={session} />
                      </div>
                    )}

                    {activeTab === "proposals" && (
                      <ProposalsTabContent
                        session={session}
                        proposals={proposals}
                        dependencyGraph={dependencyGraph}
                        criticalPathSet={criticalPathSet}
                        highlightedIds={highlightedProposalIds}
                        isReadOnly={isReadOnly}
                        onEditProposal={onEditProposal}
                        onNavigateToTask={handleNavigateToTask}
                        onViewHistoricalPlan={handleViewHistoricalPlan}
                        {...(onViewProposal !== undefined && { onViewProposal })}
                        {...(selectedProposalId !== undefined && { selectedProposalId })}
                        onImportPlan={handleImportPlan}
                        onClearAll={handleClearAll}
                        onAcceptPlan={handleAcceptPlan}
                        onReviewSync={handleReviewSync}
                        onUndoSync={handleUndoSync}
                        onDismissSync={handleDismissSync}
                        onDeleteProposal={onRemoveProposal}
                      />
                    )}

                    {activeTab === "research" && (
                      <TeamResearchTabContent
                        teamArtifacts={teamArtifacts}
                        sessionId={session.id}
                      />
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
              {/* Parent chat panel — always mounted, hidden when showing verification child */}
              <div
                className="flex flex-col flex-1"
                style={{ display: (!isVerificationTabActive || !lastVerificationChildId) ? 'flex' : 'none' }}
              >
                <IntegratedChatPanel
                  key={session.id}
                  projectId={session.projectId}
                  ideationSessionId={session.id}
                  emptyState={currentSessionStatus?.pending_initial_prompt ? <WaitingForCapacityState /> : <ConversationEmptyState />}
                  showHelperTextAlways={true}
                  isVisible={!isVerificationTabActive || !lastVerificationChildId}
                  headerContent={
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <MessageSquare className="w-3.5 h-3.5 shrink-0" style={{ color: "hsl(220 10% 50%)" }} />
                      <span className="text-[13px] font-medium" style={{ color: "hsl(220 10% 90%)" }}>Conversation</span>
                    </div>
                  }
                />
              </div>
              {/* Verification child chat panel — mounted only when child session exists */}
              {lastVerificationChildId && (
                <div
                  className="flex flex-col flex-1"
                  style={{
                    display: isVerificationTabActive ? 'flex' : 'none',
                    borderLeft: '2px solid hsla(38 100% 60% / 0.25)',
                  }}
                >
                  <IntegratedChatPanel
                    key={lastVerificationChildId}
                    projectId={session.projectId}
                    ideationSessionId={lastVerificationChildId!}
                    emptyState={verificationChildStatus?.pending_initial_prompt ? <WaitingForCapacityState /> : <ConversationEmptyState />}
                    showHelperTextAlways={true}
                    isVisible={isVerificationTabActive}
                    headerContent={
                      <div className="flex items-center gap-2 min-w-0 flex-1">
                        <ShieldCheck className="w-3.5 h-3.5 shrink-0" style={{ color: "hsl(38 100% 60%)" }} />
                        <div className="flex flex-col min-w-0 flex-1">
                          <span className="text-[11px] font-semibold leading-tight truncate" style={{ color: "hsl(38 100% 65%)" }}>
                            Verification
                          </span>
                          <span className="text-[10px] leading-tight truncate" style={{ color: "hsl(220 10% 50%)" }}>
                            {session.title || "Untitled"}
                          </span>
                        </div>
                        <button
                          onClick={() => setActiveIdeationTab(session.id, 'plan')}
                          className="flex items-center gap-1 px-2 py-1 rounded-md text-[10px] font-medium shrink-0 transition-colors duration-150"
                          style={{
                            color: "hsl(220 10% 55%)",
                            background: "hsla(220 10% 100% / 0.04)",
                            border: "1px solid hsla(220 10% 100% / 0.08)",
                          }}
                          onMouseEnter={(e) => {
                            e.currentTarget.style.background = "hsla(220 10% 100% / 0.08)";
                            e.currentTarget.style.color = "hsl(220 10% 80%)";
                          }}
                          onMouseLeave={(e) => {
                            e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
                            e.currentTarget.style.color = "hsl(220 10% 55%)";
                          }}
                          data-testid="back-to-plan-button"
                          title="Back to plan"
                        >
                          <ArrowLeft className="w-3 h-3" />
                          Plan
                        </button>
                      </div>
                    }
                  />
                </div>
              )}
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
        {...(planBranch?.branchName !== undefined && { featureBranch: planBranch.branchName })}
        {...(planBranch?.baseBranchOverride != null && { targetBranch: planBranch.baseBranchOverride })}
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
          defaultUseFeatureBranch={activeProject?.useFeatureBranches ?? false}
          session={session}
          workingDirectory={activeProject?.workingDirectory}
          baseBranch={activeProject?.baseBranch ?? "main"}
        />
      )}
    </>
  );
}

// Backward compatibility alias
export { PlanningView as IdeationView };
