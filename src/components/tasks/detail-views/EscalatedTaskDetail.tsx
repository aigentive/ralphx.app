/**
 * EscalatedTaskDetail - Task detail view for escalated state
 *
 * Shows AI escalated state with warning banner, escalation reason,
 * and action buttons for human decision (approve or request changes).
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { SectionTitle, ReviewTimeline } from "./shared";
import { useTaskStateHistory, reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import { useConfirmation } from "@/hooks/useConfirmation";
import { api } from "@/lib/tauri";
import {
  Loader2,
  AlertTriangle,
  Bot,
  ExternalLink,
  RotateCcw,
  CheckCircle2,
  MessageSquare,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse, ReviewIssue } from "@/lib/tauri";

interface EscalatedTaskDetailProps {
  task: Task;
  /** True when viewing a historical state - disables action buttons */
  isHistorical?: boolean;
}

/**
 * EscalatedBadge - Shows warning indicator for AI-escalated status
 */
function EscalatedBadge() {
  return (
    <div
      data-testid="escalated-badge"
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{
        backgroundColor: "rgba(245, 158, 11, 0.15)",
        color: "var(--status-warning)",
      }}
    >
      <AlertTriangle
        className="w-3 h-3"
        style={{ color: "var(--status-warning)" }}
      />
      Needs Human Decision
    </div>
  );
}

/**
 * AIEscalationReasonCard - Displays AI escalation reason and issues
 */
function AIEscalationReasonCard({
  review,
  onViewDiff,
}: {
  review: ReviewNoteResponse | null;
  onViewDiff?: () => void;
}) {
  const issues = review?.issues ?? [];

  return (
    <div
      data-testid="ai-escalation-reason-card"
      className="rounded-lg p-3 space-y-3"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: "1px solid rgba(245, 158, 11, 0.2)",
      }}
    >
      {/* Header: AI icon + label */}
      <div className="flex items-center gap-2">
        <div
          className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
          style={{ backgroundColor: "rgba(245, 158, 11, 0.15)" }}
        >
          <Bot
            className="w-3.5 h-3.5"
            style={{ color: "var(--status-warning)" }}
          />
        </div>
        <span className="text-[12px] font-medium text-white/70">
          Escalation Reason
        </span>
      </div>

      {/* Reason text */}
      {review?.notes ? (
        <p
          data-testid="ai-escalation-reason-text"
          className="text-[12px] text-white/60"
          style={{ lineHeight: "1.5" }}
        >
          {review.notes}
        </p>
      ) : (
        <p className="text-[12px] italic text-white/40">
          No escalation reason provided
        </p>
      )}

      {/* Issues list */}
      {issues.length > 0 && <ReviewIssuesList issues={issues} />}

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
 * ReviewIssuesList - Displays review issues with severity badges and file:line references
 */
function ReviewIssuesList({ issues }: { issues: ReviewIssue[] }) {
  const severityColors: Record<string, string> = {
    critical: "var(--status-error)",
    major: "var(--status-warning)",
    minor: "var(--text-muted)",
    suggestion: "var(--accent-primary)",
  };

  const severityIcons: Record<string, string> = {
    critical: "\u26d4", // no entry sign
    major: "\u26a0", // warning sign
    minor: "\u2139", // info sign
    suggestion: "\u2728", // sparkle
  };

  return (
    <div data-testid="review-issues-list" className="space-y-2 mt-3">
      <div className="flex items-center gap-2 mb-2">
        <AlertTriangle
          className="w-3.5 h-3.5"
          style={{ color: "var(--status-warning)" }}
        />
        <span className="text-[11px] font-medium text-white/60 uppercase tracking-wider">
          Issues Found ({issues.length})
        </span>
      </div>
      {issues.map((issue, i) => (
        <div
          key={i}
          data-testid={`review-issue-${i}`}
          className="flex items-start gap-2 p-2 rounded"
          style={{
            backgroundColor: "rgba(0, 0, 0, 0.2)",
            border: "1px solid rgba(255,255,255,0.05)",
          }}
        >
          <span
            className="text-[10px] font-bold uppercase px-1.5 py-0.5 rounded shrink-0"
            style={{
              backgroundColor: `color-mix(in srgb, ${severityColors[issue.severity] || severityColors.minor} 15%, transparent)`,
              color: severityColors[issue.severity] || severityColors.minor,
            }}
          >
            {severityIcons[issue.severity] || "\u2022"} {issue.severity}
          </span>
          <div className="flex-1 min-w-0">
            {issue.file && (
              <span className="text-[11px] text-white/40 block truncate">
                {issue.file}
                {issue.line !== null && issue.line !== undefined && `:${issue.line}`}
              </span>
            )}
            <p className="text-[12px] text-white/70" style={{ lineHeight: "1.4" }}>
              {issue.description}
            </p>
          </div>
        </div>
      ))}
    </div>
  );
}


