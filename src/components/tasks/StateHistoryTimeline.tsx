/**
 * StateHistoryTimeline - Displays task state transition history
 * Shows a vertical timeline with status changes, actors, and notes
 * Max 80 lines per PRD requirements
 */

import { useTaskStateHistory } from "@/hooks/useReviews";
import type { ReviewOutcome, ReviewerType } from "@/types/review";

interface StateHistoryTimelineProps {
  taskId: string;
}

const OUTCOME_CONFIG: Record<ReviewOutcome, { label: string; color: string }> = {
  approved: { label: "Approved", color: "var(--status-success)" },
  changes_requested: { label: "Changes Requested", color: "var(--status-warning)" },
  rejected: { label: "Rejected", color: "var(--status-error)" },
};

function mapReviewerToActor(reviewer: ReviewerType): string {
  return reviewer === "human" ? "user" : "ai_reviewer";
}

function formatRelativeTime(dateString: string): string {
  const diff = Date.now() - new Date(dateString).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export function StateHistoryTimeline({ taskId }: StateHistoryTimelineProps) {
  const { data, isLoading, isEmpty } = useTaskStateHistory(taskId);

  if (isLoading) {
    return (
      <div data-testid="timeline-loading" className="flex justify-center p-4">
        <div className="animate-spin rounded-full h-6 w-6 border-2 border-current border-t-transparent" style={{ color: "var(--text-muted)" }} />
      </div>
    );
  }

  if (isEmpty) {
    return (
      <div data-testid="timeline-empty" className="text-center p-4" style={{ color: "var(--text-muted)" }}>
        No history
      </div>
    );
  }

  return (
    <div data-testid="timeline-container" className="p-4 rounded-md" style={{ backgroundColor: "var(--bg-surface)" }}>
      <div className="space-y-4">
        {data.map((entry) => {
          const config = OUTCOME_CONFIG[entry.outcome];
          const relativeTime = formatRelativeTime(entry.created_at);
          return (
            <div key={entry.id} data-testid={`timeline-entry-${entry.id}`} data-timestamp={entry.created_at} className="relative pl-5">
              <div data-testid={`timeline-dot-${entry.id}`} className="absolute left-0 top-1 w-2.5 h-2.5 rounded-full" style={{ backgroundColor: config.color }} />
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium" style={{ color: "var(--text-primary)" }}>{config.label}</span>
                <span className="text-xs" style={{ color: "var(--text-muted)" }}>{relativeTime}</span>
              </div>
              <div className="text-xs mt-0.5" style={{ color: "var(--text-secondary)" }}>by: {mapReviewerToActor(entry.reviewer)}</div>
              {entry.notes && (
                <div className="text-xs mt-1 italic" style={{ color: "var(--text-secondary)" }}>"{entry.notes}"</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
