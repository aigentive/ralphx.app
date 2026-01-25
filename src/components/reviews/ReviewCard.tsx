/**
 * ReviewCard - Displays a pending review with premium styling
 * Features:
 * - Hover lift animation
 * - Lucide icons for reviewer type
 * - Notes preview with "View Full" link
 * - Action buttons with proper styling
 */
import { useState } from "react";
import { Bot, User, GitCompare, Loader2 } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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
  onViewFullNotes?: (reviewId: string) => void;
  isLoading?: boolean;
}

function ReviewerTypeIndicator({ type }: { type: "ai" | "human" }) {
  const Icon = type === "ai" ? Bot : User;
  return (
    <span
      data-testid="reviewer-type-indicator"
      className="inline-flex items-center gap-1 text-xs text-[var(--text-secondary)]"
    >
      <Icon className="w-4 h-4" />
      {type === "ai" ? "AI Review" : "Human Review"}
    </span>
  );
}

function FixAttemptCounter({ attempt, max }: { attempt: number; max: number }) {
  const atMax = attempt >= max;
  return (
    <span
      data-testid="fix-attempt-counter"
      data-at-max={atMax ? "true" : "false"}
      className={cn(
        "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
        atMax
          ? "bg-[var(--status-error)] text-white"
          : "bg-[var(--status-warning)] text-[var(--bg-base)]"
      )}
    >
      Attempt {attempt} of {max}
    </span>
  );
}

function NotesPreview({
  notes,
  onViewFull,
}: {
  notes: string;
  onViewFull?: (() => void) | undefined;
}) {
  const isLong = notes.length > 100;
  return (
    <div className="mt-3">
      <div
        className={cn(
          "p-2 rounded-[var(--radius-sm)]",
          "bg-[var(--bg-base)]"
        )}
      >
        <p
          data-testid="review-notes"
          className="text-sm text-[var(--text-secondary)] italic line-clamp-2 leading-normal"
        >
          &ldquo;{notes}&rdquo;
        </p>
      </div>
      {isLong && onViewFull && (
        <button
          onClick={onViewFull}
          className="text-xs text-[var(--accent-primary)] hover:underline mt-1"
        >
          View Full
        </button>
      )}
    </div>
  );
}

const isPending = (s: ReviewStatus) => s === "pending";

export function ReviewCard({
  review,
  taskTitle,
  fixAttempt,
  maxFixAttempts,
  onViewDiff,
  onApprove,
  onRequestChanges,
  onViewFullNotes,
  isLoading = false,
}: ReviewCardProps) {
  const [isHovered, setIsHovered] = useState(false);
  const showActions = isPending(review.status);
  const showFixCounter =
    fixAttempt !== undefined && maxFixAttempts !== undefined;
  const hasButtons =
    onViewDiff || (showActions && (onApprove || onRequestChanges));

  return (
    <Card
      data-testid={`review-card-${review.id}`}
      data-status={review.status}
      data-reviewer-type={review.reviewerType}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={cn(
        "p-4 border transition-all duration-150 ease-out",
        "bg-[var(--bg-elevated)] border-[var(--border-subtle)]",
        "rounded-[var(--radius-md)]",
        isHovered && "shadow-[var(--shadow-xs)]",
        isHovered && "-translate-y-[1px]",
        isHovered && "border-white/10"
      )}
    >
      {/* Task Title */}
      <div
        data-testid="review-task-title"
        className="font-semibold text-sm truncate text-[var(--text-primary)] leading-tight"
      >
        {taskTitle}
      </div>

      {/* Status Row */}
      <div className="flex flex-wrap items-center gap-2 mt-2">
        <ReviewStatusBadge status={review.status} />
        <ReviewerTypeIndicator type={review.reviewerType} />
        {showFixCounter && (
          <FixAttemptCounter attempt={fixAttempt} max={maxFixAttempts} />
        )}
      </div>

      {/* Notes Preview */}
      {review.notes && (
        <NotesPreview
          notes={review.notes}
          onViewFull={
            onViewFullNotes ? () => onViewFullNotes(review.id) : undefined
          }
        />
      )}

      {/* Action Buttons */}
      {hasButtons && (
        <div className="flex flex-wrap gap-2 mt-4">
          {onViewDiff && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onViewDiff(review.id)}
              disabled={isLoading}
              className="bg-[var(--bg-hover)] hover:bg-[var(--bg-base)] text-[var(--text-primary)]"
            >
              <GitCompare className="w-4 h-4 mr-1.5" />
              View Diff
            </Button>
          )}
          {showActions && onRequestChanges && (
            <Button
              size="sm"
              onClick={() => onRequestChanges(review.id)}
              disabled={isLoading}
              className="bg-[var(--status-warning)] text-[var(--bg-base)] hover:opacity-90 active:scale-[0.98]"
            >
              {isLoading ? (
                <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />
              ) : null}
              Request Changes
            </Button>
          )}
          {showActions && onApprove && (
            <Button
              size="sm"
              onClick={() => onApprove(review.id)}
              disabled={isLoading}
              className="bg-[var(--status-success)] text-white hover:opacity-90 active:scale-[0.98]"
            >
              {isLoading ? (
                <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />
              ) : null}
              Approve
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}
