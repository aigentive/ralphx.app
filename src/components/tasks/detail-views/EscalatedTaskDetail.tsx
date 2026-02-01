/**
 * EscalatedTaskDetail - macOS Tahoe-inspired escalation view
 *
 * Shows AI escalation with clear reasoning and actionable decision buttons.
 */

import { useState, useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
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
import { useTaskStateHistory, reviewKeys } from "@/hooks/useReviews";
import { taskKeys } from "@/hooks/useTasks";
import { useConfirmation } from "@/hooks/useConfirmation";
import { api } from "@/lib/tauri";
import {
  Loader2,
  AlertTriangle,
  Bot,
  RotateCcw,
  MessageSquare,
  HelpCircle,
  ThumbsUp,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse, ReviewIssue } from "@/lib/tauri";

interface EscalatedTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

function getLatestEscalationReview(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  if (history.length === 0) return null;
  return history[0] ?? null;
}

/**
 * IssueCard - Individual issue with severity indicator
 */
const DEFAULT_SEVERITY = { color: "#8e8e93", bg: "rgba(142, 142, 147, 0.15)", label: "Minor" };

function IssueCard({ issue }: { issue: ReviewIssue }) {
  const severityConfig: Record<string, { color: string; bg: string; label: string }> = {
    critical: { color: "#ff453a", bg: "rgba(255, 69, 58, 0.15)", label: "Critical" },
    major: { color: "#ff9f0a", bg: "rgba(255, 159, 10, 0.15)", label: "Major" },
    minor: DEFAULT_SEVERITY,
    suggestion: { color: "#ff6b35", bg: "rgba(255, 107, 53, 0.15)", label: "Suggestion" },
  };

  const config = severityConfig[issue.severity] ?? DEFAULT_SEVERITY;

  return (
    <div
      className="flex items-start gap-3 p-3 rounded-xl"
      style={{ backgroundColor: "rgba(0,0,0,0.2)" }}
    >
      {/* Severity badge */}
      <div
        className="px-2 py-0.5 rounded-md text-[9px] font-bold uppercase tracking-wider shrink-0"
        style={{ backgroundColor: config.bg, color: config.color }}
      >
        {config.label}
      </div>

      {/* Issue details */}
      <div className="flex-1 min-w-0">
        {issue.file && (
          <code
            className="text-[11px] text-white/40 block truncate mb-1 font-mono"
          >
            {issue.file}
            {issue.line !== null && issue.line !== undefined && `:${issue.line}`}
          </code>
        )}
        <p className="text-[13px] text-white/65 leading-relaxed">
          {issue.description}
        </p>
      </div>
    </div>
  );
}

/**
 * EscalationReasonCard - Shows why AI escalated
 */
function EscalationReasonCard({ review }: { review: ReviewNoteResponse | null }) {
  const issues = review?.issues ?? [];

  return (
    <DetailCard variant="warning">
      {/* Header */}
      <div className="flex items-center gap-3 mb-4">
        <div
          className="flex items-center justify-center w-9 h-9 rounded-xl shrink-0"
          style={{ backgroundColor: "rgba(255, 159, 10, 0.2)" }}
        >
          <Bot className="w-5 h-5" style={{ color: "#ff9f0a" }} />
        </div>
        <div>
          <span className="text-[13px] font-semibold text-white/80 block">
            Escalation Reason
          </span>
          <span className="text-[11px] text-white/45">
            AI couldn't make a decision
          </span>
        </div>
      </div>

      {/* Reason text */}
      {review?.notes ? (
        <p className="text-[13px] text-white/55 leading-relaxed mb-4 pl-12">
          {review.notes}
        </p>
      ) : (
        <p className="text-[13px] text-white/35 italic mb-4 pl-12">
          No escalation reason provided
        </p>
      )}

      {/* Issues list */}
      {issues.length > 0 && (
        <div className="mt-4 pl-12 space-y-2">
          <div className="flex items-center gap-2 mb-3">
            <AlertTriangle className="w-4 h-4" style={{ color: "#ff9f0a" }} />
            <span className="text-[11px] font-semibold uppercase tracking-wider text-white/50">
              Issues Found ({issues.length})
            </span>
          </div>
          {issues.map((issue, i) => (
            <IssueCard key={i} issue={issue} />
          ))}
        </div>
      )}
    </DetailCard>
  );
}

/**
 * DecisionButtonsCard - Human decision actions
 */
function DecisionButtonsCard({
  taskId,
  onApproveSuccess,
  onRequestChangesSuccess,
}: {
  taskId: string;
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
      title: "Approve despite concerns?",
      description: "The AI flagged potential issues. Are you sure you want to approve?",
      confirmText: "Approve Anyway",
      variant: "default",
    });
    if (!confirmed) return;
    approveMutation.mutate();
  }, [confirm, approveMutation]);

  const isLoading = approveMutation.isPending || requestChangesMutation.isPending;

  return (
    <DetailCard>
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

      {/* Decision buttons */}
      <div className="flex gap-3">
        <Button
          data-testid="approve-button"
          onClick={handleApprove}
          disabled={isLoading || showFeedback}
          className="flex-1 h-11 gap-2 rounded-xl font-semibold text-[13px]"
          style={{
            background: "linear-gradient(180deg, #34c759 0%, #28a745 100%)",
            color: "white",
            boxShadow: "0 2px 8px rgba(52, 199, 89, 0.3), inset 0 1px 0 rgba(255,255,255,0.2)",
          }}
        >
          {approveMutation.isPending ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <ThumbsUp className="w-4 h-4" />
          )}
          Approve Anyway
        </Button>

        <Button
          data-testid="request-changes-button"
          onClick={handleRequestChangesClick}
          disabled={isLoading || (showFeedback && !feedback.trim())}
          variant="outline"
          className="flex-1 h-11 gap-2 rounded-xl font-semibold text-[13px]"
          style={{
            borderColor: "#ff9f0a",
            color: "#ffd60a",
            backgroundColor: "rgba(255, 159, 10, 0.1)",
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

      {(approveMutation.error || requestChangesMutation.error) && (
        <p className="mt-3 text-[12px]" style={{ color: "#ff453a" }}>
          {approveMutation.error?.message || requestChangesMutation.error?.message}
        </p>
      )}

      <ConfirmationDialog {...confirmationDialogProps} />
    </DetailCard>
  );
}

