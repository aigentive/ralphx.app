/**
 * ProposalCard - macOS Tahoe styled proposal card
 *
 * Design: Glass-morphism card with refined shadows, smooth animations,
 * and warm orange accent for selection states.
 */

import React, { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { FileEdit, Trash2, Eye, ChevronDown, ChevronRight, ExternalLink } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TaskProposal } from "@/types/ideation";
import { PRIORITY_CONFIG } from "./PlanningView.constants";

// ============================================================================
// Types
// ============================================================================

export interface DependencyDetail {
  proposalId: string;
  title: string;
  reason?: string;
}

export interface ProposalCardProps {
  proposal: TaskProposal;
  onEdit: (proposalId: string) => void;
  onRemove: (proposalId: string) => void;
  isHighlighted?: boolean;
  currentPlanVersion?: number | undefined;
  onViewHistoricalPlan?: (artifactId: string, version: number) => void | undefined;
  dependsOnCount?: number;
  dependsOnDetails?: DependencyDetail[];
  blocksCount?: number;
  isOnCriticalPath?: boolean;
  /** Whether the plan is in read-only mode (accepted/archived status) */
  isReadOnly?: boolean;
  /** Callback to navigate to the created task in kanban */
  onNavigateToTask?: (taskId: string) => void;
}

// ============================================================================
// Component
// ============================================================================

const MAX_INLINE_DEPS = 2;

