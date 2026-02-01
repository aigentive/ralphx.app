/**
 * StateTimelineNav - Horizontal timeline navigation for task state history
 *
 * Displays a clickable timeline of all states a task has been through,
 * enabling users to "time travel" and view historical task states.
 *
 * Part of Phase 59: State Time Travel feature.
 */

import { useMemo } from "react";
import { useTaskStateTransitions } from "@/hooks/useTaskStateTransitions";
import { Loader2, Clock, Circle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { InternalStatus } from "@/types/task";

// Status badge configuration - matches TaskDetailOverlay.tsx
const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; bg: string; text: string }
> = {
  backlog: {
    label: "Backlog",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  ready: {
    label: "Ready",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  blocked: {
    label: "Blocked",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  executing: {
    label: "Executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_refining: {
    label: "QA Refining",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_testing: {
    label: "QA Testing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
  qa_passed: {
    label: "QA Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  qa_failed: {
    label: "QA Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  pending_review: {
    label: "Pending Review",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  revision_needed: {
    label: "Revision Needed",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  approved: {
    label: "Approved",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  failed: {
    label: "Failed",
    bg: "rgba(239, 68, 68, 0.15)",
    text: "var(--status-error)",
  },
  cancelled: {
    label: "Cancelled",
    bg: "var(--bg-hover)",
    text: "var(--text-muted)",
  },
  reviewing: {
    label: "Reviewing",
    bg: "rgba(59, 130, 246, 0.15)",
    text: "var(--status-info)",
  },
  review_passed: {
    label: "Review Passed",
    bg: "rgba(16, 185, 129, 0.15)",
    text: "var(--status-success)",
  },
  escalated: {
    label: "Escalated",
    bg: "rgba(245, 158, 11, 0.15)",
    text: "var(--status-warning)",
  },
  re_executing: {
    label: "Re-executing",
    bg: "rgba(255, 107, 53, 0.15)",
    text: "var(--accent-primary)",
  },
};

/**
 * A unique state entry in the timeline
 * Derived from transitions - each toStatus becomes an entry
 */
interface TimelineEntry {
  status: InternalStatus;
  timestamp: string;
  isCurrent: boolean;
}

function formatRelativeTime(dateString: string): string {
  const diff = Date.now() - new Date(dateString).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "Just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

// ============================================================================
// Sub-components
// ============================================================================

interface TimelineBadgeProps {
  entry: TimelineEntry;
  isSelected: boolean;
  onClick: () => void;
}

function TimelineBadge({ entry, isSelected, onClick }: TimelineBadgeProps) {
  const config = STATUS_CONFIG[entry.status];
  const isHighlighted = isSelected || entry.isCurrent;

  return (
    <Tooltip delayDuration={300}>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onClick}
          data-testid={`timeline-badge-${entry.status}`}
          data-status={entry.status}
          data-current={entry.isCurrent}
          data-selected={isSelected}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition-all"
          style={{
            backgroundColor: isHighlighted ? config.bg : "transparent",
            color: config.text,
            border: `1px solid ${isHighlighted ? "transparent" : "rgba(255,255,255,0.1)"}`,
            boxShadow: isSelected ? `0 0 0 2px ${config.text}40` : undefined,
            opacity: isHighlighted ? 1 : 0.7,
          }}
        >
          {/* Status indicator dot */}
          {entry.isCurrent ? (
            <Circle className="w-2 h-2 fill-current" />
          ) : (
            <Circle className="w-2 h-2" style={{ opacity: 0.5 }} />
          )}

          {/* Label */}
          <span>{config.label}</span>
        </button>
      </TooltipTrigger>
      <TooltipContent
        side="bottom"
        className="px-2 py-1 text-[10px]"
        style={{
          backgroundColor: "var(--bg-elevated)",
          color: "var(--text-secondary)",
          border: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        {formatRelativeTime(entry.timestamp)}
      </TooltipContent>
    </Tooltip>
  );
}

interface TimelineConnectorProps {
  isActive: boolean;
}

function TimelineConnector({ isActive }: TimelineConnectorProps) {
  return (
    <div
      className="w-4 h-0.5 shrink-0"
      style={{
        backgroundColor: isActive ? "var(--accent-primary)" : "rgba(255,255,255,0.1)",
      }}
    />
  );
}

// ============================================================================
// Main Component
// ============================================================================

export interface StateTimelineNavProps {
  taskId: string;
  currentStatus: InternalStatus;
  onStateSelect: (state: { status: InternalStatus; timestamp: string } | null) => void;
  selectedState?: { status: InternalStatus; timestamp: string } | null;
}

export function StateTimelineNav({
  taskId,
  currentStatus,
  onStateSelect,
  selectedState,
}: StateTimelineNavProps) {
  const { data: transitions, isLoading, error } = useTaskStateTransitions(taskId);

  console.log('[StateTimelineNav] Render:', { taskId, currentStatus, transitions, isLoading, error });

  // Derive unique timeline entries from transitions
  // Each toStatus becomes an entry, preserving chronological order
  const timelineEntries = useMemo((): TimelineEntry[] => {
    if (!transitions || transitions.length === 0) {
      // If no transitions, show just the current status
      return [
        {
          status: currentStatus,
          timestamp: new Date().toISOString(),
          isCurrent: true,
        },
      ];
    }

    // Build entries from transitions - use toStatus from each
    const entries: TimelineEntry[] = [];
    const seenStatuses = new Set<InternalStatus>();

    for (const transition of transitions) {
      // Skip if we've already seen this status (show only first occurrence)
      if (seenStatuses.has(transition.toStatus)) {
        continue;
      }
      seenStatuses.add(transition.toStatus);

      entries.push({
        status: transition.toStatus,
        timestamp: transition.timestamp,
        isCurrent: transition.toStatus === currentStatus,
      });
    }

    // Ensure current status is marked (in case last transition doesn't match)
    const lastEntry = entries[entries.length - 1];
    if (lastEntry && lastEntry.status !== currentStatus) {
      // Current status might be different from last transition
      // This can happen if we filtered duplicates
      const existingEntry = entries.find((e) => e.status === currentStatus);
      if (existingEntry) {
        existingEntry.isCurrent = true;
      }
    }

    return entries;
  }, [transitions, currentStatus]);

  // Handle badge click
  const handleBadgeClick = (entry: TimelineEntry) => {
    console.log('[StateTimelineNav] Badge clicked:', entry);
    if (entry.isCurrent) {
      // Clicking current state exits history mode
      console.log('[StateTimelineNav] Exiting history mode (clicked current)');
      onStateSelect(null);
    } else {
      // Clicking historical state enters history mode
      console.log('[StateTimelineNav] Entering history mode:', { status: entry.status, timestamp: entry.timestamp });
      onStateSelect({ status: entry.status, timestamp: entry.timestamp });
    }
  };

  // Loading state
  if (isLoading) {
    return (
      <div
        data-testid="timeline-loading"
        className="flex items-center gap-2 px-4 py-2"
        style={{ color: "var(--text-muted)" }}
      >
        <Loader2 className="w-4 h-4 animate-spin" />
        <span className="text-xs">Loading history...</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        data-testid="timeline-error"
        className="flex items-center gap-2 px-4 py-2 text-xs"
        style={{ color: "var(--status-error)" }}
      >
        <Clock className="w-4 h-4" />
        <span>Failed to load history</span>
      </div>
    );
  }

  // Empty state - shouldn't happen but handle gracefully
  if (timelineEntries.length === 0) {
    return null;
  }

  // Single state - no need for timeline navigation
  if (timelineEntries.length === 1) {
    return null;
  }

  return (
    <TooltipProvider delayDuration={300}>
      <div
        data-testid="state-timeline-nav"
        className="flex items-center gap-1 px-4 py-3 overflow-x-auto"
        style={{
          backgroundColor: "rgba(0,0,0,0.2)",
          borderBottom: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <Clock
          className="w-4 h-4 shrink-0 mr-2"
          style={{ color: "var(--text-muted)" }}
        />

        {timelineEntries.map((entry, index) => (
          <div key={`${entry.status}-${entry.timestamp}`} className="flex items-center">
            <TimelineBadge
              entry={entry}
              isSelected={
                selectedState?.status === entry.status &&
                selectedState?.timestamp === entry.timestamp
              }
              onClick={() => handleBadgeClick(entry)}
            />
            {index < timelineEntries.length - 1 && (
              <TimelineConnector
                isActive={
                  selectedState
                    ? index < timelineEntries.findIndex(
                        (e) =>
                          e.status === selectedState.status &&
                          e.timestamp === selectedState.timestamp
                      )
                    : true
                }
              />
            )}
          </div>
        ))}
      </div>
    </TooltipProvider>
  );
}
