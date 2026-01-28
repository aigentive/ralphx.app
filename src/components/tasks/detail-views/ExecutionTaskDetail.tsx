/**
 * ExecutionTaskDetail - Task detail view for executing and re_executing states
 *
 * Shows execution progress with live indicator, progress bar, step tracker,
 * and revision context when re-executing based on review feedback.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Loader2, Radio, AlertTriangle, Bot, User } from "lucide-react";
import { StepList } from "../StepList";
import { SectionTitle } from "./shared";
import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import { useTaskStateHistory } from "@/hooks/useReviews";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface ExecutionTaskDetailProps {
  task: Task;
}

/**
 * LiveBadge - Animated indicator showing task is actively executing
 */
function LiveBadge({ isReExecuting }: { isReExecuting: boolean }) {
  const label = isReExecuting ? "Revising" : "Live";
  const bgColor = isReExecuting
    ? "rgba(245, 158, 11, 0.15)"
    : "rgba(239, 68, 68, 0.15)";
  const textColor = isReExecuting ? "var(--status-warning)" : "var(--status-error)";

  return (
    <div
      data-testid="execution-live-badge"
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{
        backgroundColor: bgColor,
        color: textColor,
      }}
    >
      <Radio
        className="w-3 h-3 animate-pulse"
        style={{ color: textColor }}
      />
      {label}
    </div>
  );
}

/**
 * ProgressBar - Visual progress indicator with percentage
 */
function ProgressBar({
  percentComplete,
  completed,
  total,
}: {
  percentComplete: number;
  completed: number;
  total: number;
}) {
  return (
    <div
      data-testid="execution-progress-section"
      className="space-y-2"
    >
      <div className="flex items-center justify-between text-[12px]">
        <span className="text-white/60">
          Progress: Step{" "}
          <span
            data-testid="execution-step-count"
            className="text-white/80 font-medium"
          >
            {completed} of {total}
          </span>
        </span>
        <span
          data-testid="execution-progress-text"
          className="text-white/80 font-medium"
        >
          {Math.round(percentComplete)}%
        </span>
      </div>
      <div
        data-testid="execution-progress-bar"
        className="h-1.5 rounded-full overflow-hidden"
        style={{ backgroundColor: "rgba(255,255,255,0.1)" }}
      >
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{
            width: `${percentComplete}%`,
            backgroundColor: "var(--accent-primary)",
          }}
        />
      </div>
    </div>
  );
}

/**
 * ReviewerIcon - Shows AI or Human icon based on reviewer type
 */
function ReviewerIcon({ reviewer }: { reviewer: string }) {
  const isAi = reviewer === "ai";
  return (
    <div
      className="flex items-center justify-center w-5 h-5 rounded-full shrink-0"
      style={{
        backgroundColor: isAi
          ? "rgba(59, 130, 246, 0.15)"
          : "rgba(16, 185, 129, 0.15)",
      }}
    >
      {isAi ? (
        <Bot
          className="w-3 h-3"
          style={{ color: "var(--status-info)" }}
        />
      ) : (
        <User
          className="w-3 h-3"
          style={{ color: "var(--status-success)" }}
        />
      )}
    </div>
  );
}

/**
 * RevisionFeedbackBanner - Shows review feedback being addressed
 */
function RevisionFeedbackBanner({
  feedback,
  isLoading,
}: {
  feedback: ReviewNoteResponse | null;
  isLoading: boolean;
}) {
  if (isLoading) {
    return (
      <div
        data-testid="revision-feedback-loading"
        className="flex justify-center py-3"
      >
        <Loader2
          className="w-5 h-5 animate-spin"
          style={{ color: "var(--text-muted)" }}
        />
      </div>
    );
  }

  if (!feedback) {
    return null;
  }

  return (
    <div
      data-testid="revision-feedback-banner"
      className="rounded-lg p-3 space-y-2"
      style={{
        backgroundColor: "rgba(245, 158, 11, 0.08)",
        border: "1px solid rgba(245, 158, 11, 0.2)",
      }}
    >
      <div className="flex items-center gap-2">
        <AlertTriangle
          className="w-4 h-4 shrink-0"
          style={{ color: "var(--status-warning)" }}
        />
        <span
          className="text-[12px] font-medium"
          style={{ color: "var(--status-warning)" }}
        >
          Addressing Review Feedback
        </span>
      </div>
      <div className="flex items-start gap-2 pl-6">
        <ReviewerIcon reviewer={feedback.reviewer} />
        <p className="text-[12px] text-white/60" style={{ lineHeight: "1.5" }}>
          {feedback.notes || "No specific feedback provided"}
        </p>
      </div>
    </div>
  );
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
 * ExecutionTaskDetail Component
 *
 * Renders task information for executing and re_executing states.
 * Shows: live indicator, progress bar, revision context (if re_executing),
 * step list with current step highlighted, and description.
 */
export function ExecutionTaskDetail({ task }: ExecutionTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: progress } = useStepProgress(task.id);
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(
    task.id,
    { enabled: task.internalStatus === "re_executing" }
  );

  const hasSteps = (steps?.length ?? 0) > 0;
  const isReExecuting = task.internalStatus === "re_executing";
  const revisionFeedback = isReExecuting
    ? getLatestRevisionFeedback(history ?? [])
    : null;

  // Calculate progress values
  const percentComplete = progress?.percentComplete ?? 0;
  const completed = progress?.completed ?? 0;
  const total = progress?.total ?? 0;

  return (
    <div
      data-testid="execution-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* Header: Title + Live Badge */}
      <div className="space-y-2">
        <div className="flex items-start justify-between gap-2">
          <h2
            data-testid="execution-task-title"
            className="text-base font-semibold text-white/90 flex-1"
            style={{
              letterSpacing: "-0.02em",
              lineHeight: "1.3",
            }}
          >
            {task.title}
          </h2>
          <LiveBadge isReExecuting={isReExecuting} />
        </div>
        <p className="text-[12px] text-white/50">
          Category: <span className="text-white/70">{task.category}</span>
        </p>
      </div>

      {/* Progress Bar */}
      {total > 0 && (
        <ProgressBar
          percentComplete={percentComplete}
          completed={completed}
          total={total}
        />
      )}

      {/* Revision Feedback Banner (only for re_executing) */}
      {isReExecuting && (
        <RevisionFeedbackBanner
          feedback={revisionFeedback}
          isLoading={historyLoading}
        />
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="execution-steps-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-6 h-6 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}
      {!stepsLoading && hasSteps && (
        <div data-testid="execution-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </div>
      )}

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="execution-task-description"
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
    </div>
  );
}
