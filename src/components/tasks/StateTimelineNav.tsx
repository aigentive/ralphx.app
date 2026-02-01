/**
 * StateTimelineNav - macOS Tahoe-inspired timeline navigation
 *
 * A beautiful horizontal timeline showing task state history.
 * Features:
 * - Vibrancy material background
 * - Smooth hover transitions
 * - Connected state dots with animated connectors
 * - Premium badge styling per status
 */

import { useMemo } from "react";
import { useTaskStateTransitions } from "@/hooks/useTaskStateTransitions";
import { Loader2, History, ChevronRight } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { InternalStatus } from "@/types/task";

// macOS Tahoe system colors (dark mode)
const STATUS_CONFIG: Record<
  InternalStatus,
  { label: string; color: string; bgColor: string }
> = {
  backlog: {
    label: "Backlog",
    color: "#8e8e93",
    bgColor: "rgba(142, 142, 147, 0.15)",
  },
  ready: {
    label: "Ready",
    color: "#0a84ff",
    bgColor: "rgba(10, 132, 255, 0.15)",
  },
  blocked: {
    label: "Blocked",
    color: "#ff9f0a",
    bgColor: "rgba(255, 159, 10, 0.15)",
  },
  executing: {
    label: "Executing",
    color: "#ff6b35",
    bgColor: "rgba(255, 107, 53, 0.15)",
  },
  qa_refining: {
    label: "QA Refining",
    color: "#ff6b35",
    bgColor: "rgba(255, 107, 53, 0.15)",
  },
  qa_testing: {
    label: "QA Testing",
    color: "#ff6b35",
    bgColor: "rgba(255, 107, 53, 0.15)",
  },
  qa_passed: {
    label: "QA Passed",
    color: "#34c759",
    bgColor: "rgba(52, 199, 89, 0.15)",
  },
  qa_failed: {
    label: "QA Failed",
    color: "#ff453a",
    bgColor: "rgba(255, 69, 58, 0.15)",
  },
  pending_review: {
    label: "Pending Review",
    color: "#8e8e93",
    bgColor: "rgba(142, 142, 147, 0.15)",
  },
  revision_needed: {
    label: "Revision Needed",
    color: "#ff9f0a",
    bgColor: "rgba(255, 159, 10, 0.15)",
  },
  approved: {
    label: "Approved",
    color: "#34c759",
    bgColor: "rgba(52, 199, 89, 0.15)",
  },
  failed: {
    label: "Failed",
    color: "#ff453a",
    bgColor: "rgba(255, 69, 58, 0.15)",
  },
  cancelled: {
    label: "Cancelled",
    color: "#8e8e93",
    bgColor: "rgba(142, 142, 147, 0.15)",
  },
  reviewing: {
    label: "Reviewing",
    color: "#0a84ff",
    bgColor: "rgba(10, 132, 255, 0.15)",
  },
  review_passed: {
    label: "Review Passed",
    color: "#34c759",
    bgColor: "rgba(52, 199, 89, 0.15)",
  },
  escalated: {
    label: "Escalated",
    color: "#ff9f0a",
    bgColor: "rgba(255, 159, 10, 0.15)",
  },
  re_executing: {
    label: "Re-executing",
    color: "#ff9f0a",
    bgColor: "rgba(255, 159, 10, 0.15)",
  },
  pending_merge: {
    label: "Pending Merge",
    color: "#ff6b35",
    bgColor: "rgba(255, 107, 53, 0.15)",
  },
  merging: {
    label: "Merging",
    color: "#ff6b35",
    bgColor: "rgba(255, 107, 53, 0.15)",
  },
  merge_conflict: {
    label: "Merge Conflict",
    color: "#ff9f0a",
    bgColor: "rgba(255, 159, 10, 0.15)",
  },
  merged: {
    label: "Merged",
    color: "#34c759",
    bgColor: "rgba(52, 199, 89, 0.15)",
  },
};

interface TimelineEntry {
  status: InternalStatus;
  timestamp: string;
  isCurrent: boolean;
  /** Conversation ID from the state transition metadata (for states that spawn conversations) */
  conversationId?: string | undefined;
  /** Agent run ID from the state transition metadata */
  agentRunId?: string | undefined;
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
  const isActive = isSelected || entry.isCurrent;

