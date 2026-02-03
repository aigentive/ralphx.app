/**
 * PlanGroupHeader.tsx - Header for plan groups in the Task Graph
 *
 * Two layouts:
 * - Collapsed: two rows (title + count, progress bar)
 * - Expanded: single row inline (toggle, title, progress, badges)
 */

import { memo, useMemo } from "react";
import {
  ChevronDown,
  ChevronRight,
  Check,
  Play,
  AlertTriangle,
  Eye,
  GitMerge,
  ChevronsDownUp,
  ChevronsUpDown,
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
  /** Tier group IDs within this plan */
  tierGroupIds?: string[] | undefined;
  /** Whether any tiers are collapsed */
  anyTierCollapsed?: boolean | undefined;
  /** Whether all tiers are collapsed */
  allTiersCollapsed?: boolean | undefined;
  /** Toggle all tiers expand/collapse */
  onToggleAllTiers?: ((planArtifactId: string, action: "expand" | "collapse") => void) | undefined;
  /** Toggle collapse state */
  onToggleCollapse: () => void;
  /** Optional: Open context menu */
  onContextMenu?: () => void;
  /** Optional: Navigate to planning session */
  onNavigateToSession?: () => void;
}

// ============================================================================
// Reusable Components
// ============================================================================

/** Collapse/expand toggle button */
const CollapseToggle = memo(function CollapseToggle({
  isCollapsed,
  onClick,
}: {
  isCollapsed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex-shrink-0 p-0.5 rounded hover:bg-[hsl(var(--bg-surface))] transition-colors"
      aria-label={isCollapsed ? "Expand group" : "Collapse group"}
    >
      {isCollapsed ? (
        <ChevronRight className="w-4 h-4 text-[hsl(var(--text-muted))]" />
      ) : (
        <ChevronDown className="w-4 h-4 text-[hsl(var(--text-muted))]" />
      )}
    </button>
  );
});

/** Progress bar with percentage - thin and subtle */
const ProgressBar = memo(function ProgressBar({
  completed,
  total,
}: {
  completed: number;
  total: number;
}) {
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0;

  return (
    <div className="flex items-center gap-2 min-w-[120px]">
      <div className="flex-1 h-1 bg-[hsla(220,10%,20%,0.5)] rounded-full overflow-hidden">
        <div
          className="h-full bg-[hsla(145,60%,45%,0.6)] rounded-full transition-all duration-300"
          style={{ width: `${percentage}%` }}
        />
      </div>
      <span className="text-xs text-[hsl(var(--text-muted))] whitespace-nowrap">
        {percentage}%
      </span>
    </div>
  );
});

