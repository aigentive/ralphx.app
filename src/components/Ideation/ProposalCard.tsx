/**
 * ProposalCard - Card displaying a task proposal
 *
 * Features:
 * - Selection checkbox with orange accent
 * - Priority gradient background
 * - Edit/Remove actions on hover
 * - Category and modification badges
 * - Inline dependency names with expandable details
 * - Historical plan link when applicable
 */

import React, { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { FileEdit, Trash2, Eye, ChevronDown, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TaskProposal } from "@/types/ideation";
import { PRIORITY_CONFIG } from "./IdeationView.constants";

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
  onSelect: (proposalId: string) => void;
  onEdit: (proposalId: string) => void;
  onRemove: (proposalId: string) => void;
  isHighlighted?: boolean;
  currentPlanVersion?: number | undefined;
  onViewHistoricalPlan?: (artifactId: string, version: number) => void | undefined;
  /** Number of proposals this proposal depends on */
  dependsOnCount?: number;
  /** Details of proposals this proposal depends on (for rich tooltips) */
  dependsOnDetails?: DependencyDetail[];
  /** Number of proposals blocked by this proposal */
  blocksCount?: number;
  /** Whether this proposal is on the critical path */
  isOnCriticalPath?: boolean;
}

// ============================================================================
// Component
// ============================================================================

/** Max number of dependency names to show inline before truncating */
const MAX_INLINE_DEPS = 2;

