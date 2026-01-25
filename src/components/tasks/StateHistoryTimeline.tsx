/**
 * StateHistoryTimeline - Displays task state transition history
 * Shows a vertical timeline with connected dots, status changes, actors, and notes
 * Premium styling with ring effect on dots and connected lines
 */

import { useTaskStateHistory } from "@/hooks/useReviews";
import { History, Loader2 } from "lucide-react";
import type { ReviewOutcome, ReviewerType } from "@/types/review";

interface StateHistoryTimelineProps {
  taskId: string;
}

const OUTCOME_CONFIG: Record<
  ReviewOutcome,
  { label: string; color: string }
> = {
  approved: { label: "Approved", color: "var(--status-success)" },
  changes_requested: { label: "Changes Requested", color: "var(--status-warning)" },
  rejected: { label: "Rejected", color: "var(--status-error)" },
};

function mapReviewerToActor(reviewer: ReviewerType): string {
  switch (reviewer) {
    case "human":
      return "Human Reviewer";
    case "ai":
      return "AI Reviewer";
    default:
      return "System";
  }
}

function formatRelativeTime(dateString: string): string {
  const diff = Date.now() - new Date(dateString).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "Just now";
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function StateHistoryTimeline({ taskId }: StateHistoryTimelineProps) {
  const { data, isLoading, isEmpty } = useTaskStateHistory(taskId);

  if (isLoading) {
    return (
      <div data-testid="timeline-loading" className="flex justify-center py-8">
        <Loader2
          className="w-6 h-6 animate-spin"
          style={{ color: "var(--text-muted)" }}
        />
      </div>
    );
  }

  if (isEmpty) {
    return (
      <div
        data-testid="timeline-empty"
        className="flex flex-col items-center justify-center py-8 text-center"
      >
        <History
          className="w-8 h-8 mb-2"
          style={{ color: "var(--text-muted)", opacity: 0.5 }}
        />
        <p className="text-sm" style={{ color: "var(--text-muted)" }}>
          No history yet
        </p>
        <p className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
          Status changes will appear here
        </p>
      </div>
    );
  }

  return (
    <div
      data-testid="timeline-container"
      className="p-4 rounded-lg"
      style={{ backgroundColor: "var(--bg-surface)" }}
    >
      <div className="relative">
        {data.map((entry, index) => {
          const config = OUTCOME_CONFIG[entry.outcome];
          const relativeTime = formatRelativeTime(entry.created_at);
          const isLatest = index === 0;
          const isLast = index === data.length - 1;

          return (
            <div
              key={entry.id}
              data-testid={`timeline-entry-${entry.id}`}
              data-timestamp={entry.created_at}
              className="relative pl-6"
              style={{ paddingBottom: isLast ? 0 : "16px" }}
            >
              {/* Vertical connector line */}
              {!isLast && (
                <div
                  className="absolute w-0.5"
                  style={{
                    left: "5px",
                    top: isLatest ? "20px" : "12px",
                    bottom: 0,
                    backgroundColor: "var(--border-subtle)",
                  }}
                />
              )}

              {/* Status dot with ring effect */}
              <div
                data-testid={`timeline-dot-${entry.id}`}
                className="absolute rounded-full"
                style={{
                  left: isLatest ? "-2px" : 0,
                  top: isLatest ? "2px" : "4px",
                  width: isLatest ? "16px" : "12px",
                  height: isLatest ? "16px" : "12px",
                  backgroundColor: config.color,
                  border: isLatest ? "none" : "2px solid var(--bg-elevated)",
                  boxShadow: isLatest
                    ? `0 0 0 4px ${config.color}33`
                    : undefined,
                }}
              />

              {/* Content */}
              <div className="flex items-center justify-between gap-2">
                <span
                  className="font-medium text-sm"
                  style={{ color: "var(--text-primary)" }}
                >
                  {config.label}
                </span>
                <span
                  className="text-xs"
                  style={{ color: "var(--text-muted)" }}
                >
                  {relativeTime}
                </span>
              </div>
              <div
                className="text-xs mt-0.5"
                style={{ color: "var(--text-secondary)" }}
              >
                by: {mapReviewerToActor(entry.reviewer)}
              </div>
              {entry.notes && (
                <div
                  className="text-xs mt-1 italic"
                  style={{ color: "var(--text-secondary)" }}
                >
                  "{entry.notes}"
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
