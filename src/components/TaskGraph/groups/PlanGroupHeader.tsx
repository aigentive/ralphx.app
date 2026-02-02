/**
 * PlanGroupHeader.tsx - Header for plan groups in the Task Graph
 *
 * Displays:
 * - Plan title (linked to planning session)
 * - Task count
 * - Progress bar (completed / total)
 * - Collapse toggle
 */

import { memo, useMemo } from "react";
import { ChevronDown, ChevronRight, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import type { StatusSummary } from "@/api/task-graph.types";

// ============================================================================
// Types
// ============================================================================

export interface PlanGroupHeaderProps {
  /** Plan artifact ID for linking */
  planArtifactId: string;
  /** Session ID for navigation */
  sessionId: string;
  /** Session/plan title to display */
  sessionTitle: string | null;
  /** Number of tasks in this plan group */
  taskCount: number;
  /** Status summary with counts by category */
  statusSummary: StatusSummary;
  /** Whether the group is collapsed */
  isCollapsed: boolean;
  /** Toggle collapse state */
  onToggleCollapse: () => void;
  /** Optional: Open context menu */
  onContextMenu?: () => void;
  /** Optional: Navigate to planning session */
  onNavigateToSession?: () => void;
}

// ============================================================================
// Progress Bar Component
// ============================================================================

interface ProgressBarProps {
  completed: number;
  total: number;
}

const ProgressBar = memo(function ProgressBar({
  completed,
  total,
}: ProgressBarProps) {
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <div className="flex items-center gap-2 min-w-[120px]">
      <div className="flex-1 h-1.5 bg-[hsl(var(--bg-surface))] rounded-full overflow-hidden">
        <div
          className="h-full bg-[hsl(145,60%,45%)] rounded-full transition-all duration-300"
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-xs text-[hsl(var(--text-muted))] whitespace-nowrap">
        {percentage}%
      </span>
    </div>
  );
});

// ============================================================================
// Main Component
// ============================================================================

export const PlanGroupHeader = memo(function PlanGroupHeader({
  sessionTitle,
  taskCount,
  statusSummary,
  isCollapsed,
  onToggleCollapse,
  onNavigateToSession,
}: PlanGroupHeaderProps) {
  // Calculate aggregated counts for display
  const counts = useMemo(() => {
    const done = statusSummary.completed;
    const executing = statusSummary.executing;
    const blocked = statusSummary.blocked;
    const review = statusSummary.review;
    const merge = statusSummary.merge;
    const qa = statusSummary.qa;
    const terminal = statusSummary.terminal;
    const idle = statusSummary.backlog + statusSummary.ready;

    return {
      done,
      executing,
      blocked,
      review,
      merge,
      qa,
      terminal,
      idle,
      total: taskCount,
    };
  }, [statusSummary, taskCount]);

  const displayTitle = sessionTitle || "Unnamed Plan";

  return (
    <div
      className="flex flex-col gap-1.5 px-3 py-2 bg-[hsl(var(--bg-elevated)/0.8)] rounded-t-lg cursor-pointer"
      onDoubleClick={onToggleCollapse}
    >
      {/* Top row: collapse toggle + title + count + context menu */}
      <div className="flex items-center gap-2">
        <button
          onClick={onToggleCollapse}
          className="flex-shrink-0 p-0.5 rounded hover:bg-[hsl(var(--bg-surface))] transition-colors"
          aria-label={isCollapsed ? "Expand group" : "Collapse group"}
        >
          {isCollapsed ? (
            <ChevronRight className="w-4 h-4 text-[hsl(var(--text-muted))]" />
          ) : (
            <ChevronDown className="w-4 h-4 text-[hsl(var(--text-muted))]" />
          )}
        </button>

        <span
          className={cn(
            "text-sm font-medium text-[hsl(var(--text-primary))] truncate flex-1",
            onNavigateToSession &&
              "hover:text-[hsl(var(--accent-primary))] transition-colors cursor-pointer"
          )}
          title={`Plan: ${displayTitle}`}
          onClick={onNavigateToSession}
        >
          {displayTitle}
        </span>

        {/* Completed count badge */}
        <div
          className="flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium bg-[hsla(145,60%,45%,0.15)] text-[hsl(145,60%,45%)]"
          title={`${counts.done} of ${counts.total} completed`}
        >
          <Check className="w-3 h-3" />
          <span>{counts.done}</span>
        </div>
      </div>

      {/* Bottom row: progress bar */}
      <ProgressBar completed={counts.done} total={counts.total} />
    </div>
  );
});
