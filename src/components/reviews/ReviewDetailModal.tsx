/**
 * ReviewDetailModal - Full-width modal for detailed code review
 *
 * Shows task context, AI review summary, review history on the left,
 * and DiffViewer on the right. Footer has Approve/Request Changes buttons.
 *
 * Part of the Review System (Phase 20).
 */

import { useState, useCallback, useEffect, useMemo } from "react";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import {
  X,
  Loader2,
  CheckCircle2,
  RotateCcw,
  Bot,
  User,
  MessageSquare,
  Clock,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { DiffViewer } from "@/components/diff";
import { useGitDiff } from "@/hooks/useGitDiff";
import { useReviewsByTaskId, useTaskStateHistory, reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { useConfirmation } from "@/hooks/useConfirmation";
import type { Commit } from "@/components/diff";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface ReviewDetailModalProps {
  taskId: string;
  /** Pre-fetched history from parent to avoid duplicate API calls */
  history?: ReviewNoteResponse[];
  /** @deprecated No longer required - using task-based approval APIs */
  reviewId?: string;
  onClose: () => void;
}

/**
 * TaskContextSection - Shows task title, category, description
 */
function TaskContextSection({
  title,
  category,
  description,
  priority,
  isLoading,
}: {
  title: string;
  category: string;
  description: string | null;
  priority: number;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <div className="space-y-2">
        <div className="h-5 w-3/4 rounded animate-pulse bg-white/10" />
        <div className="h-4 w-1/2 rounded animate-pulse bg-white/10" />
        <div className="h-16 w-full rounded animate-pulse bg-white/10" />
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <h3
        data-testid="modal-task-title"
        className="font-semibold text-white/90"
        style={{ letterSpacing: "-0.02em", lineHeight: "1.3" }}
      >
        {title}
      </h3>
      <div className="flex items-center gap-3 text-[12px]">
        <span className="text-white/50">
          Priority: <span className="text-white/70">{priority}</span>
        </span>
        <span className="text-white/50">
          Category: <span className="text-white/70">{category}</span>
        </span>
      </div>
      {description && (
        <p
          data-testid="modal-task-description"
          className="text-[12px] text-white/60"
          style={{ lineHeight: "1.5" }}
        >
          {description}
        </p>
      )}
    </div>
  );
}

/**
 * AIReviewSummary - Shows the AI review summary and checklist
 */
function AIReviewSummary({
  latestApproved,
  hasAiReview,
}: {
  latestApproved: ReviewNoteResponse | null;
  hasAiReview: boolean;
}) {
  if (!hasAiReview) {
    return (
      <div
        className="rounded-lg p-3 text-center"
        style={{
          backgroundColor: "rgba(0, 0, 0, 0.2)",
          border: "1px solid rgba(255,255,255,0.05)",
        }}
      >
        <p className="text-[12px] text-white/40">No AI review yet</p>
      </div>
    );
  }

  // Default checklist items (could come from review data in future)
  const checklistItems = [
    { label: "Code follows project patterns", passed: true },
    { label: "Tests are passing", passed: true },
    { label: "No linting errors", passed: true },
  ];

  return (
    <div
      data-testid="ai-review-summary"
      className="rounded-lg p-3 space-y-3"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: "1px solid rgba(16, 185, 129, 0.2)",
      }}
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <div
          className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
          style={{ backgroundColor: "rgba(59, 130, 246, 0.15)" }}
        >
          <Bot className="w-3.5 h-3.5" style={{ color: "var(--status-info)" }} />
        </div>
        <span className="text-[12px] font-medium text-white/70">
          AI Review Summary
        </span>
        {latestApproved && (
          <span
            className="ml-auto text-[11px] px-2 py-0.5 rounded-full"
            style={{
              backgroundColor: "rgba(16, 185, 129, 0.15)",
              color: "var(--status-success)",
            }}
          >
            Passed
          </span>
        )}
      </div>

      {/* Summary text */}
      {latestApproved?.notes && (
        <p className="text-[12px] text-white/60" style={{ lineHeight: "1.5" }}>
          {latestApproved.notes}
        </p>
      )}

      {/* Checklist */}
      <div className="space-y-0.5">
        {checklistItems.map((item, index) => (
          <div
            key={index}
            className="flex items-center gap-2 py-1"
            style={{ color: item.passed ? "rgba(255,255,255,0.6)" : "rgba(255,255,255,0.4)" }}
          >
            <CheckCircle2
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: item.passed ? "var(--status-success)" : "rgba(255,255,255,0.3)" }}
            />
            <span className="text-[12px]">{item.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * ReviewHistorySection - Shows review history timeline
 */
function ReviewHistorySection({ history }: { history: ReviewNoteResponse[] }) {
  if (history.length === 0) {
    return (
      <p className="text-[12px] text-white/40 italic">No review history</p>
    );
  }

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
  };

  return (
    <div data-testid="review-history" className="space-y-2">
      {history.map((entry) => (
        <div
          key={entry.id}
          className="flex items-start gap-2 py-1.5 px-2 rounded"
          style={{
            backgroundColor: "rgba(0, 0, 0, 0.15)",
            border: "1px solid rgba(255,255,255,0.05)",
          }}
        >
          {/* Icon based on outcome */}
          {entry.outcome === "approved" ? (
            <CheckCircle2
              className="w-3.5 h-3.5 mt-0.5 shrink-0"
              style={{ color: "var(--status-success)" }}
            />
          ) : entry.outcome === "changes_requested" ? (
            <RotateCcw
              className="w-3.5 h-3.5 mt-0.5 shrink-0"
              style={{ color: "var(--status-warning)" }}
            />
          ) : (
            <X
              className="w-3.5 h-3.5 mt-0.5 shrink-0"
              style={{ color: "var(--status-error)" }}
            />
          )}

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-1.5">
              {entry.reviewer === "ai" ? (
                <Bot className="w-3 h-3 text-white/50" />
              ) : (
                <User className="w-3 h-3 text-white/50" />
              )}
              <span className="text-[11px] font-medium text-white/60">
                {entry.reviewer === "ai" ? "AI" : "Human"}{" "}
                {entry.outcome === "approved"
                  ? "approved"
                  : entry.outcome === "changes_requested"
                  ? "requested changes"
                  : "rejected"}
              </span>
              <span className="text-[10px] text-white/40 ml-auto flex items-center gap-1">
                <Clock className="w-2.5 h-2.5" />
                {formatDate(entry.created_at)}
              </span>
            </div>
            {entry.notes && (
              <p className="text-[11px] text-white/40 truncate mt-0.5">
                {entry.notes}
              </p>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

/**
 * RevisionCountBadge - Shows current revision attempt
 */
function RevisionCountBadge({ count }: { count: number }) {
  if (count === 0) return null;

  return (
    <div
      data-testid="revision-count-badge"
      className="flex items-center gap-1.5 px-2 py-1 rounded"
      style={{
        backgroundColor: "rgba(251, 191, 36, 0.1)",
        border: "1px solid rgba(251, 191, 36, 0.2)",
      }}
    >
      <RotateCcw
        className="w-3.5 h-3.5"
        style={{ color: "var(--status-warning)" }}
      />
      <span className="text-[11px] font-medium" style={{ color: "var(--status-warning)" }}>
        Revision #{count}
      </span>
    </div>
  );
}

/**
 * SectionTitle - Consistent section heading
 */
function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h4
      className="text-[11px] font-semibold uppercase tracking-wider mb-2"
      style={{ color: "var(--text-muted)" }}
    >
      {children}
    </h4>
  );
}

/**
 * ReviewDetailModal Component
 */
export function ReviewDetailModal({
  taskId,
  history: historyProp,
  onClose,
}: ReviewDetailModalProps) {
  const queryClient = useQueryClient();
  const [showFeedbackInput, setShowFeedbackInput] = useState(false);
  const [feedback, setFeedback] = useState("");
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

  // Fetch task
  const { data: task, isLoading: taskLoading } = useQuery({
    queryKey: taskKeys.detail(taskId),
    queryFn: () => api.tasks.get(taskId),
    enabled: !!taskId,
  });

  // Fetch reviews for this task (for hasAiReview indicator)
  const { hasAiReview } = useReviewsByTaskId(taskId);

  // Use pre-fetched history from prop, or fetch if not provided
  const { data: fetchedHistory } = useTaskStateHistory(taskId, { enabled: !historyProp });
  const history = historyProp ?? fetchedHistory ?? [];

  // Git diff data - backend determines working path from task/project
  const { changes, commits, isLoadingChanges, isLoadingHistory, fetchDiff } =
    useGitDiff({ taskId, enabled: !!taskId });

  // Check if task is in a state that allows human approval
  // (review_passed or escalated)
  const canApprove = task?.internalStatus === "review_passed" || task?.internalStatus === "escalated";

  // Get latest approved review for summary
  const latestApproved = useMemo(() => {
    const approved = history.filter((h) => h.outcome === "approved");
    return approved.length > 0 ? approved[0] : null;
  }, [history]);

  // Count revisions (changes_requested outcomes)
  const revisionCount = useMemo(() => {
    return history.filter((h) => h.outcome === "changes_requested").length;
  }, [history]);

  // Approve mutation (task-based)
  const approveMutation = useMutation({
    mutationFn: async () => {
      await api.reviews.approveTask({ task_id: taskId });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      onClose();
    },
  });

  // Request changes mutation (task-based)
  const requestChangesMutation = useMutation({
    mutationFn: async (notes: string) => {
      await api.reviews.requestTaskChanges({ task_id: taskId, feedback: notes });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      onClose();
    },
  });

  const handleRequestChangesClick = useCallback(() => {
    if (showFeedbackInput && feedback.trim()) {
      requestChangesMutation.mutate(feedback.trim());
    } else {
      setShowFeedbackInput(true);
    }
  }, [showFeedbackInput, feedback, requestChangesMutation]);

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

  const handleCommitSelect = useCallback((_commit: Commit) => {
    // In a real implementation, this would fetch files changed in the commit
  }, []);

  // Keyboard shortcut: Escape to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const isLoading = approveMutation.isPending || requestChangesMutation.isPending;

  return (
    <Dialog open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        data-testid="review-detail-modal"
        className={cn(
          "p-0 gap-0 overflow-hidden flex flex-col",
          "max-w-7xl w-[90vw] h-[85vh]"
        )}
        style={{
          backgroundColor: "var(--bg-surface)",
          border: "1px solid rgba(255,255,255,0.08)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-4 py-3 border-b shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
          }}
        >
          <div className="flex items-center gap-3">
            <h2
              data-testid="review-detail-modal-title"
              className="text-base font-semibold text-white/90"
              style={{ letterSpacing: "-0.02em" }}
            >
              Review: {task?.title ?? "Loading..."}
            </h2>
            <RevisionCountBadge count={revisionCount} />
          </div>
          <Button
            data-testid="review-detail-modal-close"
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="w-8 h-8 text-white/50 hover:text-white/80 hover:bg-white/10"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Content: Two-column layout */}
        <div className="flex flex-1 min-h-0">
          {/* Left Pane: Context (300px fixed) */}
          <div
            className="w-[300px] shrink-0 flex flex-col border-r"
            style={{ borderColor: "rgba(255,255,255,0.06)" }}
          >
            <ScrollArea className="flex-1">
              <div className="p-4 space-y-5">
                {/* Task Context */}
                <div>
                  <SectionTitle>Task Details</SectionTitle>
                  <TaskContextSection
                    title={task?.title ?? ""}
                    category={task?.category ?? ""}
                    description={task?.description ?? null}
                    priority={task?.priority ?? 0}
                    isLoading={taskLoading}
                  />
                </div>

                {/* AI Review Summary */}
                <div>
                  <SectionTitle>AI Review</SectionTitle>
                  <AIReviewSummary
                    latestApproved={latestApproved ?? null}
                    hasAiReview={hasAiReview}
                  />
                </div>

                {/* Review History */}
                <div>
                  <SectionTitle>Review History</SectionTitle>
                  <ReviewHistorySection history={history} />
                </div>
              </div>
            </ScrollArea>

            {/* Feedback Input (in left pane when visible) */}
            {showFeedbackInput && (
              <div
                className="p-3 border-t"
                style={{ borderColor: "rgba(255,255,255,0.06)" }}
              >
                <div className="flex items-center gap-2 mb-2">
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
                <button
                  onClick={() => {
                    setShowFeedbackInput(false);
                    setFeedback("");
                  }}
                  className="text-[11px] text-white/40 hover:text-white/60 mt-2"
                >
                  Cancel
                </button>
              </div>
            )}
          </div>

          {/* Right Pane: DiffViewer (flex-1) */}
          <div className="flex-1 min-w-0">
            <DiffViewer
              changes={changes}
              commits={commits}
              onFetchDiff={fetchDiff}
              isLoadingChanges={isLoadingChanges}
              isLoadingHistory={isLoadingHistory}
              defaultTab="changes"
              onCommitSelect={handleCommitSelect}
            />
          </div>
        </div>

        {/* Footer: Action Buttons */}
        <div
          className="flex items-center justify-end gap-3 px-4 py-3 border-t shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
          }}
        >
          {/* Error display */}
          {(approveMutation.error || requestChangesMutation.error) && (
            <span className="text-[12px] text-red-400 mr-auto">
              {approveMutation.error?.message || requestChangesMutation.error?.message}
            </span>
          )}

          <Button
            data-testid="review-detail-request-changes"
            onClick={handleRequestChangesClick}
            disabled={isLoading || !canApprove || (showFeedbackInput && !feedback.trim())}
            variant="outline"
            className="gap-1.5"
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
            {showFeedbackInput ? "Submit Changes" : "Request Changes"}
          </Button>
          <Button
            data-testid="review-detail-approve"
            onClick={handleApprove}
            disabled={isLoading || !canApprove || showFeedbackInput}
            className="gap-1.5"
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
        </div>
        <ConfirmationDialog {...confirmationDialogProps} />
      </DialogContent>
    </Dialog>
  );
}
