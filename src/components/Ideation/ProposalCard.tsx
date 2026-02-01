/**
 * ProposalCard - macOS Tahoe styled proposal card
 *
 * Design: Glass-morphism card with refined shadows, smooth animations,
 * and warm orange accent for selection states.
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
  dependsOnCount?: number;
  dependsOnDetails?: DependencyDetail[];
  blocksCount?: number;
  isOnCriticalPath?: boolean;
}

// ============================================================================
// Component
// ============================================================================

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
  const [isHovered, setIsHovered] = useState(false);
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const isSelected = proposal.selected;
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

  // Priority-based accent colors
  const priorityColors = {
    critical: { bg: "rgba(239,68,68,0.08)", border: "rgba(239,68,68,0.2)", text: "#ef4444" },
    high: { bg: "rgba(255,107,53,0.08)", border: "rgba(255,107,53,0.2)", text: "#ff6b35" },
    medium: { bg: "rgba(245,158,11,0.08)", border: "rgba(245,158,11,0.2)", text: "#f59e0b" },
    low: { bg: "rgba(100,116,139,0.08)", border: "rgba(100,116,139,0.2)", text: "#64748b" },
  };

  const priorityColor = priorityColors[effectivePriority] || priorityColors.medium;

  return (
    <div
      data-testid={`proposal-card-${proposal.id}`}
      className={cn(
        "group relative rounded-xl cursor-pointer",
        "transition-all duration-200 ease-out"
      )}
      style={{
        padding: "14px 16px",
        background: isSelected
          ? "linear-gradient(135deg, rgba(255,107,53,0.1) 0%, rgba(255,107,53,0.04) 100%)"
          : isHovered
            ? "rgba(255,255,255,0.04)"
            : "rgba(255,255,255,0.02)",
        border: isHighlighted
          ? "1px solid rgba(234,179,8,0.4)"
          : isSelected
            ? "1px solid rgba(255,107,53,0.35)"
            : "1px solid rgba(255,255,255,0.06)",
        boxShadow: isHighlighted
          ? "0 0 24px rgba(234,179,8,0.1)"
          : isSelected
            ? "0 4px 16px rgba(255,107,53,0.08), 0 1px 3px rgba(0,0,0,0.2)"
            : isHovered
              ? "0 2px 8px rgba(0,0,0,0.15)"
              : "0 1px 2px rgba(0,0,0,0.1)",
        ...(isOnCriticalPath && {
          borderBottom: "2px solid rgba(255,107,53,0.4)",
        }),
      }}
      onClick={() => onSelect(proposal.id)}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Selection indicator */}
      <div
        className="absolute left-0 top-3 bottom-3 w-[3px] rounded-full transition-all duration-200"
        style={{
          background: isSelected ? "#ff6b35" : "transparent",
          boxShadow: isSelected ? "0 0 8px rgba(255,107,53,0.4)" : "none",
        }}
      />

      <div className="flex items-start gap-3 pl-2">
        {/* Checkbox */}
        <div className="pt-0.5">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => onSelect(proposal.id)}
            aria-label={`Select ${proposal.title}`}
            className="h-4 w-4 rounded-[5px] data-[state=checked]:bg-[#ff6b35] data-[state=checked]:border-[#ff6b35] border-white/20 transition-all duration-150"
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <h3
              className="text-[13px] font-medium leading-snug tracking-[-0.01em]"
              style={{ color: "var(--text-primary)" }}
            >
              {proposal.title}
            </h3>

            {/* Actions */}
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
                      className="h-7 w-7 rounded-lg hover:bg-white/[0.08]"
                      onClick={(e) => { e.stopPropagation(); onEdit(proposal.id); }}
                    >
                      <FileEdit className="w-3.5 h-3.5" style={{ color: "var(--text-muted)" }} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Edit</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 rounded-lg hover:bg-red-500/10"
                      onClick={(e) => { e.stopPropagation(); onRemove(proposal.id); }}
                    >
                      <Trash2 className="w-3.5 h-3.5 text-red-400/70 hover:text-red-400" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Remove</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          <p
            className="text-[12px] mt-1.5 line-clamp-2 leading-relaxed"
            style={{ color: "var(--text-secondary)" }}
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
                background: "rgba(255,255,255,0.04)",
                border: "1px solid rgba(255,255,255,0.08)",
                color: "var(--text-muted)",
              }}
            >
              {proposal.category}
            </span>

            {proposal.userModified && (
              <span
                className="px-2 py-0.5 rounded-md text-[10px] font-medium italic"
                style={{
                  background: "rgba(168,85,247,0.1)",
                  border: "1px solid rgba(168,85,247,0.2)",
                  color: "#a855f7",
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
                      style={{ color: "#ff6b35" }}
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
                      className="flex items-center gap-1.5 text-[11px] transition-colors"
                      style={{ color: "var(--text-muted)" }}
                      onMouseEnter={(e) => { e.currentTarget.style.color = "var(--text-secondary)"; }}
                      onMouseLeave={(e) => { e.currentTarget.style.color = "var(--text-muted)"; }}
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
                        <div key={dep.proposalId} style={{ color: "var(--text-muted)" }}>
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
                    borderLeft: "2px solid rgba(255,255,255,0.06)",
                  }}
                >
                  {dependsOnDetails!.map((dep) => (
                    <div key={dep.proposalId} className="text-[11px]">
                      <div style={{ color: "var(--text-secondary)" }} className="font-medium">
                        {dep.title}
                      </div>
                      {dep.reason && (
                        <div style={{ color: "var(--text-muted)" }} className="italic">
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
              className="mt-3 text-[12px] flex items-center gap-1.5 transition-colors"
              style={{ color: "#ff6b35" }}
              onMouseEnter={(e) => { e.currentTarget.style.color = "#ff8050"; }}
              onMouseLeave={(e) => { e.currentTarget.style.color = "#ff6b35"; }}
            >
              <Eye className="w-3.5 h-3.5" />
              View plan as of proposal creation (v{proposal.planVersionAtCreation})
            </button>
          )}
        </div>
      </div>
    </div>
  );
});