export const ProposalCard = React.memo(function ProposalCard({
  proposal,
  onSelect,
  onEdit,
  onRemove,
  isHighlighted = false,
  currentPlanVersion,
  onViewHistoricalPlan,
  dependsOnCount: _dependsOnCount,
  dependsOnDetails,
  blocksCount,
  isOnCriticalPath,
}: ProposalCardProps) {
  const [isDepsExpanded, setIsDepsExpanded] = useState(false);
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const isSelected = proposal.selected;
  const config = PRIORITY_CONFIG[effectivePriority];

  // Compute inline dependency display
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

  return (
    <div
      data-testid={`proposal-card-${proposal.id}`}
      className={cn(
        "group relative p-3 rounded-lg transition-all duration-200 cursor-pointer",
        "bg-gradient-to-br",
        config.gradient,
        "border",
        isHighlighted
          ? "border-yellow-500/50 shadow-[0_0_20px_rgba(234,179,8,0.15)]"
          : isSelected
            ? "border-[#ff6b35]/40 shadow-[0_0_20px_rgba(255,107,53,0.1)]"
            : "border-white/[0.06] hover:border-white/[0.1] hover:shadow-md hover:shadow-black/15",
        config.glow,
        isOnCriticalPath && "border-b-2 border-b-[#ff6b35]/40"
      )}
      onClick={() => onSelect(proposal.id)}
    >
      {/* Selection indicator bar */}
      <div className={cn(
        "absolute left-0 top-2 bottom-2 w-0.5 rounded-full transition-all duration-200",
        isSelected ? "bg-[#ff6b35]" : "bg-transparent"
      )} />

      <div className="flex items-start gap-2 pl-1.5">
        {/* Checkbox */}
        <div className="pt-px">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => onSelect(proposal.id)}
            aria-label={`Select ${proposal.title}`}
            className="h-3.5 w-3.5 data-[state=checked]:bg-[#ff6b35] data-[state=checked]:border-[#ff6b35] border-white/20"
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-1.5">
            <h3 className="text-xs font-medium text-[var(--text-primary)] leading-snug">
              {proposal.title}
            </h3>

            {/* Actions */}
            <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 hover:bg-white/[0.06]"
                      onClick={(e) => { e.stopPropagation(); onEdit(proposal.id); }}
                    >
                      <FileEdit className="w-3 h-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Edit</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6 hover:bg-red-500/10 hover:text-red-400"
                      onClick={(e) => { e.stopPropagation(); onRemove(proposal.id); }}
                    >
                      <Trash2 className="w-3 h-3" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Remove</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          <p className="text-[11px] text-[var(--text-secondary)] mt-1 line-clamp-2 leading-relaxed">
            {proposal.description || "No description"}
          </p>

          {/* Tags */}
          <TooltipProvider>
            <div className="flex flex-wrap items-center gap-1.5 mt-2">
              {isOnCriticalPath ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span className={cn(
                      "px-1.5 py-px rounded text-[9px] font-medium uppercase tracking-wider cursor-default",
                      effectivePriority === "critical" && "bg-red-500/20 text-red-400",
                      effectivePriority === "high" && "bg-[#ff6b35]/20 text-[#ff6b35]",
                      effectivePriority === "medium" && "bg-amber-500/20 text-amber-400",
                      effectivePriority === "low" && "bg-slate-500/20 text-slate-400"
                    )}>
                      {config.label}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>On critical path</TooltipContent>
                </Tooltip>
              ) : (
                <span className={cn(
                  "px-1.5 py-px rounded text-[9px] font-medium uppercase tracking-wider",
                  effectivePriority === "critical" && "bg-red-500/20 text-red-400",
                  effectivePriority === "high" && "bg-[#ff6b35]/20 text-[#ff6b35]",
                  effectivePriority === "medium" && "bg-amber-500/20 text-amber-400",
                  effectivePriority === "low" && "bg-slate-500/20 text-slate-400"
                )}>
                  {config.label}
                </span>
              )}
              <span className="px-1.5 py-px rounded text-[9px] font-medium bg-white/[0.05] text-[var(--text-muted)] border border-white/[0.06]">
                {proposal.category}
              </span>
              {proposal.userModified && (
                <span className="px-1.5 py-px rounded text-[9px] font-medium bg-purple-500/20 text-purple-400 italic">
                  Modified
                </span>
              )}
              {/* Blocks count badge */}
              {(blocksCount !== undefined && blocksCount > 0) && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span
                      data-testid="blocks-count"
                      className="px-1 py-px rounded text-[9px] font-medium text-[#ff6b35] cursor-default"
                    >
                      →{blocksCount}
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>Blocks {blocksCount} proposal{blocksCount !== 1 ? "s" : ""}</TooltipContent>
                </Tooltip>
              )}
            </div>
          </TooltipProvider>

          {/* Inline dependency display */}
          {hasDependencies && (
            <div className="mt-2">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      data-testid="depends-on-inline"
                      onClick={(e) => { e.stopPropagation(); setIsDepsExpanded(!isDepsExpanded); }}
                      className="flex items-center gap-1 text-[10px] text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
                    >
                      <span
                        data-testid="expand-dependencies"
                        className="flex items-center"
                      >
                        {isDepsExpanded ? (
                          <ChevronDown className="w-3 h-3" />
                        ) : (
                          <ChevronRight className="w-3 h-3" />
                        )}
                      </span>
                      <span>← {inlineText}</span>
                      {overflowCount > 0 && (
                        <span className="text-[var(--text-muted)]">+{overflowCount} more</span>
                      )}
                    </button>
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs">
                    <div className="space-y-1 text-xs">
                      <div className="font-medium">
                        Depends on {dependsOnDetails!.length} proposal{dependsOnDetails!.length !== 1 ? "s" : ""}:
                      </div>
                      {dependsOnDetails!.map((dep) => (
                        <div key={dep.proposalId} className="text-[var(--text-muted)]">
                          • {dep.title}{dep.reason && `: ${dep.reason}`}
                        </div>
                      ))}
                    </div>
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>

              {/* Expanded dependency details */}
              {isDepsExpanded && (
                <div
                  data-testid="dependencies-expanded"
                  className="mt-2 pl-4 border-l-2 border-white/[0.06] space-y-1"
                >
                  {dependsOnDetails!.map((dep) => (
                    <div key={dep.proposalId} className="text-[10px]">
                      <div className="text-[var(--text-secondary)] font-medium">{dep.title}</div>
                      {dep.reason && (
                        <div className="text-[var(--text-muted)] italic">{dep.reason}</div>
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
              className="mt-3 text-xs text-[#ff6b35] hover:text-[#ff8050] flex items-center gap-1.5 transition-colors"
            >
              <Eye className="w-3 h-3" />
              View plan as of proposal creation (v{proposal.planVersionAtCreation})
            </button>
          )}
        </div>
      </div>
    </div>
  );
});
