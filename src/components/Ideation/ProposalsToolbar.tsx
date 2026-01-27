/**
 * ProposalsToolbar - Toolbar for managing proposals
 *
 * Features:
 * - Selection count display
 * - Select all / Deselect all buttons
 * - Sort by priority
 * - Clear all
 */

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { CheckSquare, Square, ArrowUpDown, Trash2 } from "lucide-react";

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
}: ProposalsToolbarProps) {
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
      </div>
    </div>
  );
}
