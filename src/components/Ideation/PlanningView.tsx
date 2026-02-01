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
import {
  MessageSquare,
  Archive,
  Loader2,
  Upload,
  Sparkles,
} from "lucide-react";
import { useEventBus } from "@/providers/EventProvider";
import { toast } from "sonner";
import type {
  IdeationSession,
  TaskProposal,
  ApplyProposalsInput,
} from "@/types/ideation";
import { Button } from "@/components/ui/button";
import { PlanDisplay } from "./PlanDisplay";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProposalStore } from "@/stores/proposalStore";
import { IntegratedChatPanel } from "@/components/Chat/IntegratedChatPanel";
import { ConversationEmptyState } from "./EmptyStates";
import { animationStyles } from "./PlanningView.constants";
import { PlanBrowser } from "./PlanBrowser";
import { StartSessionPanel } from "./StartSessionPanel";
import { ProposalsToolbar } from "./ProposalsToolbar";
import { TieredProposalList } from "./TieredProposalList";
import { ProactiveSyncNotificationBanner } from "./ProactiveSyncNotification";
import { ProposalsEmptyState } from "./ProposalsEmptyState";
import { useIdeationHandlers } from "./useIdeationHandlers";
import { useFileDrop } from "@/hooks/useFileDrop";
import { useDependencyGraph } from "@/hooks/useDependencyGraph";
import { DropZoneOverlay } from "./DropZoneOverlay";
import { ideationApi } from "@/api/ideation";

// ============================================================================
// Types
// ============================================================================

interface PlanningViewProps {
  session: IdeationSession | null;
  sessions: IdeationSession[];
  proposals: TaskProposal[];
  onNewSession: () => void;
  onSelectSession: (sessionId: string) => void;
  onArchiveSession: (sessionId: string) => void;
  onDeleteSession?: (sessionId: string) => void;
  onSelectProposal: (proposalId: string) => void;
  onEditProposal: (proposalId: string) => void;
  onRemoveProposal: (proposalId: string) => void;
  onReorderProposals: (proposalIds: string[]) => void;
  onApply: (options: ApplyProposalsInput) => void;
}

// Empty States extracted to separate files

// Plan Browser extracted to PlanBrowser.tsx

// Start Session Panel extracted to StartSessionPanel.tsx

// Proposal Card extracted to ProposalCard.tsx

// Proactive Sync Notification extracted to ProactiveSyncNotification.tsx

// Proposals Toolbar extracted to ProposalsToolbar.tsx

// ============================================================================
// Main Component
// ============================================================================

