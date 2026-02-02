/**
 * PlanGroupHeader.tsx - Header for plan groups in the Task Graph
 *
 * Displays:
 * - Plan title (linked to planning session)
 * - Progress bar (completed / total)
 * - Status breakdown badges (done, executing, blocked, review)
 * - Collapse toggle and context menu buttons
 */

import { memo, useMemo } from "react";
import {
  ChevronDown,
  ChevronRight,
  MoreHorizontal,
  Check,
  Play,
  AlertTriangle,
  Eye,
  GitMerge,
} from "lucide-react";
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
// Status Badge Component
// ============================================================================

interface StatusBadgeProps {
  icon: React.ReactNode;
  count: number;
  label: string;
  colorClass: string;
}

const StatusBadge = memo(function StatusBadge({
  icon,
  count,
  label,
  colorClass,
}: StatusBadgeProps) {
  if (count === 0) return null;

  return (
    <div
      className={cn(
        "flex items-center gap-1 px-1.5 py-0.5 rounded text-xs font-medium",
        colorClass
      )}
      title={label}
    >
      {icon}
      <span>{count}</span>
    </div>
  );
});

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
  onContextMenu,
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
    <div className="flex items-center justify-between gap-3 px-3 py-2 bg-[hsl(var(--bg-elevated)/0.8)] rounded-t-lg">
      {/* Left section: collapse toggle + title */}
      <div className="flex items-center gap-2 min-w-0 flex-1">
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

        <button
          onClick={onNavigateToSession}
          className={cn(
            "text-sm font-medium text-[hsl(var(--text-primary))] truncate",
            onNavigateToSession &&
              "hover:text-[hsl(var(--accent-primary))] transition-colors cursor-pointer"
          )}
          title={`Plan: ${displayTitle}`}
          disabled={!onNavigateToSession}
        >
          {displayTitle}
        </button>
      </div>

      {/* Middle section: progress bar */}
      <ProgressBar completed={counts.done} total={counts.total} />

      {/* Status badges */}
      <div className="flex items-center gap-1.5">
        <StatusBadge
          icon={<Check className="w-3 h-3" />}
          count={counts.done}
          label={`${counts.done} completed`}
          colorClass="bg-[hsla(145,60%,45%,0.15)] text-[hsl(145,60%,45%)]"
        />
        <StatusBadge
          icon={<Play className="w-3 h-3" />}
          count={counts.executing}
          label={`${counts.executing} executing`}
          colorClass="bg-[hsla(14,100%,55%,0.15)] text-[hsl(14,100%,55%)]"
        />
        <StatusBadge
          icon={<AlertTriangle className="w-3 h-3" />}
          count={counts.blocked}
          label={`${counts.blocked} blocked`}
          colorClass="bg-[hsla(45,90%,55%,0.15)] text-[hsl(45,90%,55%)]"
        />
        <StatusBadge
          icon={<Eye className="w-3 h-3" />}
          count={counts.review}
          label={`${counts.review} in review`}
          colorClass="bg-[hsla(220,80%,60%,0.15)] text-[hsl(220,80%,60%)]"
        />
        <StatusBadge
          icon={<GitMerge className="w-3 h-3" />}
          count={counts.merge}
          label={`${counts.merge} merging`}
          colorClass="bg-[hsla(180,60%,50%,0.15)] text-[hsl(180,60%,50%)]"
        />
      </div>

      {/* Right section: context menu button */}
      {onContextMenu && (
        <button
          onClick={onContextMenu}
          className="flex-shrink-0 p-1 rounded hover:bg-[hsl(var(--bg-surface))] transition-colors"
          aria-label="Plan group options"
        >
          <MoreHorizontal className="w-4 h-4 text-[hsl(var(--text-muted))]" />
        </button>
      )}
    </div>
  );
});
