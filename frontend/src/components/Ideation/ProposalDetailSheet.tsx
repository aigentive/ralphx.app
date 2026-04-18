/**
 * ProposalDetailSheet - Right-sliding detail panel for proposal inspection
 *
 * Design: Dark glass aesthetic with backdrop blur, warm orange accent
 */

import React, { useEffect, useCallback, useState } from "react";
import { X, FileEdit, Trash2, ExternalLink, CheckSquare } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import type { TaskProposal } from "@/types/ideation";
import type { DependencyDetail } from "./ProposalCard";
import { PRIORITY_CONFIG } from "./PlanningView.constants";

// ============================================================================
// Types
// ============================================================================

export interface ProposalDetailEnrichment {
  dependsOnDetails: DependencyDetail[];
  blocksCount: number;
  isOnCriticalPath: boolean;
}

export interface ProposalDetailSheetProps {
  proposal: TaskProposal | null;
  enrichment?: ProposalDetailEnrichment;
  isReadOnly?: boolean;
  onClose: () => void;
  onEdit?: (proposalId: string) => void;
  onDelete?: (proposalId: string) => void;
  onNavigateToTask?: (taskId: string) => void;
}

// ============================================================================
// Sub-components
// ============================================================================

function MetadataChip({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div
      className="flex flex-col gap-0.5 px-3 py-2 rounded-lg"
      style={{
        background: accent ? "hsla(14 100% 60% / 0.08)" : "hsla(220 10% 100% / 0.04)",
        border: accent ? "1px solid hsla(14 100% 60% / 0.2)" : "1px solid hsla(220 10% 100% / 0.06)",
      }}
    >
      <span className="text-[10px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
        {label}
      </span>
      <span className="text-[12px] font-medium" style={{ color: accent ? "hsl(14 100% 65%)" : "hsl(220 10% 85%)" }}>
        {value}
      </span>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export const ProposalDetailSheet = React.memo(function ProposalDetailSheet({
  proposal,
  enrichment,
  isReadOnly = false,
  onClose,
  onEdit,
  onDelete,
  onNavigateToTask,
}: ProposalDetailSheetProps) {
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    },
    [onClose]
  );

  useEffect(() => {
    if (!proposal) return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [proposal, handleKeyDown]);

  if (!proposal) return null;

  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const config = PRIORITY_CONFIG[effectivePriority];

  const priorityColors = {
    critical: { bg: "hsla(0 85% 60% / 0.08)", border: "hsla(0 85% 60% / 0.2)", text: "hsl(0 85% 60%)" },
    high: { bg: "hsla(14 100% 60% / 0.08)", border: "hsla(14 100% 60% / 0.2)", text: "hsl(14 100% 60%)" },
    medium: { bg: "hsla(45 93% 50% / 0.08)", border: "hsla(45 93% 50% / 0.2)", text: "hsl(45 93% 55%)" },
    low: { bg: "hsla(220 10% 50% / 0.08)", border: "hsla(220 10% 50% / 0.2)", text: "hsl(220 10% 50%)" },
  };
  const priorityColor = priorityColors[effectivePriority] || priorityColors.medium;

  const complexityLabel: Record<string, string> = {
    trivial: "Trivial",
    simple: "Simple",
    moderate: "Moderate",
    complex: "Complex",
    very_complex: "Very Complex",
  };

  return (
    <>
      {/* Backdrop */}
      <div
        data-testid="proposal-detail-backdrop"
        className="fixed inset-0 z-40"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Panel */}
      <div
        data-testid="proposal-detail-sheet"
        className="fixed top-0 right-0 h-full z-50 flex flex-col"
        style={{
          width: "420px",
          background: "hsla(220 10% 8% / 0.96)",
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
          borderLeft: "1px solid hsla(220 20% 100% / 0.08)",
          boxShadow: "-4px 0 24px hsla(220 20% 0% / 0.5), -12px 0 48px hsla(220 20% 0% / 0.3)",
          animation: "slide-in-from-right 200ms ease-out both",
        }}
        role="dialog"
        aria-label="Proposal details"
        aria-modal="true"
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-5 py-4 flex-shrink-0"
          style={{ borderBottom: "1px solid hsla(220 10% 100% / 0.06)" }}
        >
          <div className="flex items-center gap-2.5 min-w-0">
            <div
              className="w-1 h-5 rounded-full flex-shrink-0"
              style={{ background: "var(--accent-primary)" }}
            />
            <h2
              className="text-[13px] font-semibold truncate"
              style={{ color: "hsl(220 10% 90%)" }}
            >
              Proposal Details
            </h2>
          </div>
          <div className="flex items-center gap-1.5 flex-shrink-0">
            {!isReadOnly && onEdit && (
              <Button
                data-testid="edit-proposal-button"
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                style={{ color: "hsl(220 10% 55%)" }}
                onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)"; e.currentTarget.style.color = "hsl(220 10% 80%)"; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "hsl(220 10% 55%)"; }}
                onClick={() => onEdit(proposal.id)}
                title="Edit proposal"
              >
                <FileEdit className="w-3.5 h-3.5" />
              </Button>
            )}
            {!isReadOnly && onDelete && (
              <Button
                data-testid="delete-proposal-button"
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                style={{ color: "hsl(220 10% 55%)" }}
                onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(0 85% 60% / 0.08)"; e.currentTarget.style.color = "hsl(0 85% 60%)"; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "hsl(220 10% 55%)"; }}
                onClick={() => setDeleteDialogOpen(true)}
                title="Delete proposal"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </Button>
            )}
            <Button
              data-testid="close-sheet-button"
              variant="ghost"
              size="icon"
              className="h-7 w-7 rounded-lg"
              style={{ color: "hsl(220 10% 55%)" }}
              onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)"; e.currentTarget.style.color = "hsl(220 10% 80%)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "hsl(220 10% 55%)"; }}
              onClick={onClose}
              title="Close"
            >
              <X className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto">
          <div className="px-5 py-5 space-y-6">
            {/* Title */}
            <div>
              <h3
                className="text-[15px] font-semibold leading-snug tracking-[-0.01em]"
                style={{ color: "hsl(220 10% 92%)" }}
              >
                {proposal.title}
              </h3>
              {enrichment?.isOnCriticalPath && (
                <span
                  className="inline-block mt-1.5 px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider"
                  style={{
                    background: "hsla(14 100% 60% / 0.1)",
                    border: "1px solid hsla(14 100% 60% / 0.25)",
                    color: "hsl(14 100% 60%)",
                  }}
                >
                  Critical Path
                </span>
              )}
            </div>

            {/* Description */}
            {proposal.description && (
              <div className="space-y-1.5">
                <span className="text-[11px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Description
                </span>
                <p
                  className="text-[13px] leading-relaxed"
                  style={{ color: "hsl(220 10% 70%)" }}
                >
                  {proposal.description}
                </p>
              </div>
            )}

            {/* Metadata chips */}
            <div className="grid grid-cols-3 gap-2">
              <div
                className="flex flex-col gap-0.5 px-3 py-2 rounded-lg"
                style={{
                  background: priorityColor.bg,
                  border: `1px solid ${priorityColor.border}`,
                }}
              >
                <span className="text-[10px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Priority
                </span>
                <span className="text-[12px] font-semibold" style={{ color: priorityColor.text }}>
                  {config.label}
                </span>
              </div>
              <MetadataChip label="Category" value={proposal.category} />
              <MetadataChip label="Complexity" value={complexityLabel[proposal.estimatedComplexity] ?? proposal.estimatedComplexity} />
            </div>

            {/* Priority Reason */}
            {proposal.priorityReason && (
              <div className="space-y-1.5">
                <span className="text-[11px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Priority Rationale
                </span>
                <p
                  className="text-[12px] leading-relaxed italic"
                  style={{ color: "hsl(220 10% 60%)" }}
                >
                  "{proposal.priorityReason}"
                </p>
              </div>
            )}

            {/* Implementation Steps */}
            {proposal.steps.length > 0 && (
              <div className="space-y-2">
                <span className="text-[11px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Implementation Steps
                </span>
                <ol className="space-y-2">
                  {proposal.steps.map((step, index) => (
                    <li key={index} className="flex items-start gap-3">
                      <span
                        className="flex-shrink-0 text-[11px] font-mono font-semibold mt-0.5 w-4 text-right"
                        style={{ color: "var(--accent-primary)" }}
                      >
                        {index + 1}.
                      </span>
                      <span className="text-[13px] leading-snug" style={{ color: "hsl(220 10% 75%)" }}>
                        {step}
                      </span>
                    </li>
                  ))}
                </ol>
              </div>
            )}

            {/* Acceptance Criteria */}
            {proposal.acceptanceCriteria.length > 0 && (
              <div className="space-y-2">
                <span className="text-[11px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Acceptance Criteria
                </span>
                <ul className="space-y-1.5">
                  {proposal.acceptanceCriteria.map((criterion, index) => (
                    <li key={index} className="flex items-start gap-2.5">
                      <CheckSquare
                        className="w-3.5 h-3.5 flex-shrink-0 mt-0.5"
                        style={{ color: "hsla(14 100% 60% / 0.5)" }}
                      />
                      <span className="text-[13px] leading-snug" style={{ color: "hsl(220 10% 75%)" }}>
                        {criterion}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Dependencies */}
            {enrichment && enrichment.dependsOnDetails.length > 0 && (
              <div className="space-y-2">
                <span className="text-[11px] font-medium uppercase tracking-wider" style={{ color: "hsl(220 10% 45%)" }}>
                  Depends On ({enrichment.dependsOnDetails.length})
                </span>
                <ul className="space-y-1.5">
                  {enrichment.dependsOnDetails.map((dep) => (
                    <li
                      key={dep.proposalId}
                      className="px-3 py-2 rounded-lg"
                      style={{
                        background: "hsla(220 10% 100% / 0.03)",
                        border: "1px solid hsla(220 10% 100% / 0.06)",
                      }}
                    >
                      <div className="text-[12px] font-medium" style={{ color: "hsl(220 10% 75%)" }}>
                        {dep.title}
                      </div>
                      {dep.reason && (
                        <div className="text-[11px] mt-0.5 italic" style={{ color: "hsl(220 10% 50%)" }}>
                          {dep.reason}
                        </div>
                      )}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {/* Blocks count */}
            {enrichment && enrichment.blocksCount > 0 && (
              <div
                className="flex items-center gap-2 px-3 py-2.5 rounded-lg"
                style={{
                  background: "hsla(14 100% 60% / 0.06)",
                  border: "1px solid hsla(14 100% 60% / 0.15)",
                }}
              >
                <span className="text-[12px] font-semibold" style={{ color: "hsl(14 100% 65%)" }}>
                  →{enrichment.blocksCount}
                </span>
                <span className="text-[12px]" style={{ color: "hsl(220 10% 60%)" }}>
                  Blocks {enrichment.blocksCount} proposal{enrichment.blocksCount !== 1 ? "s" : ""}
                </span>
              </div>
            )}

            {/* View Task link */}
            {proposal.createdTaskId && onNavigateToTask && (
              <button
                data-testid="view-task-from-detail"
                onClick={() => onNavigateToTask(proposal.createdTaskId!)}
                className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg text-[13px] font-medium transition-colors duration-150"
                style={{
                  background: "hsla(14 100% 60% / 0.08)",
                  border: "1px solid hsla(14 100% 60% / 0.2)",
                  color: "hsl(14 100% 60%)",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(14 100% 60% / 0.12)"; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = "hsla(14 100% 60% / 0.08)"; }}
              >
                <ExternalLink className="w-3.5 h-3.5" />
                View Task →
              </button>
            )}
          </div>
        </div>
      </div>
      {onDelete && (
        <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Delete Proposal</AlertDialogTitle>
              <AlertDialogDescription>
                Are you sure you want to delete "{proposal.title}"? This action cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={(e) => {
                  e.preventDefault();
                  onDelete(proposal.id);
                  onClose();
                }}
                className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              >
                Delete
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      )}
    </>
  );
});
