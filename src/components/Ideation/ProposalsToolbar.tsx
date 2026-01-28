/**
 * ProposalsToolbar - Toolbar for managing proposals
 *
 * Features:
 * - Selection count display
 * - Select all / Deselect all buttons
 * - Sort by priority
 * - Clear all
 * - Apply to dropdown
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
    <div className="flex items-center justify-between px-3 py-2 border-b border-white/[0.06] bg-black/20">
      <span className="text-[10px] text-[var(--text-muted)]">
        <span className="text-[var(--text-primary)] font-medium">{selectedCount}</span> of {totalCount} selected
      </span>

      <div className="flex items-center gap-0.5">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-6 w-6 hover:bg-white/[0.06]" onClick={onSelectAll}>
                <CheckSquare className="w-3 h-3" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Select all</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-6 w-6 hover:bg-white/[0.06]" onClick={onDeselectAll}>
                <Square className="w-3 h-3" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Deselect all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <div className="w-px h-3 bg-white/[0.1] mx-0.5" />

        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-6 w-6 hover:bg-white/[0.06]" onClick={onSortByPriority}>
                <ArrowUpDown className="w-3 h-3" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Sort by priority</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-6 w-6 hover:bg-red-500/10 hover:text-red-400" onClick={onClearAll}>
                <Trash2 className="w-3 h-3" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Clear all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <div className="w-px h-3 bg-white/[0.1] mx-0.5" />

        {/* Apply to dropdown */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              disabled={!canApply}
              className={cn(
                "h-6 px-2 text-[10px] gap-1",
                canApply
                  ? "text-[#ff6b35] hover:bg-[#ff6b35]/10"
                  : "text-[var(--text-muted)]"
              )}
            >
              Apply
              <ChevronDown className="w-3 h-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="bg-[#1a1a1a] border-white/[0.1]">
            <DropdownMenuItem onClick={() => onApply("draft")} className="hover:bg-white/[0.06]">
              <FileEdit className="w-4 h-4 mr-2" /> Draft
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onApply("backlog")} className="hover:bg-white/[0.06]">
              <Inbox className="w-4 h-4 mr-2" /> Backlog
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => onApply("todo")} className="hover:bg-white/[0.06]">
              <ListTodo className="w-4 h-4 mr-2" /> Todo
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}