export function EscalatedTaskDetail({ task, isHistorical = false }: EscalatedTaskDetailProps) {
  const { data: history, isLoading } = useTaskStateHistory(task.id);
  const escalationReview = getLatestEscalationReview(history);

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
      data-testid="escalated-task-detail"
      data-task-id={task.id}
      className="space-y-6"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={HelpCircle}
        title="Escalated to Human"
        subtitle="AI reviewer couldn't make a decision"
        variant="warning"
        badge={
          <StatusPill
            icon={AlertTriangle}
            label="Needs Decision"
            variant="warning"
            size="md"
          />
        }
      />

      {/* Escalation Reason */}
      <section data-testid="ai-escalation-reason-section">
        <SectionTitle>Why AI Escalated</SectionTitle>
        <EscalationReasonCard review={escalationReview} />
      </section>

      {/* Previous Attempts */}
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

      {/* Description */}
      <section>
        <SectionTitle>Description</SectionTitle>
        <DescriptionBlock
          description={task.description}
          testId="escalated-task-description"
        />
      </section>

      {/* Decision Buttons (hidden in historical mode) */}
      {!isHistorical && (
        <section data-testid="action-buttons">
          <SectionTitle>Your Decision</SectionTitle>
          <DecisionButtonsCard taskId={task.id} />
        </section>
      )}
    </div>
  );
}
