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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { CheckSquare, Square, ArrowUpDown, Trash2, ChevronDown, FileEdit, Inbox, ListTodo, Network, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

interface ProposalsToolbarProps {
  selectedCount: number;
  totalCount: number;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onSortByPriority: () => void;
  onClearAll: () => void;
  onApply: (targetColumn: string) => void;
  onAnalyzeDependencies?: () => void;
  isAnalyzingDependencies?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function ProposalsToolbar({
  selectedCount,
  totalCount,
  onSelectAll,
  onDeselectAll,
  onSortByPriority,
  onClearAll,
  onApply,
  onAnalyzeDependencies,
  isAnalyzingDependencies = false,
}: ProposalsToolbarProps) {
  const canApply = selectedCount > 0;
  const showAnalyzeButton = totalCount >= 2 && onAnalyzeDependencies;

  return (
    <div
      className="flex items-center justify-between px-4 h-11"
      style={{
        borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
        background: "hsla(220 10% 8% / 0.6)",
      }}
    >
      {/* Left: Selection count and analyzing status */}
      <div className="flex items-center gap-3">
        <span className="text-[11px]" style={{ color: "hsl(220 10% 50%)" }}>
          <span style={{ color: "hsl(220 10% 90%)" }} className="font-semibold">
            {selectedCount}
          </span>
          {" "}of {totalCount} selected
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

          {/* Select All */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                onClick={onSelectAll}
                style={{ color: "hsl(220 10% 50%)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 90%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <CheckSquare className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Select all</TooltipContent>
          </Tooltip>

          {/* Deselect All */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                onClick={onDeselectAll}
                style={{ color: "hsl(220 10% 50%)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 90%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <Square className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Deselect all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        {/* Separator */}
        <div
          className="w-px h-4 mx-1"
          style={{ background: "hsla(220 10% 100% / 0.08)" }}
        />

        <TooltipProvider>
          {/* Sort */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                onClick={onSortByPriority}
                style={{ color: "hsl(220 10% 50%)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 90%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <ArrowUpDown className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Sort by priority</TooltipContent>
          </Tooltip>

          {/* Clear All */}
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
            <TooltipContent>Clear all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        {/* Separator */}
        <div
          className="w-px h-4 mx-1"
          style={{ background: "hsla(220 10% 100% / 0.08)" }}
        />

        {/* Apply dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              disabled={!canApply}
              className={cn(
                "h-7 px-3 text-[11px] font-semibold gap-1.5 rounded-lg",
                "transition-all duration-150"
              )}
              style={{
                color: canApply ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)",
                background: canApply ? "hsla(14 100% 60% / 0.1)" : "transparent",
                border: canApply ? "1px solid hsla(14 100% 60% / 0.2)" : "1px solid transparent",
              }}
              onMouseEnter={(e) => {
                if (canApply) {
                  e.currentTarget.style.background = "hsla(14 100% 60% / 0.15)";
                }
              }}
              onMouseLeave={(e) => {
                if (canApply) {
                  e.currentTarget.style.background = "hsla(14 100% 60% / 0.1)";
                }
              }}
            >
              Apply
              <ChevronDown className="w-3 h-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            align="end"
            className="w-36"
            style={{
              background: "hsl(220 10% 14%)",
              backdropFilter: "blur(20px)",
              border: "1px solid hsla(220 10% 100% / 0.1)",
              boxShadow: "0 8px 32px hsla(220 10% 0% / 0.4)",
            }}
          >
            <DropdownMenuItem
              onClick={() => onApply("draft")}
              className="text-[13px] cursor-pointer gap-2.5 py-2"
            >
              <FileEdit className="w-4 h-4" />
              Draft
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onApply("backlog")}
              className="text-[13px] cursor-pointer gap-2.5 py-2"
            >
              <Inbox className="w-4 h-4" />
              Backlog
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onApply("todo")}
              className="text-[13px] cursor-pointer gap-2.5 py-2"
            >
              <ListTodo className="w-4 h-4" />
              Todo
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
