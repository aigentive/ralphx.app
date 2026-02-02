/**
 * TimelineEntry.tsx - Individual event entry in the execution timeline
 *
 * Displays a single timeline event with:
 * - Timestamp (relative time)
 * - Task reference (if applicable)
 * - Event description
 * - Status color indicator using nodeStyles
 * - Clickable area for node interaction
 *
 * @see specs/plans/task_graph_view.md section "Task D.3"
 */

import { memo, useCallback, useMemo } from "react";
import { FileText, GitMerge, CheckCircle2, Circle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TimelineEvent, TimelineEventType } from "@/api/task-graph.types";
import { getNodeStyle, getStatusCategory } from "../nodes/nodeStyles";

// ============================================================================
// Types
// ============================================================================

export interface TimelineEntryProps {
  /** The timeline event to display */
  event: TimelineEvent;
  /** Callback when clicking on a task-related event (for node highlighting) */
  onTaskClick?: (taskId: string) => void;
  /** Whether this entry's associated task is currently highlighted */
  isHighlighted?: boolean;
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Format timestamp as relative time (e.g., "2m ago", "1h ago", "Yesterday")
 */
function formatRelativeTime(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay === 1) return "Yesterday";
  if (diffDay < 7) return `${diffDay}d ago`;

  // For older events, show the date
  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

/**
 * Format full timestamp for tooltip
 */
function formatFullTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/**
 * Get the icon element for the event type
 */
function EventIcon({
  eventType,
  style,
}: {
  eventType: TimelineEventType;
  style: { color: string };
}) {
  const className = "w-3 h-3";
  switch (eventType) {
    case "plan_accepted":
      return <FileText className={className} style={style} />;
    case "plan_completed":
      return <CheckCircle2 className={className} style={style} />;
    case "status_change":
    default:
      return <Circle className={className} style={style} />;
  }
}

// ============================================================================
// Component
// ============================================================================

export const TimelineEntry = memo(function TimelineEntry({
  event,
  onTaskClick,
  isHighlighted = false,
}: TimelineEntryProps) {
  // Get the style based on the event's status
  const statusStyle = useMemo(() => {
    // For status_change events, use the "to" status
    if (event.eventType === "status_change" && event.toStatus) {
      return getNodeStyle(event.toStatus);
    }
    // For plan events, use complete style for plan_completed, idle for accepted
    if (event.eventType === "plan_completed") {
      return getNodeStyle("approved");
    }
    return getNodeStyle("ready");
  }, [event.eventType, event.toStatus]);

  // Determine the status category for additional styling
  const statusCategory = useMemo(() => {
    if (event.eventType === "status_change" && event.toStatus) {
      return getStatusCategory(event.toStatus as Parameters<typeof getStatusCategory>[0]);
    }
    if (event.eventType === "plan_completed") {
      return "complete";
    }
    return "idle";
  }, [event.eventType, event.toStatus]);

  // Handle click for task-related events
  const handleClick = useCallback(() => {
    if (event.taskId && onTaskClick) {
      onTaskClick(event.taskId);
    }
  }, [event.taskId, onTaskClick]);

  const isClickable = Boolean(event.taskId && onTaskClick);

  return (
    <div
      className={cn(
        "group relative flex items-start gap-3 px-3 py-2 rounded-lg transition-colors",
        isClickable && "cursor-pointer hover:bg-white/5",
        isHighlighted && "bg-white/10"
      )}
      onClick={isClickable ? handleClick : undefined}
      role={isClickable ? "button" : undefined}
      tabIndex={isClickable ? 0 : undefined}
      onKeyDown={
        isClickable
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleClick();
              }
            }
          : undefined
      }
    >
      {/* Status indicator dot with color */}
      <div
        className={cn(
          "mt-1 flex-shrink-0 w-5 h-5 rounded-full flex items-center justify-center",
          "border transition-shadow",
          statusCategory === "executing" && "animate-pulse"
        )}
        style={{
          borderColor: statusStyle.borderColor,
          backgroundColor: statusStyle.backgroundColor,
          boxShadow: statusStyle.boxShadow,
        }}
      >
        <EventIcon
          eventType={event.eventType}
          style={{ color: statusStyle.borderColor }}
        />
      </div>

      {/* Event content */}
      <div className="flex-1 min-w-0">
        {/* Timestamp and description row */}
        <div className="flex items-baseline gap-2">
          <span
            className="text-xs text-muted-foreground flex-shrink-0"
            title={formatFullTimestamp(event.timestamp)}
          >
            {formatRelativeTime(event.timestamp)}
          </span>
          <span className="text-sm text-foreground/90 truncate">
            {event.description}
          </span>
        </div>

        {/* Task reference (if applicable) */}
        {event.taskTitle && event.eventType === "status_change" && (
          <div className="mt-0.5 text-xs text-muted-foreground truncate">
            {event.toStatus && (
              <span
                className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium"
                style={{
                  backgroundColor: statusStyle.backgroundColor,
                  color: statusStyle.borderColor,
                }}
              >
                {event.toStatus.replace(/_/g, " ")}
              </span>
            )}
          </div>
        )}

        {/* Plan context (for plan-level events) */}
        {event.sessionTitle && event.eventType !== "status_change" && (
          <div className="mt-0.5 flex items-center gap-1 text-xs text-muted-foreground">
            <GitMerge className="w-3 h-3" />
            <span className="truncate">{event.sessionTitle}</span>
          </div>
        )}
      </div>

      {/* Hover indicator for clickable items */}
      {isClickable && (
        <div className="absolute right-2 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity">
          <span className="text-[10px] text-muted-foreground">
            Click to focus
          </span>
        </div>
      )}
    </div>
  );
});

export default TimelineEntry;
