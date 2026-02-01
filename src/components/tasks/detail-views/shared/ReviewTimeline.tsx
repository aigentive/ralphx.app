/**
 * ReviewTimeline - macOS Tahoe-inspired review history timeline
 *
 * Shows a vertical timeline of review events with native styling.
 */

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { CheckCircle2, RotateCcw, Bot, User } from "lucide-react";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import type { ReviewNoteResponse } from "@/lib/tauri";

export interface ReviewTimelineProps {
  history: ReviewNoteResponse[];
  filter?: (entry: ReviewNoteResponse) => boolean;
  emptyMessage?: string;
  showAttemptNumbers?: boolean;
}

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

interface TimelineItemProps {
  entry: ReviewNoteResponse;
  isLast: boolean;
  attemptNumber?: number;
}

function TimelineItem({ entry, isLast, attemptNumber }: TimelineItemProps) {
  const isApproved = entry.outcome === "approved";
  const isChangesRequested = entry.outcome === "changes_requested";
  const isHuman = entry.reviewer === "human";

  const getConfig = () => {
    if (isApproved) {
      return {
        Icon: CheckCircle2,
        color: "#34c759",
        bgColor: "rgba(52, 199, 89, 0.15)",
        lineColor: "rgba(52, 199, 89, 0.3)",
      };
    }
    if (isChangesRequested) {
      return {
        Icon: RotateCcw,
        color: "#ff9f0a",
        bgColor: "rgba(255, 159, 10, 0.15)",
        lineColor: "rgba(255, 159, 10, 0.3)",
      };
    }
    return {
      Icon: CheckCircle2,
      color: "#8e8e93",
      bgColor: "rgba(142, 142, 147, 0.15)",
      lineColor: "rgba(142, 142, 147, 0.2)",
    };
  };

  const config = getConfig();
  const ReviewerIcon = isHuman ? User : Bot;

  const getLabel = () => {
    if (attemptNumber !== undefined && isChangesRequested) {
      return `Attempt #${attemptNumber}: Changes requested`;
    }
    if (isApproved) {
      return `${isHuman ? "Human" : "AI"} approved`;
    }
    if (isChangesRequested) {
      return `${isHuman ? "Human" : "AI"} requested changes`;
    }
    return `${isHuman ? "Human" : "AI"} reviewed`;
  };

  return (
    <div className="flex gap-3">
      {/* Timeline connector */}
      <div className="flex flex-col items-center">
        {/* Icon circle */}
        <div
          className="flex items-center justify-center w-7 h-7 rounded-xl shrink-0"
          style={{ backgroundColor: config.bgColor }}
        >
          <config.Icon className="w-4 h-4" style={{ color: config.color }} />
        </div>
        {/* Vertical line */}
        {!isLast && (
          <div
            className="w-0.5 flex-1 min-h-[20px] mt-1"
            style={{ backgroundColor: config.lineColor }}
          />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 pb-4">
        <div className="flex items-center gap-2">
          <ReviewerIcon
            className="w-3.5 h-3.5"
            style={{ color: isHuman ? "#34c759" : "#0a84ff" }}
          />
          <span className="text-[12px] font-semibold text-white/75">
            {getLabel()}
          </span>
          <span className="text-[11px] text-white/40 ml-auto">
            {formatRelativeTime(entry.created_at)}
          </span>
        </div>
        {entry.notes && (
          <div className="text-[12px] text-white/50 mt-1.5 pl-5 leading-relaxed prose prose-sm prose-invert max-w-none">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={markdownComponents}
            >
              {entry.notes}
            </ReactMarkdown>
          </div>
        )}
      </div>
    </div>
  );
}

export function ReviewTimeline({
  history,
  filter,
  emptyMessage = "No review history available",
  showAttemptNumbers = false,
}: ReviewTimelineProps) {
  const displayedHistory = filter ? history.filter(filter) : history;

  if (displayedHistory.length === 0) {
    return (
      <p className="text-[12px] text-white/35 italic py-2">
        {emptyMessage}
      </p>
    );
  }

  return (
    <div data-testid="review-history-timeline">
      {displayedHistory.map((entry, index) => (
        <TimelineItem
          key={entry.id}
          entry={entry}
          isLast={index === displayedHistory.length - 1}
          {...(showAttemptNumbers && { attemptNumber: index + 1 })}
        />
      ))}
    </div>
  );
}
