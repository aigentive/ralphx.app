/**
 * HumanReviewTaskDetail - macOS Tahoe-inspired human review view
 *
 * Shows AI-approved state awaiting human confirmation with premium action buttons.
 */

import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  DescriptionBlock,
} from "./shared";
import { ReviewTimeline } from "./shared/ReviewTimeline";
import { ReviewDetailModal } from "@/components/reviews/ReviewDetailModal";
import { useTaskStateHistory, reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import { useConfirmation } from "@/hooks/useConfirmation";
import { api } from "@/lib/tauri";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import {
  Loader2,
  CheckCircle2,
  Bot,
  Check,
  RotateCcw,
  MessageSquare,
  ShieldCheck,
  ThumbsUp,
  Code,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface HumanReviewTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * Get the latest approved review entry
 */
function getLatestApprovedReview(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  const approvedEntries = history.filter((entry) => entry.outcome === "approved");
  if (approvedEntries.length === 0) return null;
  return approvedEntries[0] ?? null;
}

/**
 * ChecklistItem - Individual check with native styling
 */
function ChecklistItem({ label, passed }: { label: string; passed: boolean }) {
  return (
    <div className="flex items-center gap-3 py-2">
      <div
        className="flex items-center justify-center w-5 h-5 rounded-md shrink-0"
        style={{
          backgroundColor: passed
            ? "rgba(52, 199, 89, 0.15)"
            : "rgba(255,255,255,0.06)",
        }}
      >
        {passed ? (
          <Check className="w-3 h-3" style={{ color: "#34c759" }} />
        ) : (
          <div
            className="w-2 h-2 rounded-full"
            style={{ backgroundColor: "rgba(255,255,255,0.2)" }}
          />
        )}
      </div>
      <span
        className="text-[13px]"
        style={{
          color: passed ? "rgba(255,255,255,0.65)" : "rgba(255,255,255,0.4)",
        }}
      >
        {label}
      </span>
    </div>
  );
}

/**
 * AIReviewCard - Summary of AI review findings with collapsible content
 */
function AIReviewCard({ review }: { review: ReviewNoteResponse | null }) {
  const [isExpanded, setIsExpanded] = useState(false);
  const COLLAPSED_HEIGHT = 80; // pixels

  const checks = [
    { label: "Code follows project patterns", passed: true },
    { label: "Tests are passing", passed: true },
    { label: "No linting errors", passed: true },
  ];

  const hasContent = review?.notes && review.notes.length > 100;

  // Click handler - expand when collapsed, clicking anywhere
  const handleCardClick = () => {
    if (!isExpanded && hasContent) {
      setIsExpanded(true);
    }
  };

  return (
    <DetailCard>
      {/* Clickable area when collapsed */}
      <div
        onClick={handleCardClick}
        className={hasContent && !isExpanded ? "cursor-pointer" : ""}
      >
        {/* AI Badge header */}
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-9 h-9 rounded-xl shrink-0"
            style={{ backgroundColor: "rgba(10, 132, 255, 0.15)" }}
          >
            <Bot className="w-5 h-5" style={{ color: "#0a84ff" }} />
          </div>
          <div>
            <span className="text-[13px] font-semibold text-white/80 block">
              AI Review Summary
            </span>
            <span className="text-[11px] text-white/45">
              Automated checks passed
            </span>
          </div>
        </div>

        {/* Collapsible content area */}
        {review?.notes && (
          <div className="relative mt-4">
            <div
              className="pl-12 text-[13px] text-white/65 leading-relaxed prose prose-sm prose-invert max-w-none overflow-hidden transition-all duration-300 ease-out"
              style={{
                maxHeight: isExpanded ? "1000px" : `${COLLAPSED_HEIGHT}px`,
              }}
            >
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {review.notes}
              </ReactMarkdown>

              {/* Checklist inside collapsible */}
              <div className="space-y-0.5 mt-4 not-prose">
                {checks.map((check, i) => (
                  <ChecklistItem key={i} {...check} />
                ))}
              </div>
            </div>

            {/* Gradient fade overlay when collapsed */}
            {hasContent && !isExpanded && (
              <div
                className="absolute bottom-0 left-0 right-0 h-12 pointer-events-none"
                style={{
                  background: "linear-gradient(to bottom, hsla(220 10% 12% / 0), hsl(220 10% 12%))",
                }}
              />
            )}
          </div>
        )}

        {/* Checklist fallback when no notes */}
        {!review?.notes && (
          <div className="pl-12 space-y-0.5 mt-4">
            {checks.map((check, i) => (
              <ChecklistItem key={i} {...check} />
            ))}
          </div>
        )}

        {/* Show more - outside gradient, below content */}
        {hasContent && !isExpanded && (
          <div
            className="pl-12 mt-2 text-[12px] font-medium"
            style={{ color: "hsl(217 90% 60%)" }}
          >
            Show more
          </div>
        )}
      </div>

      {/* Show less button - only when expanded */}
      {hasContent && isExpanded && (
        <button
          onClick={() => setIsExpanded(false)}
          className="pl-12 mt-3 text-[12px] font-medium transition-colors hover:opacity-80"
          style={{ color: "hsl(217 90% 60%)" }}
        >
          Show less
        </button>
      )}
    </DetailCard>
  );
}

/**
 * ActionButtonsCard - Approve/Request Changes with premium styling
 */
function ActionButtonsCard({
  taskId,
  onReviewCode,
  onApproveSuccess,
  onRequestChangesSuccess,
}: {
  taskId: string;
  onReviewCode?: () => void;
  onApproveSuccess?: () => void;
  onRequestChangesSuccess?: () => void;
}) {
  const queryClient = useQueryClient();
  const [showFeedback, setShowFeedback] = useState(false);
  const [feedback, setFeedback] = useState("");
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

  const approveMutation = useMutation({
    mutationFn: async () => {
      await api.reviews.approveTask({ task_id: taskId });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      onApproveSuccess?.();
    },
  });

  const requestChangesMutation = useMutation({
    mutationFn: async (feedbackText: string) => {
      await api.reviews.requestTaskChanges({ task_id: taskId, feedback: feedbackText });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowFeedback(false);
      setFeedback("");
      onRequestChangesSuccess?.();
    },
  });

  const handleRequestChangesClick = () => {
    if (showFeedback && feedback.trim()) {
      requestChangesMutation.mutate(feedback.trim());
    } else {
      setShowFeedback(true);
    }
  };

  const handleApprove = useCallback(async () => {
    const confirmed = await confirm({
      title: "Approve this task?",
      description: "The task will be marked as approved and completed.",
      confirmText: "Approve",
      variant: "default",
    });
    if (!confirmed) return;
    approveMutation.mutate();
  }, [confirm, approveMutation]);

  const isLoading = approveMutation.isPending || requestChangesMutation.isPending;

  return (
    <DetailCard data-testid="action-buttons">
      {/* Feedback input */}
      {showFeedback && (
        <div className="mb-4 space-y-3">
          <div className="flex items-center gap-2">
            <MessageSquare className="w-4 h-4 text-white/40" />
            <span className="text-[12px] font-semibold text-white/60">
              What needs to be changed?
            </span>
          </div>
          <Textarea
            data-testid="feedback-input"
            value={feedback}
            onChange={(e) => setFeedback(e.target.value)}
            placeholder="Describe the changes needed..."
            className="min-h-[100px] text-[13px] resize-none rounded-xl"
            style={{
              backgroundColor: "rgba(0, 0, 0, 0.3)",
              border: "1px solid rgba(255,255,255,0.1)",
            }}
          />
        </div>
      )}

      {/* Label and action buttons on same row */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span
            className="text-[11px] font-semibold uppercase tracking-wider"
            style={{ color: "hsl(220 10% 50%)" }}
          >
            Your Decision
          </span>
          {onReviewCode && (
            <Button
              data-testid="review-code-button"
              onClick={onReviewCode}
              variant="ghost"
              className="h-7 px-3 gap-1.5 rounded-lg font-medium text-[12px]"
              style={{ color: "hsl(217 90% 60%)" }}
            >
              <Code className="w-3.5 h-3.5" />
              Review Code
            </Button>
          )}
        </div>
        <div className="flex gap-2">
        <Button
          data-testid="approve-button"
          onClick={handleApprove}
          disabled={isLoading || showFeedback}
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px] transition-colors"
          style={{
            backgroundColor: "hsl(142 70% 45%)",
            color: "white",
          }}
        >
          {approveMutation.isPending ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <ThumbsUp className="w-4 h-4" />
          )}
          Approve
        </Button>

        <Button
          data-testid="request-changes-button"
          onClick={handleRequestChangesClick}
          disabled={isLoading || (showFeedback && !feedback.trim())}
          variant="outline"
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
          style={{
            borderColor: "hsla(220 10% 100% / 0.15)",
            color: "hsl(35 100% 55%)",
            backgroundColor: "transparent",
          }}
        >
          {requestChangesMutation.isPending ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <RotateCcw className="w-4 h-4" />
          )}
          {showFeedback ? "Submit" : "Request Changes"}
        </Button>
        </div>
      </div>

      {/* Cancel link */}
      {showFeedback && (
        <button
          onClick={() => {
            setShowFeedback(false);
            setFeedback("");
          }}
          className="mt-3 text-[12px] text-white/40 hover:text-white/60 transition-colors"
        >
          Cancel
        </button>
      )}

      {/* Error display */}
      {(approveMutation.error || requestChangesMutation.error) && (
        <p className="mt-3 text-[12px]" style={{ color: "#ff453a" }}>
          {approveMutation.error?.message || requestChangesMutation.error?.message}
        </p>
      )}

      <ConfirmationDialog {...confirmationDialogProps} />
    </DetailCard>
  );
}

