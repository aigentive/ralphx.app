/**
 * IdeationView - Premium Ideation Interface
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
  ListTodo,
  Archive,
  Loader2,
  Upload,
  Sparkles,
  Network,
} from "lucide-react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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
import { cn } from "@/lib/utils";
import { ConversationEmptyState } from "./EmptyStates";
import { animationStyles } from "./IdeationView.constants";
import { SessionBrowser } from "./SessionBrowser";
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
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

// ============================================================================
// Types
// ============================================================================

interface IdeationViewProps {
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

// Session Browser extracted to SessionBrowser.tsx

// Start Session Panel extracted to StartSessionPanel.tsx

// Proposal Card extracted to ProposalCard.tsx

// Proactive Sync Notification extracted to ProactiveSyncNotification.tsx

// Proposals Toolbar extracted to ProposalsToolbar.tsx

// ============================================================================
// Main Component
// ============================================================================

export function IdeationView({
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
}: IdeationViewProps) {
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

  // Listen for dependency analysis events
  useEffect(() => {
    const sessionId = session?.id;
    if (!sessionId) return;

    const unlistenFns: Promise<UnlistenFn>[] = [];

    // Listen for analysis started
    unlistenFns.push(
      listen<{ session_id: string }>("dependencies:analysis_started", (event) => {
        if (event.payload.session_id === sessionId) {
          setIsAnalyzingDependencies(true);
        }
      })
    );

    // Listen for suggestions applied
    unlistenFns.push(
      listen<{ session_id: string; applied_count: number }>("dependencies:suggestions_applied", (event) => {
        if (event.payload.session_id === sessionId) {
          setIsAnalyzingDependencies(false);
          const count = event.payload.applied_count;
          if (count > 0) {
            toast.success(`${count} ${count === 1 ? "dependency" : "dependencies"} added`);
          } else {
            toast.info("No new dependencies found");
          }
        }
      })
    );

    return () => {
      unlistenFns.forEach((unlisten) => unlisten.then((fn) => fn()));
    };
  }, [session?.id]);

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
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      unlisten = await listen<{ artifact_id: string; proposal_ids: string[] }>(
        "plan:proposals_may_need_update",
        (event) => {
          const affectedProposals = proposals.filter((p) => event.payload.proposal_ids.includes(p.id));
          const previousStates: Record<string, unknown> = {};
          affectedProposals.forEach((p) => { previousStates[p.id] = { ...p }; });

          showSyncNotification({
            artifactId: event.payload.artifact_id,
            proposalIds: event.payload.proposal_ids,
            previousStates,
            timestamp: Date.now(),
          });
        }
      );
    };

    setupListener();
    return () => { if (unlisten) unlisten(); };
  }, [proposals, showSyncNotification]);

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

  const handleApply = useCallback((targetColumn: string) => {
    if (!session) return;
    const selectedProposals = proposals.filter((p) => p.selected);
    onApply({
      sessionId: session.id,
      proposalIds: selectedProposals.map((p) => p.id),
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
    handleSelectAll,
    handleDeselectAll,
    handleSortByPriority,
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

  const selectedCount = proposals.filter((p) => p.selected).length;

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

  const activeSessions = useMemo(() => sessions.filter((s) => s.status === "active"), [sessions]);

  return (
    <>
      <style>{animationStyles}</style>
      <div
        ref={containerRef}
        data-testid="ideation-view"
        className="flex h-full relative bg-[#050505]"
        role="main"
      >
        {/* Session Browser Sidebar */}
        <SessionBrowser
          sessions={activeSessions}
          currentSessionId={session?.id ?? null}
          onSelectSession={onSelectSession}
          onNewSession={onNewSession}
          {...(onDeleteSession !== undefined && { onDeleteSession })}
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
                borderColor: "rgba(255,255,255,0.06)",
                background: "rgba(18,18,18,0.85)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
              }}
            >
              <div className="flex items-center gap-2">
                <div
                  className="w-6 h-6 rounded-md flex items-center justify-center"
                  style={{
                    background: "rgba(255,107,53,0.1)",
                    border: "1px solid rgba(255,107,53,0.2)",
                  }}
                >
                  <Sparkles className="w-3 h-3 text-[#ff6b35]" />
                </div>
                <div>
                  <h1 className="text-xs font-semibold text-[var(--text-primary)] tracking-tight">
                    {session.title || "New Session"}
                  </h1>
                  <p className="text-[10px] text-[var(--text-muted)]">
                    {proposals.length} {proposals.length === 1 ? "proposal" : "proposals"}
                  </p>
                </div>
              </div>
              <Button variant="ghost" size="sm" onClick={handleArchive} className="h-7 gap-1.5 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-white/[0.06]">
                <Archive className="w-3.5 h-3.5" />
                Archive
              </Button>
            </header>

            {/* Split Layout - Proposals left, Conversation right (matching Kanban pattern) */}
            <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
              {/* Proposals Panel (Left) */}
              <div
                data-testid="proposals-panel"
                className="flex flex-col border-r border-white/[0.06] bg-gradient-to-b from-black/10 to-transparent relative"
                style={{ width: `${leftPanelWidth}%`, minWidth: "360px" }}
                {...dropProps}
              >
                {/* Drop zone overlay - shown during drag */}
                <DropZoneOverlay isVisible={isDragging} message="Drop to import plan" />
                {/* Panel Header */}
                <div className="flex items-center justify-between px-4 h-10 border-b border-white/[0.06] bg-black/20">
                  <div className="flex items-center gap-2">
                    <ListTodo className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                    <h2 className="text-[13px] font-medium text-[var(--text-primary)]">Proposals</h2>
                    {isAnalyzingDependencies && (
                      <div className="flex items-center gap-1.5 text-[11px] text-[#ff6b35]">
                        <Loader2 className="w-3 h-3 animate-spin" />
                        <span>Analyzing...</span>
                      </div>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    {proposals.length >= 2 && (
                      <TooltipProvider>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={handleReanalyzeDependencies}
                              disabled={isAnalyzingDependencies}
                              className="h-7 w-7 text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-white/[0.06] disabled:opacity-50"
                            >
                              <Network className="w-3.5 h-3.5" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent side="bottom">
                            <p>Re-analyze dependencies</p>
                          </TooltipContent>
                        </Tooltip>
                      </TooltipProvider>
                    )}
                    {proposals.length > 0 && (
                      <span className="px-2 py-0.5 rounded-md text-[11px] font-medium bg-white/[0.05] text-[var(--text-muted)] border border-white/[0.06]">
                        {proposals.length}
                      </span>
                    )}
                  </div>
                </div>

                {proposals.length > 0 && (
                  <ProposalsToolbar
                    selectedCount={selectedCount}
                    totalCount={proposals.length}
                    onSelectAll={handleSelectAll}
                    onDeselectAll={handleDeselectAll}
                    onSortByPriority={handleSortByPriority}
                    onClearAll={handleClearAll}
                    onApply={handleApply}
                  />
                )}

                {/* Proposals List */}
                <div ref={proposalsScrollRef} className="flex-1 overflow-y-auto p-4">
                  {importStatus && (
                    <div className={cn(
                      "mb-4 p-4 rounded-xl border",
                      importStatus.type === "success"
                        ? "bg-emerald-500/10 border-emerald-500/30"
                        : "bg-red-500/10 border-red-500/30"
                    )}>
                      <div className="flex items-center justify-between">
                        <p className="text-sm font-medium text-[var(--text-primary)]">{importStatus.message}</p>
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
                    <Button variant="outline" onClick={handleImportPlan} className="w-full mb-4 gap-2 border-white/[0.1] hover:border-white/[0.2] hover:bg-white/[0.03]" data-testid="import-plan-button">
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
                        <div className="absolute inset-0 bg-[#ff6b35]/5 rounded-3xl blur-2xl" />
                        <div className="relative p-8 rounded-2xl bg-gradient-to-br from-white/[0.03] to-transparent border border-white/[0.06] text-center">
                          <Loader2 className="w-10 h-10 mx-auto mb-4 text-[#ff6b35] animate-spin" />
                          <p className="font-medium text-[var(--text-secondary)]">Waiting for implementation plan...</p>
                          <p className="text-sm text-[var(--text-muted)] mt-1">The orchestrator will create a plan first</p>
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
                className={cn("w-1 cursor-ew-resize relative group shrink-0", isResizing && "bg-[#ff6b35]/50")}
                onMouseDown={handleResizeStart}
              >
                <div className={cn(
                  "absolute top-0 bottom-0 left-1/2 -translate-x-1/2 w-px transition-all duration-150",
                  isResizing
                    ? "bg-[#ff6b35] shadow-[0_0_12px_rgba(255,107,53,0.5)]"
                    : "bg-white/[0.06] group-hover:bg-[#ff6b35]/60 group-hover:shadow-[0_0_8px_rgba(255,107,53,0.3)]"
                )} />
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
                  inputContainerClassName="border-t border-white/[0.06] bg-black/30"
                  headerContent={
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <MessageSquare className="w-3.5 h-3.5 shrink-0 text-[var(--text-muted)]" />
                      <span className="text-[13px] font-medium text-[var(--text-primary)]">Conversation</span>
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
