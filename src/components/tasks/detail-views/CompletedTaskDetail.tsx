/**
 * CompletedTaskDetail - Task detail view for approved state
 *
 * Shows completed task with approval info, final summary, review history timeline,
 * and action buttons for viewing diff or reopening task.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { Button } from "@/components/ui/button";
import { SectionTitle } from "./shared";
import { useTaskStateHistory } from "@/hooks/useReviews";
import {
  CheckCircle2,
  Loader2,
  Bot,
  User,
  RotateCcw,
  ExternalLink,
  RefreshCw,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface CompletedTaskDetailProps {
  task: Task;
}

/**
 * CompletedBadge - Shows green indicator for completed status
 */
function CompletedBadge() {
  return (
    <div
      data-testid="completed-badge"
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
      Done
    </div>
  );
}

/**
 * Format relative time from date
 */
function formatRelativeTime(date: Date | string | undefined): string {
  if (!date) return "Unknown";

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
 * Get approval info from history
 */
function getApprovalInfo(history: ReviewNoteResponse[]): {
  humanApproval: ReviewNoteResponse | null;
  aiApproval: ReviewNoteResponse | null;
} {
  const approvedEntries = history.filter((entry) => entry.outcome === "approved");

  const humanApproval = approvedEntries.find((e) => e.reviewer === "human") ?? null;
  const aiApproval = approvedEntries.find((e) => e.reviewer === "ai") ?? null;

  return { humanApproval, aiApproval };
}

/**
 * HistoryTimelineItem - Individual item in the review history timeline
 */
function HistoryTimelineItem({
  entry,
  isLast,
}: {
  entry: ReviewNoteResponse;
  isLast: boolean;
}) {
  const isApproved = entry.outcome === "approved";
  const isChangesRequested = entry.outcome === "changes_requested";
  const isHuman = entry.reviewer === "human";

  const getIconAndColor = () => {
    if (isApproved) {
      return {
        Icon: CheckCircle2,
        color: "var(--status-success)",
        bgColor: "rgba(16, 185, 129, 0.15)",
      };
    }
    if (isChangesRequested) {
      return {
        Icon: RotateCcw,
        color: "var(--status-warning)",
        bgColor: "rgba(245, 158, 11, 0.15)",
      };
    }
    return {
      Icon: CheckCircle2,
      color: "rgba(255,255,255,0.5)",
      bgColor: "rgba(255,255,255,0.08)",
    };
  };

  const { Icon, color, bgColor } = getIconAndColor();
  const ReviewerIcon = isHuman ? User : Bot;

  const getLabel = () => {
    if (isApproved) {
      return `${isHuman ? "Human" : "AI"} approved`;
    }
    if (isChangesRequested) {
      return `${isHuman ? "Human" : "AI"} changes requested`;
    }
    return `${isHuman ? "Human" : "AI"} reviewed`;
  };

  return (
    <div className="flex gap-3">
      {/* Timeline line and dot */}
      <div className="flex flex-col items-center">
        <div
          className="flex items-center justify-center w-6 h-6 rounded-full shrink-0"
          style={{ backgroundColor: bgColor }}
        >
          <Icon className="w-3.5 h-3.5" style={{ color }} />
        </div>
        {!isLast && (
          <div
            className="w-px flex-1 min-h-[16px]"
            style={{ backgroundColor: "rgba(255,255,255,0.1)" }}
          />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 pb-3">
        <div className="flex items-center gap-2">
          <ReviewerIcon
            className="w-3.5 h-3.5"
            style={{ color: "rgba(255,255,255,0.5)" }}
          />
          <span className="text-[12px] font-medium text-white/70">
            {getLabel()}
          </span>
          <span className="text-[11px] text-white/40">
            {formatRelativeTime(entry.created_at)}
          </span>
        </div>
        {entry.notes && (
          <p className="text-[11px] text-white/50 mt-1 pl-5">
            {entry.notes}
          </p>
        )}
      </div>
    </div>
  );
}

/**
 * ReviewHistoryTimeline - Shows timeline of review events
 */
function ReviewHistoryTimeline({ history }: { history: ReviewNoteResponse[] }) {
  if (history.length === 0) {
    return (
      <p className="text-[12px] text-white/40 italic">
        No review history available
      </p>
    );
  }

  return (
    <div data-testid="review-history-timeline">
      {history.map((entry, index) => (
        <HistoryTimelineItem
          key={entry.id}
          entry={entry}
          isLast={index === history.length - 1}
        />
      ))}
    </div>
  );
}

/**
 * ActionButtons - View Diff and Reopen Task buttons
 */
function ActionButtons({
  onViewDiff,
  onReopenTask,
}: {
  onViewDiff?: () => void;
  onReopenTask?: () => void;
}) {
  return (
    <div data-testid="action-buttons" className="flex items-center gap-2">
      <Button
        data-testid="view-diff-button"
        onClick={onViewDiff}
        variant="outline"
        className="flex-1 gap-1.5"
        style={{
          borderColor: "rgba(255,255,255,0.15)",
          color: "rgba(255,255,255,0.7)",
        }}
      >
        <ExternalLink className="w-4 h-4" />
        View Final Diff
      </Button>
      <Button
        data-testid="reopen-task-button"
        onClick={onReopenTask}
        variant="outline"
        className="flex-1 gap-1.5"
        style={{
          borderColor: "rgba(255,255,255,0.15)",
          color: "rgba(255,255,255,0.7)",
        }}
      >
        <RefreshCw className="w-4 h-4" />
        Reopen Task
      </Button>
    </div>
  );
}

/**
 * CompletedTaskDetail Component
 *
 * Renders task information for approved state.
 * Shows: completed banner, final summary, review history timeline, and action buttons.
 */
export function CompletedTaskDetail({ task }: CompletedTaskDetailProps) {
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(
    task.id
  );

  const { humanApproval } = getApprovalInfo(history);

  const handleViewDiff = () => {
    console.warn("Diff viewer not yet implemented");
  };

  const handleReopenTask = () => {
    // TODO: Transition task back to ready state
  };

  const approvalTimeDisplay = humanApproval
    ? formatRelativeTime(humanApproval.created_at)
    : task.completedAt
      ? formatRelativeTime(task.completedAt)
      : "Unknown";

  return (
    <div
      data-testid="completed-task-detail"
      data-task-id={task.id}
      className="space-y-5"
    >
      {/* Completed Banner */}
      <div
        data-testid="completed-banner"
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
            COMPLETED
          </span>
          <span className="text-[12px] text-white/50 ml-2">
            Approved {approvalTimeDisplay}
            {humanApproval ? " by Human" : ""}
          </span>
        </div>
        <CompletedBadge />
      </div>

      {/* Header: Title */}
      <div className="space-y-1">
        <h2
          data-testid="completed-task-title"
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

      {/* Final Summary Section */}
      <div>
        <SectionTitle>Final Summary</SectionTitle>
        {task.description ? (
          <p
            data-testid="completed-task-summary"
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

      {/* Loading state for history */}
      {historyLoading && (
        <div
          data-testid="completed-history-loading"
          className="flex justify-center py-4"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "var(--text-muted)" }}
          />
        </div>
      )}

      {/* Review History Section */}
      {!historyLoading && (
        <div data-testid="review-history-section">
          <SectionTitle>Review History</SectionTitle>
          <div
            className="rounded-lg p-3"
            style={{
              backgroundColor: "rgba(0, 0, 0, 0.2)",
              border: "1px solid rgba(255,255,255,0.08)",
            }}
          >
            <ReviewHistoryTimeline history={history} />
          </div>
        </div>
      )}

      {/* Action Buttons */}
      <ActionButtons onViewDiff={handleViewDiff} onReopenTask={handleReopenTask} />
    </div>
  );
}