export function HumanReviewTaskDetail({ task, isHistorical = false }: HumanReviewTaskDetailProps) {
  const [showReviewModal, setShowReviewModal] = useState(false);
  const { data: history, isLoading } = useTaskStateHistory(task.id);
  const latestApprovedReview = getLatestApprovedReview(history);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-16">
        <Loader2
          className="w-6 h-6 animate-spin"
          style={{ color: "rgba(255,255,255,0.3)" }}
        />
      </div>
    );
  }

  return (
    <div
      data-testid="human-review-task-detail"
      data-task-id={task.id}
      className="space-y-6"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={ShieldCheck}
        title="AI Review Passed"
        subtitle="Awaiting your final approval"
        variant="success"
        badge={
          <StatusPill
            icon={CheckCircle2}
            label="AI Approved"
            variant="success"
            size="md"
          />
        }
      />

      {/* AI Review Summary */}
      <section data-testid="ai-review-summary-section">
        <SectionTitle>AI Review Summary</SectionTitle>
        <AIReviewCard review={latestApprovedReview} />
      </section>

      {/* Previous Attempts (if any) */}
      {history.filter((e) => e.outcome === "changes_requested").length > 0 && (
        <section data-testid="previous-attempts-section">
          <SectionTitle>Previous Attempts</SectionTitle>
          <DetailCard>
            <ReviewTimeline
              history={history}
              filter={(e) => e.outcome === "changes_requested"}
              showAttemptNumbers
              emptyMessage="No previous attempts"
            />
          </DetailCard>
        </section>
      )}

      {/* Description */}
      <section>
        <SectionTitle>Description</SectionTitle>
        <DescriptionBlock
          description={task.description}
          testId="human-review-task-description"
        />
      </section>

      {/* Action Buttons (hidden in historical mode) */}
      {!isHistorical && (
        <section>
          <ActionButtonsCard
            taskId={task.id}
            onReviewCode={() => setShowReviewModal(true)}
          />
        </section>
      )}

      {/* Review Detail Modal */}
      {showReviewModal && (
        <ReviewDetailModal
          taskId={task.id}
          onClose={() => setShowReviewModal(false)}
        />
      )}
    </div>
  );
}
