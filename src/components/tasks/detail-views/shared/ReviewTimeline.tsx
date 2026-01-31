/**
 * ReviewTimeline - Shared review history timeline component
 *
 * Extracted from CompletedTaskDetail for use in both CompletedTaskDetail
 * and EscalatedTaskDetail with configurable filtering.
 *
 * Usage:
 * - CompletedTaskDetail: <ReviewTimeline history={history} /> (shows all)
 * - EscalatedTaskDetail: <ReviewTimeline history={history} filter={(e) => e.outcome === "changes_requested"} showAttemptNumbers />
 */

import { CheckCircle2, RotateCcw, Bot, User } from "lucide-react";
import type { ReviewNoteResponse } from "@/lib/tauri";

export interface ReviewTimelineProps {
  history: ReviewNoteResponse[];
  /** Filter function to select which entries to display */
  filter?: (entry: ReviewNoteResponse) => boolean;
  /** Message to display when filtered list is empty */
  emptyMessage?: string;
  /** Show attempt numbers (#1, #2) for filtered entries - used in EscalatedTaskDetail */
  showAttemptNumbers?: boolean;
}

/**
 * Format relative time from date
 */
function formatRelativeTime(date: Date | string | undefined): string {
  if (!date) return "Unknown";

  const now = new Date();
  const then = typeof date === "string" ? new Date(date) : date;
  const diffMs = now.getTime() - then.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

/**
 * HistoryTimelineItem - Individual item in the review history timeline
 */
function HistoryTimelineItem({
  entry,
  isLast,
  attemptNumber,
}: {
  entry: ReviewNoteResponse;
  isLast: boolean;
  attemptNumber?: number;
}) {
  const isApproved = entry.outcome === "approved";
  const isChangesRequested = entry.outcome === "changes_requested";
  const isHuman = entry.reviewer === "human";

  const getIconAndColor = () => {
    if (isApproved) {
      return {
        Icon: CheckCircle2,
        color: "var(--status-success)",
        bgColor: "rgba(16, 185, 129, 0.15)",
      };
    }
    if (isChangesRequested) {
      return {
        Icon: RotateCcw,
        color: "var(--status-warning)",
        bgColor: "rgba(245, 158, 11, 0.15)",
      };
    }
    return {
      Icon: CheckCircle2,
      color: "rgba(255,255,255,0.5)",
      bgColor: "rgba(255,255,255,0.08)",
    };
  };

  const { Icon, color, bgColor } = getIconAndColor();
  const ReviewerIcon = isHuman ? User : Bot;

  const getLabel = () => {
    // When showing attempt numbers, use "Attempt #N: Changes requested" format
    if (attemptNumber !== undefined && isChangesRequested) {
      return `Attempt #${attemptNumber}: Changes requested`;
    }
    if (isApproved) {
      return `${isHuman ? "Human" : "AI"} approved`;
    }
    if (isChangesRequested) {
      return `${isHuman ? "Human" : "AI"} changes requested`;
    }
    return `${isHuman ? "Human" : "AI"} reviewed`;
  };

  return (
    <div className="flex gap-3">
      {/* Timeline line and dot */}
      <div className="flex flex-col items-center">
        <div
          className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
          style={{ backgroundColor: bgColor }}
        >
          <Icon className="w-3.5 h-3.5" style={{ color }} />
        </div>
        {!isLast && (
          <div
            className="w-px flex-1 min-h-[16px]"
            style={{ backgroundColor: "rgba(255,255,255,0.1)" }}
          />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 pb-3">
        <div className="flex items-center gap-2">
          <ReviewerIcon
            className="w-3.5 h-3.5"
            style={{ color: "rgba(255,255,255,0.5)" }}
          />
          <span className="text-[12px] font-medium text-white/70">
            {getLabel()}
          </span>
          <span className="text-[11px] text-white/40">
            {formatRelativeTime(entry.created_at)}
          </span>
        </div>
        {entry.notes && (
          <p className="text-[11px] text-white/50 mt-1 pl-5">
            {entry.notes}
          </p>
        )}
      </div>
    </div>
  );
}

/**
 * ReviewTimeline - Shows timeline of review events with optional filtering
 */
export function ReviewTimeline({
  history,
  filter,
  emptyMessage = "No review history available",
  showAttemptNumbers = false,
}: ReviewTimelineProps) {
  // Apply filter if provided
  const displayedHistory = filter ? history.filter(filter) : history;

  if (displayedHistory.length === 0) {
    return (
      <p className="text-[12px] text-white/40 italic">
        {emptyMessage}
      </p>
    );
  }

  return (
    <div data-testid="review-history-timeline">
      {displayedHistory.map((entry, index) => (
        <HistoryTimelineItem
          key={entry.id}
          entry={entry}
          isLast={index === displayedHistory.length - 1}
          {...(showAttemptNumbers && { attemptNumber: index + 1 })}
        />
      ))}
    </div>
  );
}
