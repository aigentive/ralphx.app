/**
 * ProposalsToolbar - macOS Tahoe styled action toolbar
 *
 * Design: Refined toolbar with subtle separators, icon-based actions,
 * and warm orange accent for primary actions.
 */

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Trash2,
  Network,
  Loader2,
  Check,
  AlertCircle,
} from "lucide-react";
import { useDependencyGraphValidation } from "@/hooks/useDependencyGraphComplete";
import type { TaskProposal } from "@/types/ideation";
import type { DependencyGraphResponse } from "@/api/ideation.types";

// ============================================================================
// Types
// ============================================================================

interface ProposalsToolbarProps {
  proposals: TaskProposal[];
  graph: DependencyGraphResponse | null | undefined;
  isReadOnly?: boolean;
  onClearAll: () => void;
  onAcceptPlan: (targetColumn: string) => void;
  onAnalyzeDependencies?: () => void;
  isAnalyzingDependencies?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function ProposalsToolbar({
  proposals,
  graph,
  isReadOnly = false,
  onClearAll,
  onAcceptPlan,
  onAnalyzeDependencies,
  isAnalyzingDependencies = false,
}: ProposalsToolbarProps) {
  const totalCount = proposals.length;
  const validation = useDependencyGraphValidation(proposals, graph);
  const canAccept = totalCount > 0 && !isReadOnly && validation.isComplete;
  const showAnalyzeButton = totalCount >= 2 && onAnalyzeDependencies && !isReadOnly;

  return (
    <div
      className="flex items-center justify-between px-4 h-11"
      style={{
        borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
        background: "hsla(220 10% 8% / 0.6)",
      }}
    >
      {/* Left: Proposal count and analyzing status */}
      <div className="flex items-center gap-3">
        <span className="text-[11px]" style={{ color: "hsl(220 10% 50%)" }}>
          <span style={{ color: "hsl(220 10% 90%)" }} className="font-semibold">
            {totalCount}
          </span>
          {" "}{totalCount === 1 ? "proposal" : "proposals"}
        </span>

        {isAnalyzingDependencies && (
          <div className="flex items-center gap-1.5 text-[11px]" style={{ color: "hsl(14 100% 60%)" }}>
            <Loader2 className="w-3 h-3 animate-spin" />
            <span>Analyzing...</span>
          </div>
        )}
      </div>

      {/* Right: Actions */}
      <div className="flex items-center gap-1">
        <TooltipProvider>
          {/* Analyze Dependencies */}
          {showAnalyzeButton && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onAnalyzeDependencies}
                  disabled={isAnalyzingDependencies}
                  className="h-7 w-7 rounded-lg disabled:opacity-50 transition-colors duration-150"
                  style={{ color: "hsl(220 10% 50%)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = "hsl(220 10% 90%)";
                    e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = "hsl(220 10% 50%)";
                    e.currentTarget.style.background = "transparent";
                  }}
                >
                  <Network className="w-3.5 h-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Re-analyze dependencies</TooltipContent>
            </Tooltip>
          )}

          {/* Separator after analyze button */}
          {showAnalyzeButton && (
            <div
              className="w-px h-4 mx-1"
              style={{ background: "hsla(220 10% 100% / 0.08)" }}
            />
          )}

          {/* Clear All (only when not read-only) */}
          {!isReadOnly && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 rounded-lg"
                  onClick={onClearAll}
                  style={{ color: "hsl(220 10% 50%)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "hsla(0 70% 50% / 0.1)";
                    e.currentTarget.style.color = "hsl(0 70% 60%)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "transparent";
                    e.currentTarget.style.color = "hsl(220 10% 50%)";
                  }}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Clear all proposals</TooltipContent>
            </Tooltip>
          )}

          {/* Separator before Accept Plan */}
          {!isReadOnly && (
            <div
              className="w-px h-4 mx-1"
              style={{ background: "hsla(220 10% 100% / 0.08)" }}
            />
          )}

          {/* Graph incomplete warning */}
          {!isReadOnly && totalCount > 0 && !validation.isComplete && (
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center px-1">
                  <AlertCircle
                    className="w-4 h-4"
                    style={{ color: "hsl(40 90% 55%)" }}
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent className="max-w-xs">
                {validation.message}
              </TooltipContent>
            </Tooltip>
          )}
        </TooltipProvider>

        {/* Accept Plan button (only when not read-only) */}
        {/* Status is determined automatically by backend based on dependencies */}
        {!isReadOnly && (
          <Button
            variant="ghost"
            size="sm"
            disabled={!canAccept}
            onClick={() => onAcceptPlan("auto")}
            className="h-7 px-3 text-[11px] font-semibold gap-1.5 rounded-lg transition-all duration-150"
            style={{
              color: canAccept ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)",
              background: canAccept ? "hsla(14 100% 60% / 0.1)" : "transparent",
              border: canAccept ? "1px solid hsla(14 100% 60% / 0.2)" : "1px solid transparent",
            }}
            onMouseEnter={(e) => {
              if (canAccept) {
                e.currentTarget.style.background = "hsla(14 100% 60% / 0.15)";
              }
            }}
            onMouseLeave={(e) => {
              if (canAccept) {
                e.currentTarget.style.background = "hsla(14 100% 60% / 0.1)";
              }
            }}
          >
            <Check className="w-3 h-3" />
            Accept Plan ({totalCount})
          </Button>
        )}
      </div>
    </div>
  );
}
