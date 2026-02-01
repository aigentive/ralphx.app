/**
 * ExecutionTaskDetail - macOS Tahoe-inspired execution view
 *
 * Live execution state with animated progress, step tracking,
 * and revision context when re-executing.
 */

import { Loader2, Radio, AlertTriangle, Bot, User, Zap } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { StepList } from "../StepList";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  ProgressIndicator,
  TwoColumnLayout,
} from "./shared";
import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";
import { reviewIssuesApi } from "@/api/review-issues";
import { IssueList } from "@/components/reviews/IssueList";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface ExecutionTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * Get the latest revision feedback from history
 */
function getLatestRevisionFeedback(
  history: ReviewNoteResponse[]
): ReviewNoteResponse | null {
  const revisionEntries = history.filter(
    (entry) => entry.outcome === "changes_requested"
  );
  if (revisionEntries.length === 0) return null;
  return revisionEntries[0] ?? null;
}

/**
 * RevisionFeedbackCard - Shows the feedback being addressed during re-execution
 */
function RevisionFeedbackCard({
  feedback,
  isLoading,
}: {
  feedback: ReviewNoteResponse | null;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-6">
        <Loader2
          className="w-5 h-5 animate-spin"
          style={{ color: "rgba(255,255,255,0.3)" }}
        />
      </div>
    );
  }

  if (!feedback) return null;

  const isAiReviewer = feedback.reviewer === "ai";

  return (
    <DetailCard variant="warning">
      <div className="flex items-start gap-3">
        {/* Reviewer icon */}
        <div
          className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
          style={{
            backgroundColor: isAiReviewer
              ? "rgba(10, 132, 255, 0.15)"
              : "rgba(52, 199, 89, 0.15)",
          }}
        >
          {isAiReviewer ? (
            <Bot className="w-4 h-4" style={{ color: "#0a84ff" }} />
          ) : (
            <User className="w-4 h-4" style={{ color: "#34c759" }} />
          )}
        </div>

        {/* Feedback content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-[12px] font-semibold text-white/70">
              {isAiReviewer ? "AI Feedback" : "Human Feedback"}
            </span>
            <StatusPill
              icon={AlertTriangle}
              label="Addressing"
              variant="warning"
              size="sm"
            />
          </div>
          <p className="text-[13px] text-white/55 leading-relaxed">
            {feedback.notes || "No specific feedback provided"}
          </p>
        </div>
      </div>
    </DetailCard>
  );
}

export function ExecutionTaskDetail({ task, isHistorical }: ExecutionTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: progress } = useStepProgress(task.id);
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(
    task.id,
    { enabled: task.internalStatus === "re_executing" }
  );

  // Fetch open issues when re-executing to show what needs to be addressed
  const { data: openIssues = [] } = useQuery({
    queryKey: ["review-issues", task.id, "open"],
    queryFn: () => reviewIssuesApi.getByTaskId(task.id, "open"),
    enabled: task.internalStatus === "re_executing",
  });

  const hasSteps = (steps?.length ?? 0) > 0;
  const isReExecuting = task.internalStatus === "re_executing";
  const revisionFeedback = isReExecuting
    ? getLatestRevisionFeedback(history ?? [])
    : null;

  const percentComplete = progress?.percentComplete ?? 0;
  const completed = progress?.completed ?? 0;
  const total = progress?.total ?? 0;

  return (
    <TwoColumnLayout
      description={task.description}
      testId="execution-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={isHistorical ? Zap : Radio}
        title={isHistorical ? "Execution Completed" : isReExecuting ? "Revising Task" : "Executing Task"}
        subtitle={isHistorical ? "This execution has finished" : "AI agent is actively working"}
        variant={isHistorical ? "success" : isReExecuting ? "warning" : "accent"}
        animated={!isHistorical}
        badge={
          <StatusPill
            icon={isHistorical ? Zap : Radio}
            label={isHistorical ? "Done" : isReExecuting ? "Revising" : "Live"}
            variant={isHistorical ? "success" : isReExecuting ? "warning" : "accent"}
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Progress Section */}
      {total > 0 && (
        <section data-testid="execution-progress-section">
          <SectionTitle>Progress</SectionTitle>
          <DetailCard>
            <ProgressIndicator
              percentComplete={percentComplete}
              completedSteps={completed}
              totalSteps={total}
              variant={isReExecuting ? "info" : "accent"}
            />
          </DetailCard>
        </section>
      )}

      {/* Revision Feedback (only for re-executing) */}
      {isReExecuting && (
        <section data-testid="revision-feedback-banner">
          <SectionTitle>Feedback Being Addressed</SectionTitle>
          <RevisionFeedbackCard
            feedback={revisionFeedback}
            isLoading={historyLoading}
          />
        </section>
      )}

      {/* Open Issues to Address (only for re-executing with issues) */}
      {isReExecuting && openIssues.length > 0 && (
        <section data-testid="open-issues-section">
          <SectionTitle>Issues to Address ({openIssues.length})</SectionTitle>
          <DetailCard>
            <IssueList issues={openIssues} groupBy="severity" compact />
          </DetailCard>
        </section>
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="execution-steps-loading"
          className="flex items-center justify-center py-8"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      )}

      {!stepsLoading && hasSteps && (
        <section data-testid="execution-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