/** Status badge with icon and count */
const StatusBadge = memo(function StatusBadge({
  icon,
  count,
  label,
  colorClass,
}: {
  icon: React.ReactNode;
  count: number;
  label: string;
  colorClass: string;
}) {
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
// Main Component
// ============================================================================

export const PlanGroupHeader = memo(function PlanGroupHeader({
  planArtifactId,
  sessionTitle,
  taskCount,
  statusSummary,
  isCollapsed,
  tierGroupIds,
  anyTierCollapsed,
  allTiersCollapsed,
  onToggleAllTiers,
  onToggleCollapse,
  onNavigateToSession,
}: PlanGroupHeaderProps) {
  const counts = useMemo(() => ({
    done: statusSummary.completed,
    executing: statusSummary.executing,
    blocked: statusSummary.blocked,
    review: statusSummary.review,
    merge: statusSummary.merge,
    total: taskCount,
  }), [statusSummary, taskCount]);

  const displayTitle = sessionTitle || "Unnamed Plan";
  const hasTiers = (tierGroupIds?.length ?? 0) > 0;
  const shouldExpandAll = Boolean(anyTierCollapsed);
  const disableExpand = !anyTierCollapsed;
  const disableCollapse = Boolean(allTiersCollapsed);

  // Collapsed: two-row layout with text count
  if (isCollapsed) {
    return (
      <div
        className="flex flex-col gap-1.5 px-3 py-2 bg-[hsl(var(--bg-elevated)/0.8)] rounded-lg cursor-pointer"
        onDoubleClick={(e) => {
          e.stopPropagation();
          onToggleCollapse();
        }}
      >
        {/* Row 1: toggle + title + count */}
        <div className="flex items-center gap-2">
          <CollapseToggle isCollapsed={true} onClick={onToggleCollapse} />
          <span
            className="text-sm font-medium text-[hsl(var(--text-primary))] truncate flex-1"
            title={`Plan: ${displayTitle}`}
          >
            {displayTitle}
          </span>
          <span className="text-xs text-[hsl(var(--text-muted))]">
            {counts.total} tasks
          </span>
        </div>
        {/* Row 2: progress bar */}
        <ProgressBar completed={counts.done} total={counts.total} />
      </div>
    );
  }

  // Expanded: single-row inline layout
  return (
    <div
      className="flex items-center justify-between gap-3 px-3 py-2 bg-[hsl(var(--bg-elevated)/0.8)] rounded-t-lg cursor-pointer"
      onDoubleClick={(e) => {
        e.stopPropagation();
        onToggleCollapse();
      }}
    >
      {/* Left: toggle + title */}
      <div className="flex items-center gap-2 min-w-0 flex-1">
        <CollapseToggle isCollapsed={false} onClick={onToggleCollapse} />
        <span
          className={cn(
            "text-sm font-medium text-[hsl(var(--text-primary))] truncate",
            onNavigateToSession &&
              "hover:text-[hsl(var(--accent-primary))] transition-colors cursor-pointer"
          )}
          title={`Plan: ${displayTitle}`}
          onClick={onNavigateToSession}
        >
          {displayTitle}
        </span>
      </div>

      {/* Middle: progress bar */}
      <ProgressBar completed={counts.done} total={counts.total} />

      {/* Tier controls */}
      {hasTiers && (
        <div className="flex items-center gap-1.5">
          <button
            className={cn(
              "p-1 rounded transition-colors",
              "hover:bg-[hsl(var(--bg-surface))]",
              disableExpand && "opacity-50 pointer-events-none"
            )}
            onClick={(event) => {
              event.stopPropagation();
              onToggleAllTiers?.(
                planArtifactId,
                shouldExpandAll ? "expand" : "collapse"
              );
            }}
            aria-label={shouldExpandAll ? "Expand all tiers" : "Collapse all tiers"}
            title={shouldExpandAll ? "Expand all tiers" : "Collapse all tiers"}
          >
            {shouldExpandAll ? (
              <ChevronsDownUp className="w-3.5 h-3.5 text-[hsl(var(--text-muted))]" />
            ) : (
              <ChevronsUpDown className="w-3.5 h-3.5 text-[hsl(var(--text-muted))]" />
            )}
          </button>
          <button
            className={cn(
              "flex items-center gap-1 px-2 py-1 rounded text-[11px] text-[hsl(var(--text-muted))] transition-colors",
              "hover:bg-[hsl(var(--bg-surface))]",
              disableExpand && "opacity-50 pointer-events-none"
            )}
            onClick={(event) => {
              event.stopPropagation();
              onToggleAllTiers?.(planArtifactId, "expand");
            }}
            aria-label="Expand all tiers"
            title="Expand all tiers"
          >
            <ChevronsDownUp className="w-3 h-3" />
            Expand tiers
          </button>
          <button
            className={cn(
              "flex items-center gap-1 px-2 py-1 rounded text-[11px] text-[hsl(var(--text-muted))] transition-colors",
              "hover:bg-[hsl(var(--bg-surface))]",
              disableCollapse && "opacity-50 pointer-events-none"
            )}
            onClick={(event) => {
              event.stopPropagation();
              onToggleAllTiers?.(planArtifactId, "collapse");
            }}
            aria-label="Collapse all tiers"
            title="Collapse all tiers"
          >
            <ChevronsUpDown className="w-3 h-3" />
            Collapse tiers
          </button>
        </div>
      )}

      {/* Right: status badges */}
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
    </div>
  );
});
