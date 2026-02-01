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
import { CheckSquare, Square, ArrowUpDown, Trash2, ChevronDown, FileEdit, Inbox, ListTodo } from "lucide-react";
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
}: ProposalsToolbarProps) {
  const canApply = selectedCount > 0;

  return (
    <div
      className="flex items-center justify-between px-4 h-11"
      style={{
        borderBottom: "1px solid rgba(255,255,255,0.04)",
        background: "rgba(0,0,0,0.3)",
      }}
    >
      {/* Selection count */}
      <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>
        <span style={{ color: "var(--text-primary)" }} className="font-semibold">
          {selectedCount}
        </span>
        {" "}of {totalCount} selected
      </span>

      {/* Actions */}
      <div className="flex items-center gap-1">
        <TooltipProvider>
          {/* Select All */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 rounded-lg"
                onClick={onSelectAll}
                style={{ color: "var(--text-muted)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(255,255,255,0.06)";
                  e.currentTarget.style.color = "var(--text-primary)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "var(--text-muted)";
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
                style={{ color: "var(--text-muted)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(255,255,255,0.06)";
                  e.currentTarget.style.color = "var(--text-primary)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "var(--text-muted)";
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
          style={{ background: "rgba(255,255,255,0.08)" }}
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
                style={{ color: "var(--text-muted)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(255,255,255,0.06)";
                  e.currentTarget.style.color = "var(--text-primary)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "var(--text-muted)";
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
                style={{ color: "var(--text-muted)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "rgba(239,68,68,0.1)";
                  e.currentTarget.style.color = "#ef4444";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "var(--text-muted)";
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
          style={{ background: "rgba(255,255,255,0.08)" }}
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
                color: canApply ? "#ff6b35" : "var(--text-muted)",
                background: canApply ? "rgba(255,107,53,0.1)" : "transparent",
                border: canApply ? "1px solid rgba(255,107,53,0.2)" : "1px solid transparent",
              }}
              onMouseEnter={(e) => {
                if (canApply) {
                  e.currentTarget.style.background = "rgba(255,107,53,0.15)";
                }
              }}
              onMouseLeave={(e) => {
                if (canApply) {
                  e.currentTarget.style.background = "rgba(255,107,53,0.1)";
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
              background: "rgba(30,30,30,0.95)",
              backdropFilter: "blur(20px)",
              border: "1px solid rgba(255,255,255,0.1)",
              boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
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
