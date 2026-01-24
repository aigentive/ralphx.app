/**
 * ReviewCard - Displays a pending review with actions
 * Shows task title, review status, notes, and action buttons
 */
import type { Review, ReviewStatus } from "@/types/review";
import { ReviewStatusBadge } from "./ReviewStatusBadge";

interface ReviewCardProps {
  review: Review;
  taskTitle: string;
  fixAttempt?: number;
  maxFixAttempts?: number;
  onViewDiff?: (reviewId: string) => void;
  onApprove?: (reviewId: string) => void;
  onRequestChanges?: (reviewId: string) => void;
}

function ReviewerTypeIndicator({ type }: { type: "ai" | "human" }) {
  return (
    <span data-testid="reviewer-type-indicator" className="inline-flex items-center gap-1 text-xs" style={{ color: "var(--text-secondary)" }}>
      <span>{type === "ai" ? "🤖" : "👤"}</span>
      {type === "ai" ? "AI Review" : "Human Review"}
    </span>
  );
}

function FixAttemptCounter({ attempt, max }: { attempt: number; max: number }) {
  const atMax = attempt >= max;
  return (
    <span data-testid="fix-attempt-counter" data-at-max={atMax ? "true" : "false"} className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium" style={{ backgroundColor: atMax ? "var(--status-error)" : "var(--status-warning)", color: "var(--bg-base)" }}>
      Attempt {attempt} of {max}
    </span>
  );
}

const isPending = (s: ReviewStatus) => s === "pending";
const btnBase = "px-3 py-1.5 rounded text-sm font-medium transition-colors";

export function ReviewCard({ review, taskTitle, fixAttempt, maxFixAttempts, onViewDiff, onApprove, onRequestChanges }: ReviewCardProps) {
  const showActions = isPending(review.status);
  const showFixCounter = fixAttempt !== undefined && maxFixAttempts !== undefined;
  const hasButtons = onViewDiff || (showActions && (onApprove || onRequestChanges));

  return (
    <div data-testid={`review-card-${review.id}`} data-status={review.status} data-reviewer-type={review.reviewerType} className="p-4 rounded-lg border" style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}>
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div data-testid="review-task-title" className="font-medium truncate" style={{ color: "var(--text-primary)" }}>{taskTitle}</div>
          <div className="flex flex-wrap items-center gap-2 mt-2">
            <ReviewStatusBadge status={review.status} />
            <ReviewerTypeIndicator type={review.reviewerType} />
            {showFixCounter && <FixAttemptCounter attempt={fixAttempt} max={maxFixAttempts} />}
          </div>
        </div>
      </div>
      {review.notes && <p data-testid="review-notes" className="mt-3 text-sm" style={{ color: "var(--text-secondary)" }}>{review.notes}</p>}
      {hasButtons && (
        <div className="flex flex-wrap gap-2 mt-4">
          {onViewDiff && <button onClick={() => onViewDiff(review.id)} className={btnBase} style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}>View Diff</button>}
          {showActions && onApprove && <button onClick={() => onApprove(review.id)} className={btnBase} style={{ backgroundColor: "var(--status-success)", color: "var(--bg-base)" }}>Approve</button>}
          {showActions && onRequestChanges && <button onClick={() => onRequestChanges(review.id)} className={btnBase} style={{ backgroundColor: "var(--status-warning)", color: "var(--bg-base)" }}>Request Changes</button>}
        </div>
      )}
    </div>
  );
}
