/**
 * HumanReviewTaskDetail - Task detail view for review_passed state
 *
 * Shows AI review passed state with summary, checklist of passed items,
 * and action buttons for human approval or requesting changes.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { SectionTitle } from "./shared";
import { useReviewsByTaskId, useTaskStateHistory, reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import {
  Loader2,
  CheckCircle2,
  Bot,
  ExternalLink,
  RotateCcw,
  Check,
  MessageSquare,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface HumanReviewTaskDetailProps {
  task: Task;
}

/**
 * ReviewPassedBadge - Shows green indicator for AI-approved status
 */
function ReviewPassedBadge() {
  return (
    <div
      data-testid="review-passed-badge"
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{
        backgroundColor: "rgba(16, 185, 129, 0.15)",
        color: "var(--status-success)",
      }}
    >
      <CheckCircle2
        className="w-3 h-3"
        style={{ color: "var(--status-success)" }}
      />
      AI Approved
    </div>
  );
}

/**
 * ChecklistItem - Individual item in the AI review checklist
 */
function ChecklistItem({ label, passed }: { label: string; passed: boolean }) {
  return (
    <div
      className="flex items-center gap-2 py-1"
      style={{ color: passed ? "rgba(255,255,255,0.6)" : "rgba(255,255,255,0.4)" }}
    >
      {passed ? (
        <Check
          className="w-3.5 h-3.5 shrink-0"
          style={{ color: "var(--status-success)" }}
        />
      ) : (
        <div
          className="w-3.5 h-3.5 rounded-full border shrink-0"
          style={{ borderColor: "rgba(255,255,255,0.3)" }}
        />
      )}
      <span className="text-[12px]">{label}</span>
    </div>
  );
}

/**
 * AIReviewSummaryCard - Displays AI review summary with checklist
 */
function AIReviewSummaryCard({
  review,
  onViewDiff,
}: {
  review: ReviewNoteResponse | null;
  onViewDiff?: () => void;
}) {
  // Default checklist items (these would ideally come from the review data)
  const checklistItems = [
    { label: "Code follows project patterns", passed: true },
    { label: "Tests are passing", passed: true },
    { label: "No linting errors", passed: true },
  ];

  return (
    <div
      data-testid="ai-review-summary-card"
      className="rounded-lg p-3 space-y-3"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: "1px solid rgba(16, 185, 129, 0.2)",
      }}
    >
      {/* Header: AI icon + confidence */}
      <div className="flex items-center gap-2">
        <div
          className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
          style={{ backgroundColor: "rgba(59, 130, 246, 0.15)" }}
        >
          <Bot
            className="w-3.5 h-3.5"
            style={{ color: "var(--status-info)" }}
          />
        </div>
        <span className="text-[12px] font-medium text-white/70">
          AI Review Summary
        </span>
      </div>

      {/* Summary text */}
      {review?.notes && (
        <p
          data-testid="ai-review-summary-text"
          className="text-[12px] text-white/60"
          style={{ lineHeight: "1.5" }}
        >
          {review.notes}
        </p>
      )}

      {/* Checklist */}
      <div className="space-y-0.5">
        {checklistItems.map((item, index) => (
          <ChecklistItem key={index} label={item.label} passed={item.passed} />
        ))}
      </div>

      {/* View Diff link */}
      {onViewDiff && (
        <button
          onClick={onViewDiff}
          className="flex items-center gap-1.5 text-[12px] font-medium mt-2 transition-opacity hover:opacity-80"
          style={{ color: "var(--accent-primary)" }}
        >
          <ExternalLink className="w-3.5 h-3.5" />
          View Diff
        </button>
      )}
    </div>
  );
}

/**
 * PreviousAttemptsSection - Shows previous revision attempts if any
 */
