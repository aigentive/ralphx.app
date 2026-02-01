/**
 * WaitingTaskDetail - Task detail view for pending_review state
 *
 * Shows task waiting for AI reviewer with work completed summary,
 * step list (all completed), and description.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Clock, CheckCircle2, FileCode2, Loader2 } from "lucide-react";
import { StepList } from "../StepList";
import { SectionTitle } from "./shared";
import { useTaskSteps, useStepProgress } from "@/hooks/useTaskSteps";
import type { Task } from "@/types/task";

interface WaitingTaskDetailProps {
  task: Task;
}

/**
 * PendingReviewBadge - Shows waiting status indicator
 */
function PendingReviewBadge() {
  return (
    <div
      data-testid="pending-review-badge"
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{
        backgroundColor: "rgba(255,255,255,0.08)",
        color: "rgba(255,255,255,0.6)",
      }}
    >
      <Clock
        className="w-3 h-3"
        style={{ color: "rgba(255,255,255,0.5)" }}
      />
      Pending Review
    </div>
  );
}

/**
 * Format relative time from date
 */
function formatRelativeTime(date: Date | string | undefined): string {
  if (!date) return "Just now";

  const now = new Date();
  const then = typeof date === "string" ? new Date(date) : date;
  const diffMs = now.getTime() - then.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

/**
 * WorkCompletedSection - Shows summary of completed work
 */
function WorkCompletedSection({
  submittedAt,
  stepsCompleted,
  totalSteps,
  isLoading,
}: {
  submittedAt: Date | string | undefined;
  stepsCompleted: number;
  totalSteps: number;
  isLoading: boolean;
}) {
  const allStepsCompleted = stepsCompleted === totalSteps && totalSteps > 0;

  return (
    <div
      data-testid="work-completed-section"
      className="rounded-lg p-3 space-y-2"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: "1px solid rgba(255,255,255,0.08)",
      }}
    >
      {isLoading ? (
        <div className="flex justify-center py-2">
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      ) : (
        <>
          {/* Submitted time */}
          <div className="flex items-center gap-2">
            <Clock
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: "rgba(255,255,255,0.4)" }}
            />
            <span
              data-testid="submitted-time"
              className="text-[12px] text-white/60"
            >
              Submitted {formatRelativeTime(submittedAt)}
            </span>
          </div>

          {/* Files changed - placeholder since git diff not available */}
          <div className="flex items-center gap-2">
            <FileCode2
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: "rgba(255,255,255,0.4)" }}
            />
            <span className="text-[12px] text-white/60">
              Files changed info pending
            </span>
          </div>

          {/* Steps completed indicator */}
          <div className="flex items-center gap-2">
            {allStepsCompleted ? (
              <CheckCircle2
                className="w-3.5 h-3.5 shrink-0"
                style={{ color: "var(--status-success)" }}
              />
            ) : (
              <CheckCircle2
                className="w-3.5 h-3.5 shrink-0"
                style={{ color: "rgba(255,255,255,0.4)" }}
              />
            )}
            <span
              data-testid="steps-completed-indicator"
              className="text-[12px]"
              style={{
                color: allStepsCompleted
                  ? "var(--status-success)"
                  : "rgba(255,255,255,0.6)",
              }}
            >
              {totalSteps > 0
                ? allStepsCompleted
                  ? "All steps completed"
                  : `${stepsCompleted} of ${totalSteps} steps completed`
                : "No steps defined"}
            </span>
          </div>
        </>
      )}
    </div>
  );
}

/**
 * WaitingTaskDetail Component
 *
 * Renders task information for pending_review state.
 * Shows: waiting banner, work completed summary, steps (all completed), and description.
 */
export function WaitingTaskDetail({ task }: WaitingTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const { data: progress } = useStepProgress(task.id);

  const hasSteps = (steps?.length ?? 0) > 0;
  const stepsCompleted = progress?.completed ?? 0;
  const totalSteps = progress?.total ?? 0;

  return (
    <div
      data-testid="waiting-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* Waiting for AI Reviewer Banner */}
      <div
        data-testid="waiting-banner"
        className="flex items-center gap-2 px-3 py-2 rounded-lg"
        style={{
          backgroundColor: "rgba(255,255,255,0.05)",
          border: "1px solid rgba(255,255,255,0.1)",
        }}
      >
        <Clock
          className="w-4 h-4 shrink-0"
          style={{ color: "rgba(255,255,255,0.5)" }}
        />
        <span className="text-[13px] font-medium text-white/70">
          WAITING FOR AI REVIEWER
        </span>
        <div className="ml-auto">
          <PendingReviewBadge />
        </div>
      </div>

      {/* Work Completed Section */}
      <div data-testid="work-completed-wrapper">
        <SectionTitle>Work Completed</SectionTitle>
        <WorkCompletedSection
          submittedAt={task.updatedAt}
          stepsCompleted={stepsCompleted}
          totalSteps={totalSteps}
          isLoading={stepsLoading}
        />
      </div>

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="waiting-steps-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-6 h-6 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}
      {!stepsLoading && hasSteps && (
        <div data-testid="waiting-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} />
        </div>
      )}

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="waiting-task-description"
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