  return (
    <Tooltip delayDuration={200}>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onClick}
          data-testid={`timeline-badge-${entry.status}`}
          data-status={entry.status}
          data-current={entry.isCurrent}
          data-selected={isSelected}
          className="group relative flex items-center gap-2 px-3 py-1.5 rounded-full transition-all duration-200"
          style={{
            backgroundColor: isActive ? config.bgColor : "transparent",
            boxShadow: isSelected
              ? `0 0 0 2px ${config.color}50, 0 2px 8px ${config.color}30`
              : undefined,
          }}
        >
          {/* Status dot */}
          <div
            className="relative w-2 h-2 rounded-full shrink-0 transition-transform duration-200 group-hover:scale-125"
            style={{
              backgroundColor: config.color,
              boxShadow: isActive ? `0 0 8px ${config.color}60` : undefined,
            }}
          >
            {/* Pulse ring for current */}
            {entry.isCurrent && (
              <div
                className="absolute inset-0 rounded-full animate-ping"
                style={{
                  backgroundColor: config.color,
                  opacity: 0.4,
                }}
              />
            )}
          </div>

          {/* Label */}
          <span
            className="text-[11px] font-semibold tracking-tight transition-colors duration-200"
            style={{
              color: isActive ? config.color : "rgba(255,255,255,0.45)",
            }}
          >
            {config.label}
          </span>
        </button>
      </TooltipTrigger>
      <TooltipContent
        side="bottom"
        sideOffset={8}
        className="px-3 py-1.5 text-[11px] font-medium rounded-lg"
        style={{
          backgroundColor: "rgba(30, 30, 30, 0.95)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          border: "0.5px solid rgba(255,255,255,0.1)",
          color: "rgba(255,255,255,0.7)",
          boxShadow: "0 4px 16px rgba(0,0,0,0.3)",
        }}
      >
        <div className="flex items-center gap-2">
          <div
            className="w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: config.color }}
          />
          <span>{formatRelativeTime(entry.timestamp)}</span>
          {entry.isCurrent && (
            <span
              className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase"
              style={{
                backgroundColor: config.bgColor,
                color: config.color,
              }}
            >
              Current
            </span>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
}

interface TimelineConnectorProps {
  isActive: boolean;
  color: string;
}

function TimelineConnector({ isActive, color }: TimelineConnectorProps) {
  return (
    <div className="flex items-center px-0.5">
      <ChevronRight
        className="w-3.5 h-3.5 transition-colors duration-200"
        style={{
          color: isActive ? color : "rgba(255,255,255,0.15)",
        }}
      />
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export interface StateTimelineNavProps {
  taskId: string;
  currentStatus: InternalStatus;
  onStateSelect: (state: {
    status: InternalStatus;
    timestamp: string;
    conversationId?: string | undefined;
    agentRunId?: string | undefined;
  } | null) => void;
  selectedState?: {
    status: InternalStatus;
    timestamp: string;
    conversationId?: string | undefined;
    agentRunId?: string | undefined;
  } | null;
}

export function StateTimelineNav({
  taskId,
  currentStatus,
  onStateSelect,
  selectedState,
}: StateTimelineNavProps) {
  const { data: transitions, isLoading, error } = useTaskStateTransitions(taskId);

  // Derive unique timeline entries from transitions
  const timelineEntries = useMemo((): TimelineEntry[] => {
    // Transient states to skip in the timeline
    // - ready: brief transition between draft and executing
    // - pending_review: brief wait for AI reviewer
    // - reviewing: AI review in progress (info shown in review_passed)
    const transientStatuses: InternalStatus[] = ["ready", "pending_review", "reviewing"];

    if (!transitions || transitions.length === 0) {
      // Don't show timeline for transient states with no history
      if (transientStatuses.includes(currentStatus)) {
        return [];
      }
      return [
        {
          status: currentStatus,
          timestamp: new Date().toISOString(),
          isCurrent: true,
        },
      ];
    }

    const entries: TimelineEntry[] = [];
    const seenStatuses = new Set<InternalStatus>();

    for (const transition of transitions) {
      // Skip transient states - they're brief transitions not worth showing
      if (transientStatuses.includes(transition.toStatus)) {
        continue;
      }
      if (seenStatuses.has(transition.toStatus)) {
        continue;
      }
      seenStatuses.add(transition.toStatus);

      entries.push({
        status: transition.toStatus,
        timestamp: transition.timestamp,
        isCurrent: transition.toStatus === currentStatus,
        conversationId: transition.conversationId,
        agentRunId: transition.agentRunId,
      });
    }

    const lastEntry = entries[entries.length - 1];
    if (lastEntry && lastEntry.status !== currentStatus) {
      const existingEntry = entries.find((e) => e.status === currentStatus);
      if (existingEntry) {
        existingEntry.isCurrent = true;
      }
    }

    return entries;
  }, [transitions, currentStatus]);

  // Handle badge click
  const handleBadgeClick = (entry: TimelineEntry) => {
    if (entry.isCurrent) {
      onStateSelect(null);
    } else {
      onStateSelect({
        status: entry.status,
        timestamp: entry.timestamp,
        conversationId: entry.conversationId,
        agentRunId: entry.agentRunId,
      });
    }
  };

  // Loading state
  if (isLoading) {
    return (
      <div
        data-testid="timeline-loading"
        className="flex items-center gap-2.5 px-4 py-3"
        style={{ color: "rgba(255,255,255,0.4)" }}
      >
        <Loader2 className="w-4 h-4 animate-spin" />
        <span className="text-[11px] font-medium">Loading history...</span>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        data-testid="timeline-error"
        className="flex items-center gap-2 px-4 py-3 text-[11px] font-medium"
        style={{ color: "#ff453a" }}
      >
        <History className="w-4 h-4" />
        <span>Failed to load history</span>
      </div>
    );
  }

  // Hide if single state
  if (timelineEntries.length <= 1) {
    return null;
  }

  // Get the color of the last active entry for connectors
  const selectedIndex = selectedState
    ? timelineEntries.findIndex(
        (e) =>
          e.status === selectedState.status &&
          e.timestamp === selectedState.timestamp
      )
    : -1;

  return (
    <TooltipProvider delayDuration={200}>
      <div
        data-testid="state-timeline-nav"
        className="flex items-center gap-1 px-4 py-3 overflow-x-auto"
        style={{
          backgroundColor: "rgba(20, 20, 20, 0.6)",
          backdropFilter: "blur(40px) saturate(150%)",
          WebkitBackdropFilter: "blur(40px) saturate(150%)",
          borderBottom: "0.5px solid rgba(255,255,255,0.06)",
        }}
      >
        {/* History icon */}
        <div
          className="flex items-center gap-2 mr-2 pr-3"
          style={{ borderRight: "1px solid rgba(255,255,255,0.08)" }}
        >
          <History
            className="w-4 h-4"
            style={{ color: "rgba(255,255,255,0.35)" }}
          />
          <span
            className="text-[10px] font-semibold uppercase tracking-wider"
            style={{ color: "rgba(255,255,255,0.35)" }}
          >
            History
          </span>
        </div>

        {/* Timeline entries */}
        {timelineEntries.map((entry, index) => {
          const isConnectorActive =
            selectedState === null || (selectedIndex !== -1 && index < selectedIndex);
          const nextEntry = timelineEntries[index + 1];

          return (
            <div key={`${entry.status}-${entry.timestamp}`} className="flex items-center">
              <TimelineBadge
                entry={entry}
                isSelected={
                  selectedState?.status === entry.status &&
                  selectedState?.timestamp === entry.timestamp
                }
                onClick={() => handleBadgeClick(entry)}
              />
              {nextEntry && (
                <TimelineConnector
                  isActive={isConnectorActive}
                  color={STATUS_CONFIG[entry.status].color}
                />
              )}
            </div>
          );
        })}

        {/* Viewing historical state indicator */}
        {selectedState && (
          <div
            className="ml-auto pl-3 flex items-center gap-2"
            style={{ borderLeft: "1px solid rgba(255,255,255,0.08)" }}
          >
            <span
              className="text-[10px] font-medium"
              style={{ color: "rgba(255,255,255,0.4)" }}
            >
              Viewing past state
            </span>
            <button
              onClick={() => onStateSelect(null)}
              className="px-2 py-1 rounded-md text-[10px] font-semibold transition-colors"
              style={{
                backgroundColor: "rgba(255, 107, 53, 0.15)",
                color: "#ff8050",
              }}
            >
              Back to Current
            </button>
          </div>
        )}
      </div>
    </TooltipProvider>
  );
}