export function PlanningView({
  session,
  sessions,
  proposals,
  onNewSession,
  onSelectSession,
  onArchiveSession,
  onDeleteSession,
  onSelectProposal,
  onEditProposal,
  onRemoveProposal,
  onReorderProposals,
  onApply,
}: PlanningViewProps) {
  const [leftPanelWidth, setLeftPanelWidth] = useState(60); // 60/40 split like Kanban
  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const proposalsScrollRef = useRef<HTMLDivElement>(null);

  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const ideationSettings = useIdeationStore((state) => state.ideationSettings);
  const fetchPlanArtifact = useIdeationStore((state) => state.fetchPlanArtifact);
  const showSyncNotification = useIdeationStore((state) => state.showSyncNotification);
  const syncNotification = useIdeationStore((state) => state.syncNotification);
  const dismissSyncNotification = useIdeationStore((state) => state.dismissSyncNotification);

  // Fetch dependency graph for the session
  const { data: dependencyGraph } = useDependencyGraph(session?.id ?? "");

  // Build critical path set from the graph (TieredProposalList handles other computations)
  const criticalPathSet = useMemo(() => {
    if (!dependencyGraph) {
      return new Set<string>();
    }
    return new Set(dependencyGraph.criticalPath);
  }, [dependencyGraph]);

  // Dependency analysis loading state
  const [isAnalyzingDependencies, setIsAnalyzingDependencies] = useState(false);

  // Read-only mode: plans that are not active are read-only
  const isReadOnly = session?.status !== "active";

  // Get the event bus from context (TauriEventBus or MockEventBus)
  const eventBus = useEventBus();

  // Listen for dependency analysis events
  useEffect(() => {
    const sessionId = session?.id;
    if (!sessionId) return;

    // Listen for analysis started
    const unsubAnalysisStarted = eventBus.subscribe<{ session_id: string }>(
      "dependencies:analysis_started",
      (payload) => {
        if (payload.session_id === sessionId) {
          setIsAnalyzingDependencies(true);
        }
      }
    );

    // Listen for suggestions applied
    const unsubSuggestionsApplied = eventBus.subscribe<{ session_id: string; applied_count: number }>(
      "dependencies:suggestions_applied",
      (payload) => {
        if (payload.session_id === sessionId) {
          setIsAnalyzingDependencies(false);
          const count = payload.applied_count;
          if (count > 0) {
            toast.success(`${count} ${count === 1 ? "dependency" : "dependencies"} added`);
          } else {
            toast.info("No new dependencies found");
          }
        }
      }
    );

    return () => {
      unsubAnalysisStarted();
      unsubSuggestionsApplied();
    };
  }, [eventBus, session?.id]);

  // Manual re-trigger dependency analysis
  const handleReanalyzeDependencies = useCallback(async () => {
    if (!session || isAnalyzingDependencies || proposals.length < 2) return;
    try {
      await ideationApi.sessions.spawnDependencySuggester(session.id);
    } catch (err) {
      console.error("Failed to spawn dependency suggester:", err);
      toast.error("Failed to analyze dependencies");
    }
  }, [session, isAnalyzingDependencies, proposals.length]);

  useEffect(() => {
    if (session?.planArtifactId) {
      fetchPlanArtifact(session.planArtifactId);
    }
  }, [session?.planArtifactId, fetchPlanArtifact]);

  useEffect(() => {
    const unsubProposalsUpdate = eventBus.subscribe<{ artifact_id: string; proposal_ids: string[] }>(
      "plan:proposals_may_need_update",
      (payload) => {
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
  }, [eventBus, proposals, showSyncNotification]);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = ((e.clientX - rect.left) / rect.width) * 100;
      setLeftPanelWidth(Math.max(30, Math.min(70, newWidth)));
    };

    const handleMouseUp = () => setIsResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  // Accept Plan handler - accepts ALL proposals (no selection)
  const handleAcceptPlan = useCallback((targetColumn: string) => {
    if (!session) return;
    onApply({
      sessionId: session.id,
      proposalIds: proposals.map((p) => p.id),
      targetColumn,
      preserveDependencies: true,
    });
  }, [session, proposals, onApply]);

  const {
    highlightedProposalIds,
    isPlanExpanded,
    setIsPlanExpanded,
    importStatus,
    setImportStatus,
    fileInputRef,
    handleArchive,
    handleSelectProposal,
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
    onSelectProposal,
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
    if (planArtifact && proposals.length === 0 && !isPlanExpanded) {
      setIsPlanExpanded(true);
    }
  }, [planArtifact, proposals.length, isPlanExpanded, setIsPlanExpanded]);

  // Auto-collapse plan when new proposal arrives
  const lastProposalAddedAt = useProposalStore((state) => state.lastProposalAddedAt);
  useEffect(() => {
    if (lastProposalAddedAt !== null && isPlanExpanded) {
      setIsPlanExpanded(false);
    }
  }, [lastProposalAddedAt, isPlanExpanded, setIsPlanExpanded]);

  // Auto-scroll to bottom when new proposals arrive
  const proposalCount = proposals.length;
  useLayoutEffect(() => {
    if (proposalCount > 0 && proposalsScrollRef.current) {
      proposalsScrollRef.current.scrollTo({
        top: proposalsScrollRef.current.scrollHeight,
        behavior: "smooth",
      });
    }
  }, [proposalCount]);

  const activePlans = useMemo(() => sessions.filter((s) => s.status === "active"), [sessions]);
  const historyPlans = useMemo(() => sessions.filter((s) => s.status !== "active"), [sessions]);

  return (
    <>
      <style>{animationStyles}</style>
      <div
        ref={containerRef}
        data-testid="ideation-view"
        className="flex h-full relative"
        style={{ background: "hsl(220 10% 8%)" }}
        role="main"
      >
        {/* Plan Browser Sidebar */}
        <PlanBrowser
          plans={activePlans}
          historyPlans={historyPlans}
          currentPlanId={session?.id ?? null}
          onSelectPlan={onSelectSession}
          onNewPlan={onNewSession}
          {...(onDeleteSession !== undefined && { onDeletePlan: onDeleteSession })}
        />

        {/* Main Content */}
        {!session ? (
          <StartSessionPanel onNewSession={onNewSession} />
        ) : (
          <div className="flex flex-col flex-1 overflow-hidden">
            {/* Header */}
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
            </header>

            {/* Split Layout - Proposals left, Conversation right (matching Kanban pattern) */}
            <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
              {/* Proposals Panel (Left) */}
              <div
                data-testid="proposals-panel"
                className="flex flex-col relative"
                style={{
                  width: `${leftPanelWidth}%`,
                  minWidth: "360px",
                  borderRight: "1px solid hsla(220 10% 100% / 0.06)",
                  background: "hsl(220 10% 8%)",
                }}
                {...dropProps}
              >
                {/* Drop zone overlay - shown during drag */}
                <DropZoneOverlay isVisible={isDragging} message="Drop to import plan" />

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
                  />
                )}

                {/* Proposals List */}
                <div ref={proposalsScrollRef} className="flex-1 overflow-y-auto p-4">
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
                        onExpandedChange={setIsPlanExpanded}
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
                      highlightedIds={highlightedProposalIds}
                      criticalPathIds={criticalPathSet}
                      onSelect={handleSelectProposal}
                      onEdit={onEditProposal}
                      onRemove={onRemoveProposal}
                      {...(planArtifact?.metadata.version !== undefined && {
                        currentPlanVersion: planArtifact.metadata.version,
                      })}
                    />
                  )}
                </div>

              </div>

              {/* Resize Handle */}
              <div
                data-testid="resize-handle"
                className="w-1 cursor-ew-resize relative group shrink-0"
                style={{
                  background: isResizing ? "hsla(14 100% 60% / 0.5)" : "transparent",
                }}
                onMouseDown={handleResizeStart}
              >
                <div
                  className="absolute top-0 bottom-0 left-1/2 -translate-x-1/2 w-px transition-all duration-150"
                  style={{
                    background: isResizing ? "hsl(14 100% 60%)" : "hsla(220 10% 100% / 0.06)",
                  }}
                />
              </div>

              {/* Conversation Panel (Right) - Using IntegratedChatPanel */}
              <div
                data-testid="conversation-panel"
                className="flex flex-col flex-1"
                style={{ minWidth: "320px" }}
              >
                <IntegratedChatPanel
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
            </div>
          </div>
        )}

        <input
          ref={fileInputRef}
          type="file"
          accept=".md"
          onChange={handleFileSelected}
          className="hidden"
          data-testid="plan-import-file-input"
        />
      </div>
    </>
  );
}

// Backward compatibility alias
export { PlanningView as IdeationView };