/**
 * ActionButtons - Approve and Request Changes buttons
 * Uses task-based API (not review-based) for human approval actions.
 */
function ActionButtons({
  taskId,
  onApproveSuccess,
  onRequestChangesSuccess,
}: {
  taskId: string;
  onApproveSuccess?: () => void;
  onRequestChangesSuccess?: () => void;
}) {
  const queryClient = useQueryClient();
  const [showFeedbackInput, setShowFeedbackInput] = useState(false);
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
          onClick={handleApprove}
          disabled={isLoading || showFeedbackInput}
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
          disabled={isLoading || (showFeedbackInput && !feedback.trim())}
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

      <ConfirmationDialog {...confirmationDialogProps} />
    </div>
  );
}

/**
 * Get the latest review entry with escalation reason
 *
 * When AI escalates, the review outcome is stored as "rejected" with the
 * escalation reason in the notes field. We get the most recent entry
 * since the task is already in escalated status.
 */
function getLatestEscalationReview(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  // Get the most recent entry - when task is in escalated state,
  // the latest review note contains the escalation reason
  if (history.length === 0) return null;
  // History is already sorted newest first by useTaskStateHistory
  return history[0] ?? null;
}

/**
 * EscalatedTaskDetail Component
 *
 * Renders task information for escalated state.
 * Shows: Warning banner, escalation reason, previous attempts, and action buttons.
 */
export function EscalatedTaskDetail({ task, isHistorical = false }: EscalatedTaskDetailProps) {
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(task.id);

  const latestEscalationReview = getLatestEscalationReview(history);

  const handleViewDiff = () => {
    // TODO: Implement DiffViewer/ReviewDetailModal (deferred to PRD:20)
  };

  const isLoading = historyLoading;

  return (
    <div
      data-testid="escalated-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* AI Escalated Banner */}
      <div
        data-testid="escalated-banner"
        className="flex items-center gap-2 px-3 py-2 rounded-lg"
        style={{
          backgroundColor: "rgba(245, 158, 11, 0.1)",
          border: "1px solid rgba(245, 158, 11, 0.25)",
        }}
      >
        <AlertTriangle
          className="w-4 h-4 shrink-0"
          style={{ color: "var(--status-warning)" }}
        />
        <div className="flex-1">
          <span
            className="text-[13px] font-medium"
            style={{ color: "var(--status-warning)" }}
          >
            AI ESCALATED TO HUMAN
          </span>
          <span className="text-[12px] text-white/50 ml-2">
            AI reviewer couldn't decide - needs your input
          </span>
        </div>
        <EscalatedBadge />
      </div>

      {/* Loading state */}
      {isLoading && (
        <div
          data-testid="escalated-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}

      {/* AI Escalation Reason */}
      {!isLoading && (
        <div data-testid="ai-escalation-reason-section">
          <SectionTitle>Why AI Escalated</SectionTitle>
          <AIEscalationReasonCard
            review={latestEscalationReview}
            onViewDiff={handleViewDiff}
          />
        </div>
      )}

      {/* Previous Attempts */}
      {!isLoading && (
        <div data-testid="previous-attempts-section">
          <SectionTitle>Previous Attempts</SectionTitle>
          <ReviewTimeline
            history={history}
            filter={(e) => e.outcome === "changes_requested"}
            showAttemptNumbers
            emptyMessage="No previous attempts"
          />
        </div>
      )}

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="escalated-task-description"
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

      {/* Action Buttons - hidden in historical mode */}
      {!isLoading && !isHistorical && (
        <ActionButtons
          taskId={task.id}
        />
      )}
    </div>
  );
}
