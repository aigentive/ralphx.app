/**
 * EscalatedTaskDetail - macOS Tahoe-inspired escalation view
 *
 * Shows AI escalation with clear reasoning and actionable decision buttons.
 */

import { useState, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import { useMutation, useQueryClient, useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { ReviewTimeline } from "./shared/ReviewTimeline";
import { IssueList } from "@/components/reviews/IssueList";
import { reviewIssuesApi } from "@/api/review-issues";
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
import type { ReviewNoteResponse } from "@/lib/tauri";

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
 * EscalationReasonCard - Shows why AI escalated (reason text only)
 */
function EscalationReasonCard({ review }: { review: ReviewNoteResponse | null }) {
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
        <div className="text-[13px] text-white/55 leading-relaxed pl-12" style={{ wordBreak: "break-word" }}>
          <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
            {review.notes}
          </ReactMarkdown>
        </div>
      ) : (
        <p className="text-[13px] text-white/35 italic pl-12">
          No escalation reason provided
        </p>
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
      <div className="flex gap-2 justify-end">
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
          Approve Anyway
        </Button>

        <Button
          data-testid="request-changes-button"
          onClick={handleRequestChangesClick}
          disabled={isLoading || (showFeedback && !feedback.trim())}
          variant="ghost"
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
          style={{
            color: "hsl(35 100% 55%)",
            backgroundColor: "hsl(220 10% 16%)",
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

  // Fetch structured issues from review issues API
  const { data: issues = [] } = useQuery({
    queryKey: ["review-issues", task.id],
    queryFn: () => reviewIssuesApi.getByTaskId(task.id),
  });

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
    <TwoColumnLayout
      description={task.description}
      testId="escalated-task-detail"
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

      {/* Issues Found */}
      {issues.length > 0 && (
        <section data-testid="issues-section">
          <SectionTitle>Issues Found ({issues.length})</SectionTitle>
          <DetailCard>
            <IssueList issues={issues} groupBy="severity" compact />
          </DetailCard>
        </section>
      )}

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

      {/* Decision Buttons (hidden in historical mode) */}
      {!isHistorical && (
        <section data-testid="action-buttons">
          <SectionTitle>Your Decision</SectionTitle>
          <DecisionButtonsCard taskId={task.id} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
