/**
 * ReviewingTaskDetail - Task detail view for reviewing state
 *
 * Shows AI review in progress with review steps indicator and files under review.
 * The user can see the AI review progress and interact via the chat panel.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Loader2, Bot, FileCode2, CheckCircle2, Circle } from "lucide-react";
import { SectionTitle } from "./shared";
import type { Task } from "@/types/task";

interface ReviewingTaskDetailProps {
  task: Task;
  /** True when viewing a historical state - shows completed state instead of loading */
  isHistorical?: boolean;
}

/**
 * ReviewingBadge - Shows animated indicator for active review or completed for historical
 */
function ReviewingBadge({ isHistorical }: { isHistorical?: boolean | undefined }) {
  if (isHistorical) {
    return (
      <div
        data-testid="reviewing-badge"
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
        Completed
      </div>
    );
  }

  return (
    <div
      data-testid="reviewing-badge"
      className="flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium"
      style={{
        backgroundColor: "rgba(59, 130, 246, 0.15)",
        color: "var(--status-info)",
      }}
    >
      <Loader2
        className="w-3 h-3 animate-spin"
        style={{ color: "var(--status-info)" }}
      />
      Reviewing
    </div>
  );
}

/**
 * Review step status type
 */
type ReviewStepStatus = "completed" | "active" | "pending";

/**
 * ReviewStepItem - Individual step in the review process
 */
function ReviewStepItem({
  label,
  status,
}: {
  label: string;
  status: ReviewStepStatus;
}) {
  const getIcon = () => {
    switch (status) {
      case "completed":
        return (
          <CheckCircle2
            className="w-4 h-4 shrink-0"
            style={{ color: "var(--status-success)" }}
          />
        );
      case "active":
        return (
          <Loader2
            className="w-4 h-4 shrink-0 animate-spin"
            style={{ color: "var(--status-info)" }}
          />
        );
      case "pending":
        return (
          <Circle className="w-4 h-4 shrink-0" style={{ color: "rgba(255,255,255,0.3)" }} />
        );
    }
  };

  const getTextStyle = () => {
    switch (status) {
      case "completed":
        return "text-white/60";
      case "active":
        return "text-white/80 font-medium";
      case "pending":
        return "text-white/40";
    }
  };

  return (
    <div
      data-testid={`review-step-${label.toLowerCase().replace(/\s+/g, "-")}`}
      data-status={status}
      className="flex items-center gap-2.5 py-1.5"
    >
      {getIcon()}
      <span className={`text-[12px] ${getTextStyle()}`}>{label}</span>
    </div>
  );
}

/**
 * ReviewStepsIndicator - Shows progress through review phases
 *
 * Note: Currently uses simulated step status since we don't have real-time
 * review step tracking from the backend. The active step is shown as
 * "Examining changes" which is typically the main review activity.
 */
function ReviewStepsIndicator({ isHistorical }: { isHistorical?: boolean | undefined }) {
  // For now, we simulate review progress - in a full implementation,
  // this would come from the review process itself
  // When viewing historical state, show all steps as completed
  const steps: Array<{ label: string; status: ReviewStepStatus }> = isHistorical
    ? [
        { label: "Gathering context", status: "completed" },
        { label: "Examining changes", status: "completed" },
        { label: "Running checks", status: "completed" },
        { label: "Generating feedback", status: "completed" },
      ]
    : [
        { label: "Gathering context", status: "completed" },
        { label: "Examining changes", status: "active" },
        { label: "Running checks", status: "pending" },
        { label: "Generating feedback", status: "pending" },
      ];

  return (
    <div
      data-testid="review-steps-indicator"
      className="rounded-lg p-3"
      style={{
        backgroundColor: "rgba(0, 0, 0, 0.2)",
        border: isHistorical ? "1px solid rgba(16, 185, 129, 0.15)" : "1px solid rgba(59, 130, 246, 0.15)",
      }}
    >
      {steps.map((step, index) => (
        <ReviewStepItem key={index} label={step.label} status={step.status} />
      ))}
    </div>
  );
}

/**
 * FileItem - Individual file in the files under review list
 */
function FileItem({ path }: { path: string }) {
  return (
    <div
      className="flex items-center gap-2 py-1"
      style={{ color: "rgba(255,255,255,0.6)" }}
    >
      <FileCode2
        className="w-3.5 h-3.5 shrink-0"
        style={{ color: "var(--accent-primary)" }}
      />
      <span className="text-[12px] font-mono truncate">{path}</span>
    </div>
  );
}

/**
 * FilesUnderReview - Shows list of files being reviewed
 *
 * Note: Currently shows a placeholder since git diff data is not yet
 * available from the review process. In a full implementation, this
 * would show actual changed files from the git diff.
 */
function FilesUnderReview() {
  // Placeholder - in a full implementation, we'd fetch the actual
  // changed files from git diff via the review context
  const files: string[] = [];
  const hasFiles = files.length > 0;

  if (!hasFiles) {
    return (
      <div data-testid="files-under-review-empty">
        <SectionTitle>Files Under Review</SectionTitle>
        <p className="text-[12px] text-white/40 italic">
          File list will appear once review gathers context
        </p>
      </div>
    );
  }

  return (
    <div data-testid="files-under-review">
      <SectionTitle>Files Under Review</SectionTitle>
      <div className="space-y-0.5">
        {files.map((file, index) => (
          <FileItem key={index} path={file} />
        ))}
      </div>
    </div>
  );
}

/**
 * ReviewingTaskDetail Component
 *
 * Renders task information for reviewing state.
 * Shows: AI review banner, review steps progress, files under review, and description.
 * When isHistorical is true, shows a completed state instead of loading indicators.
 */
export function ReviewingTaskDetail({ task, isHistorical }: ReviewingTaskDetailProps) {
  return (
    <div
      data-testid="reviewing-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* AI Review Banner - shows completed state when historical */}
      <div
        data-testid="reviewing-banner"
        className="flex items-center gap-2 px-3 py-2 rounded-lg"
        style={{
          backgroundColor: isHistorical ? "rgba(16, 185, 129, 0.1)" : "rgba(59, 130, 246, 0.1)",
          border: isHistorical ? "1px solid rgba(16, 185, 129, 0.25)" : "1px solid rgba(59, 130, 246, 0.25)",
        }}
      >
        <Bot
          className="w-4 h-4 shrink-0"
          style={{ color: isHistorical ? "var(--status-success)" : "var(--status-info)" }}
        />
        <span
          className="text-[13px] font-medium"
          style={{ color: isHistorical ? "var(--status-success)" : "var(--status-info)" }}
        >
          {isHistorical ? "AI REVIEW COMPLETED" : "AI REVIEW IN PROGRESS"}
        </span>
        <div className="ml-auto">
          <ReviewingBadge isHistorical={isHistorical} />
        </div>
      </div>

      {/* Review Steps Indicator */}
      <div data-testid="reviewing-steps-section">
        <SectionTitle>Review Steps</SectionTitle>
        <ReviewStepsIndicator isHistorical={isHistorical} />
      </div>

      {/* Files Under Review */}
      <FilesUnderReview />

      {/* Description Section */}
      <div>
        <SectionTitle>Description</SectionTitle>
        {task.description ? (
          <p
            data-testid="reviewing-task-description"
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