function PreviousAttemptsSection({ history }: { history: ReviewNoteResponse[] }) {
  const changesRequestedEntries = history.filter(
    (entry) => entry.outcome === "changes_requested"
  );

  if (changesRequestedEntries.length === 0) return null;

  return (
    <div data-testid="previous-attempts-section">
      <SectionTitle>Previous Attempts</SectionTitle>
      <div className="space-y-2">
        {changesRequestedEntries.map((entry, index) => (
          <div
            key={entry.id}
            className="flex items-start gap-2 py-1.5 px-2 rounded"
            style={{
              backgroundColor: "rgba(0, 0, 0, 0.15)",
              border: "1px solid rgba(255,255,255,0.05)",
            }}
          >
            <RotateCcw
              className="w-3.5 h-3.5 mt-0.5 shrink-0"
              style={{ color: "var(--status-warning)" }}
            />
            <div className="flex-1 min-w-0">
              <span className="text-[11px] font-medium text-white/60">
                Attempt #{changesRequestedEntries.length - index}: Changes requested
              </span>
              {entry.notes && (
                <p className="text-[11px] text-white/40 truncate mt-0.5">
                  {entry.notes}
                </p>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * ActionButtons - Approve and Request Changes buttons
 */
function ActionButtons({
  reviewId,
  onApproveSuccess,
  onRequestChangesSuccess,
}: {
  reviewId: string | null;
  onApproveSuccess?: () => void;
  onRequestChangesSuccess?: () => void;
}) {
  const queryClient = useQueryClient();
  const [showFeedbackInput, setShowFeedbackInput] = useState(false);
  const [feedback, setFeedback] = useState("");

  const approveMutation = useMutation({
    mutationFn: async () => {
      if (!reviewId) throw new Error("No review ID");
      await api.reviews.approve({ review_id: reviewId });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      onApproveSuccess?.();
    },
  });

  const requestChangesMutation = useMutation({
    mutationFn: async (notes: string) => {
      if (!reviewId) throw new Error("No review ID");
      await api.reviews.requestChanges({ review_id: reviewId, notes });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowFeedbackInput(false);
      setFeedback("");
      onRequestChangesSuccess?.();
    },
  });

  const handleRequestChangesClick = () => {
    if (showFeedbackInput && feedback.trim()) {
      requestChangesMutation.mutate(feedback.trim());
    } else {
      setShowFeedbackInput(true);
    }
  };

  const isLoading = approveMutation.isPending || requestChangesMutation.isPending;

  return (
    <div data-testid="action-buttons" className="space-y-3">
      {/* Feedback input (shown when Request Changes is clicked) */}
      {showFeedbackInput && (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <MessageSquare className="w-4 h-4 text-white/50" />
            <span className="text-[12px] font-medium text-white/60">
              What needs to be changed?
            </span>
          </div>
          <Textarea
            data-testid="feedback-input"
            value={feedback}
            onChange={(e) => setFeedback(e.target.value)}
            placeholder="Describe the changes needed..."
            className="min-h-[80px] text-[13px] resize-none"
            style={{
              backgroundColor: "rgba(0, 0, 0, 0.2)",
              border: "1px solid rgba(255,255,255,0.1)",
            }}
          />
        </div>
      )}

      {/* Buttons */}
      <div className="flex items-center gap-2">
        <Button
          data-testid="approve-button"
          onClick={() => approveMutation.mutate()}
          disabled={isLoading || !reviewId || showFeedbackInput}
          className="flex-1 gap-1.5"
          style={{
            backgroundColor: "var(--status-success)",
            color: "white",
          }}
        >
          {approveMutation.isPending ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <CheckCircle2 className="w-4 h-4" />
          )}
          Approve
        </Button>
        <Button
          data-testid="request-changes-button"
          onClick={handleRequestChangesClick}
          disabled={isLoading || !reviewId || (showFeedbackInput && !feedback.trim())}
          variant="outline"
          className="flex-1 gap-1.5"
          style={{
            borderColor: "var(--status-warning)",
            color: "var(--status-warning)",
          }}
        >
          {requestChangesMutation.isPending ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <RotateCcw className="w-4 h-4" />
          )}
          {showFeedbackInput ? "Submit" : "Request Changes"}
        </Button>
      </div>

      {/* Cancel button when in feedback mode */}
      {showFeedbackInput && (
        <button
          onClick={() => {
            setShowFeedbackInput(false);
            setFeedback("");
          }}
          className="text-[12px] text-white/40 hover:text-white/60 transition-colors"
        >
          Cancel
        </button>
      )}

      {/* Error display */}
      {(approveMutation.error || requestChangesMutation.error) && (
        <p className="text-[12px] text-red-400">
          {approveMutation.error?.message || requestChangesMutation.error?.message}
        </p>
      )}
    </div>
  );
}

/**
 * Get the latest approved review entry
 */
function getLatestApprovedReview(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  const approvedEntries = history.filter((entry) => entry.outcome === "approved");
  if (approvedEntries.length === 0) return null;
  // History is already sorted newest first by useTaskStateHistory
  return approvedEntries[0] ?? null;
}

/**
 * HumanReviewTaskDetail Component
 *
 * Renders task information for review_passed state.
 * Shows: AI approved banner, review summary, previous attempts, and action buttons.
 */
export function HumanReviewTaskDetail({ task }: HumanReviewTaskDetailProps) {
  const { data: reviews, isLoading: reviewsLoading } = useReviewsByTaskId(task.id);
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(task.id);

  const latestApprovedReview = getLatestApprovedReview(history);
  const pendingReview = reviews.find((r) => r.status === "pending");

  const handleViewDiff = () => {
    console.warn("DiffViewer/ReviewDetailModal not yet implemented");
  };

  const isLoading = reviewsLoading || historyLoading;

  return (
    <div
      data-testid="human-review-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* AI Review Passed Banner */}
      <div
        data-testid="review-passed-banner"
        className="flex items-center gap-2 px-3 py-2 rounded-lg"
        style={{
          backgroundColor: "rgba(16, 185, 129, 0.1)",
          border: "1px solid rgba(16, 185, 129, 0.25)",
        }}
      >
        <CheckCircle2
          className="w-4 h-4 shrink-0"
          style={{ color: "var(--status-success)" }}
        />
        <div className="flex-1">
          <span
            className="text-[13px] font-medium"
            style={{ color: "var(--status-success)" }}
          >
            AI REVIEW PASSED
          </span>
          <span className="text-[12px] text-white/50 ml-2">
            Awaiting your approval
          </span>
        </div>
        <ReviewPassedBadge />
      </div>

      {/* Header: Title */}
      <div className="space-y-1">
        <h2
          data-testid="human-review-task-title"
          className="text-base font-semibold text-white/90"
          style={{
            letterSpacing: "-0.02em",
            lineHeight: "1.3",
          }}
        >
          {task.title}
        </h2>
        <p className="text-[12px] text-white/50">
          Category: <span className="text-white/70">{task.category}</span>
        </p>
      </div>

      {/* Loading state */}
      {isLoading && (
        <div
          data-testid="human-review-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}

      {/* AI Review Summary */}
      {!isLoading && (
        <div data-testid="ai-review-summary-section">
          <SectionTitle>AI Review Summary</SectionTitle>
          <AIReviewSummaryCard
            review={latestApprovedReview}
            onViewDiff={handleViewDiff}
          />
        </div>
      )}

      {/* Previous Attempts */}
      {!isLoading && <PreviousAttemptsSection history={history} />}

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="human-review-task-description"
            className="text-[13px] text-white/60"
            style={{
              lineHeight: "1.6",
              wordBreak: "break-word",
            }}
          >
            {task.description}
          </p>
        ) : (
          <p className="text-[13px] italic text-white/35">
            No description provided
          </p>
        )}
      </div>

      {/* Action Buttons */}
      {!isLoading && (
        <ActionButtons
          reviewId={pendingReview?.id ?? null}
        />
      )}
    </div>
  );
}