export const ProposalCard = React.memo(function ProposalCard({
  proposal,
  onEdit,
  onRemove,
  isHighlighted = false,
  currentPlanVersion,
  onViewHistoricalPlan,
  dependsOnCount: _dependsOnCount,
  dependsOnDetails,
  blocksCount,
  isOnCriticalPath,
  isReadOnly = false,
  onNavigateToTask,
}: ProposalCardProps) {
  const [isDepsExpanded, setIsDepsExpanded] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const config = PRIORITY_CONFIG[effectivePriority];

  const hasDependencies = dependsOnDetails && dependsOnDetails.length > 0;
  const visibleDeps = hasDependencies ? dependsOnDetails.slice(0, MAX_INLINE_DEPS) : [];
  const overflowCount = hasDependencies ? Math.max(0, dependsOnDetails.length - MAX_INLINE_DEPS) : 0;
  const inlineText = visibleDeps.map((d) => d.title).join(", ");

  const showHistoricalPlanLink =
    proposal.planArtifactId &&
    proposal.planVersionAtCreation &&
    currentPlanVersion &&
    proposal.planVersionAtCreation !== currentPlanVersion;

  const handleViewHistoricalPlan = () => {
    if (proposal.planArtifactId && proposal.planVersionAtCreation && onViewHistoricalPlan) {
      onViewHistoricalPlan(proposal.planArtifactId, proposal.planVersionAtCreation);
    }
  };

  // Priority-based accent colors - HSL format
  const priorityColors = {
    critical: { bg: "hsla(0 85% 60% / 0.08)", border: "hsla(0 85% 60% / 0.2)", text: "hsl(0 85% 60%)" },
    high: { bg: "hsla(14 100% 60% / 0.08)", border: "hsla(14 100% 60% / 0.2)", text: "hsl(14 100% 60%)" },
    medium: { bg: "hsla(45 93% 50% / 0.08)", border: "hsla(45 93% 50% / 0.2)", text: "hsl(45 93% 55%)" },
    low: { bg: "hsla(220 10% 50% / 0.08)", border: "hsla(220 10% 50% / 0.2)", text: "hsl(220 10% 50%)" },
  };

  const priorityColor = priorityColors[effectivePriority] || priorityColors.medium;

  return (
    <div
      data-testid={`proposal-card-${proposal.id}`}
      className={cn(
        "group relative rounded-xl",
        "transition-all duration-200 ease-out"
      )}
      style={{
        padding: "14px 16px",
        background: isHovered
          ? "hsla(220 10% 100% / 0.04)"
          : "hsla(220 10% 100% / 0.02)",
        border: isHighlighted
          ? "1px solid hsla(45 93% 50% / 0.4)"
          : "1px solid hsla(220 10% 100% / 0.06)",
        boxShadow: isHovered
          ? "0 2px 8px hsla(220 10% 0% / 0.2)"
          : "0 1px 2px hsla(220 10% 0% / 0.1)",
        ...(isOnCriticalPath && {
          borderBottom: "2px solid hsla(14 100% 60% / 0.4)",
        }),
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div className="flex items-start gap-3 pl-2">
        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <h3
              className="text-[13px] font-medium leading-snug tracking-[-0.01em]"
              style={{ color: "hsl(220 10% 90%)" }}
            >
              {proposal.title}
            </h3>

            {/* Actions - hidden in read-only mode */}
            {!isReadOnly && (
              <div
                className={cn(
                  "flex items-center gap-1 transition-opacity duration-150",
                  isHovered ? "opacity-100" : "opacity-0"
                )}
              >
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 rounded-lg transition-colors duration-150"
                        style={{ background: "transparent" }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(220 10% 100% / 0.08)"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
                        onClick={(e) => { e.stopPropagation(); onEdit(proposal.id); }}
                      >
                        <FileEdit className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 50%)" }} />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>Edit</TooltipContent>
                  </Tooltip>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 rounded-lg transition-colors duration-150"
                        style={{ background: "transparent" }}
                        onMouseEnter={(e) => { e.currentTarget.style.background = "hsla(0 70% 50% / 0.1)"; }}
                        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
                        onClick={(e) => { e.stopPropagation(); onRemove(proposal.id); }}
                      >
                        <Trash2 className="w-3.5 h-3.5" style={{ color: "hsl(0 70% 60%)" }} />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>Remove</TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>
            )}
          </div>

          <p
            className="text-[12px] mt-1.5 line-clamp-2 leading-relaxed"
            style={{ color: "hsl(220 10% 65%)" }}
          >
            {proposal.description || "No description"}
          </p>

          {/* Tags */}
          <div className="flex flex-wrap items-center gap-2 mt-3">
            <TooltipProvider>
              {isOnCriticalPath ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span
                      className="px-2 py-0.5 rounded-md text-[10px] font-semibold uppercase tracking-wider cursor-default"
                      style={{
                        background: priorityColor.bg,
                        border: `1px solid ${priorityColor.border}`,
                        color: priorityColor.text,
                      }}
                    >
                      {config.label}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>On critical path</TooltipContent>
                </Tooltip>
              ) : (
                <span
                  className="px-2 py-0.5 rounded-md text-[10px] font-semibold uppercase tracking-wider"
                  style={{
                    background: priorityColor.bg,
                    border: `1px solid ${priorityColor.border}`,
                    color: priorityColor.text,
                  }}
                >
                  {config.label}
                </span>
              )}
            </TooltipProvider>

            <span
              className="px-2 py-0.5 rounded-md text-[10px] font-medium"
              style={{
                background: "hsla(220 10% 100% / 0.04)",
                border: "1px solid hsla(220 10% 100% / 0.08)",
                color: "hsl(220 10% 55%)",
              }}
            >
              {proposal.category}
            </span>

            {proposal.userModified && (
              <span
                className="px-2 py-0.5 rounded-md text-[10px] font-medium italic"
                style={{
                  background: "hsla(270 70% 60% / 0.1)",
                  border: "1px solid hsla(270 70% 60% / 0.2)",
                  color: "hsl(270 70% 65%)",
                }}
              >
                Modified
              </span>
            )}

            {(blocksCount !== undefined && blocksCount > 0) && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span
                      data-testid="blocks-count"
                      className="px-1.5 py-0.5 rounded text-[10px] font-semibold cursor-default"
                      style={{ color: "hsl(14 100% 60%)" }}
                    >
                      →{blocksCount}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>Blocks {blocksCount} proposal{blocksCount !== 1 ? "s" : ""}</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            )}
          </div>

          {/* Dependencies */}
          {hasDependencies && (
            <div className="mt-3">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      data-testid="depends-on-inline"
                      onClick={(e) => { e.stopPropagation(); setIsDepsExpanded(!isDepsExpanded); }}
                      className="flex items-center gap-1.5 text-[11px] transition-colors duration-150"
                      style={{ color: "hsl(220 10% 50%)" }}
                      onMouseEnter={(e) => { e.currentTarget.style.color = "hsl(220 10% 70%)"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.color = "hsl(220 10% 50%)"; }}
                    >
                      <span data-testid="expand-dependencies" className="flex items-center">
                        {isDepsExpanded ? (
                          <ChevronDown className="w-3.5 h-3.5" />
                        ) : (
                          <ChevronRight className="w-3.5 h-3.5" />
                        )}
                      </span>
                      <span>← {inlineText}</span>
                      {overflowCount > 0 && (
                        <span style={{ opacity: 0.6 }}>+{overflowCount} more</span>
                      )}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs">
                    <div className="space-y-1.5 text-[12px]">
                      <div className="font-medium">
                        Depends on {dependsOnDetails!.length} proposal{dependsOnDetails!.length !== 1 ? "s" : ""}:
                      </div>
                      {dependsOnDetails!.map((dep) => (
                        <div key={dep.proposalId} style={{ color: "hsl(220 10% 50%)" }}>
                          • {dep.title}{dep.reason && `: ${dep.reason}`}
                        </div>
                      ))}
                    </div>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              {isDepsExpanded && (
                <div
                  data-testid="dependencies-expanded"
                  className="mt-2 pl-4 space-y-1.5"
                  style={{
                    borderLeft: "2px solid hsla(220 10% 100% / 0.06)",
                  }}
                >
                  {dependsOnDetails!.map((dep) => (
                    <div key={dep.proposalId} className="text-[11px]">
                      <div style={{ color: "hsl(220 10% 70%)" }} className="font-medium">
                        {dep.title}
                      </div>
                      {dep.reason && (
                        <div style={{ color: "hsl(220 10% 50%)" }} className="italic">
                          {dep.reason}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {showHistoricalPlanLink && (
            <button
              data-testid="view-historical-plan"
              onClick={(e) => { e.stopPropagation(); handleViewHistoricalPlan(); }}
              className="mt-3 text-[12px] flex items-center gap-1.5 transition-colors duration-150"
              style={{ color: "hsl(14 100% 60%)" }}
              onMouseEnter={(e) => { e.currentTarget.style.color = "hsl(14 100% 65%)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = "hsl(14 100% 60%)"; }}
            >
              <Eye className="w-3.5 h-3.5" />
              View plan as of proposal creation (v{proposal.planVersionAtCreation})
            </button>
          )}

          {/* View Task link - shown when proposal has been converted to task */}
          {proposal.createdTaskId && onNavigateToTask && (
            <button
              data-testid="view-task-link"
              onClick={(e) => { e.stopPropagation(); onNavigateToTask(proposal.createdTaskId!); }}
              className="mt-3 text-[12px] flex items-center gap-1.5 transition-colors duration-150"
              style={{ color: "hsl(14 100% 60%)" }}
              onMouseEnter={(e) => { e.currentTarget.style.color = "hsl(14 100% 65%)"; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = "hsl(14 100% 60%)"; }}
            >
              <ExternalLink className="w-3.5 h-3.5" />
              View Task →
            </button>
          )}
        </div>
      </div>
    </div>
  );
});
