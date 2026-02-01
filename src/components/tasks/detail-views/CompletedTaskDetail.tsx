/**
 * CompletedTaskDetail - Task detail view for approved state
 *
 * Shows completed task with approval info, final summary, review history timeline,
 * and action buttons for viewing diff or reopening task.
 *
 * Part of the View Registry Pattern for state-specific task detail views.
 */

import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { SectionTitle, ReviewTimeline } from "./shared";
import { useTaskStateHistory } from "@/hooks/useReviews";
import {
  CheckCircle2,
  Loader2,
  ExternalLink,
  RefreshCw,
} from "lucide-react";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";
import { api } from "@/lib/tauri";
import { useQueryClient } from "@tanstack/react-query";
import { taskKeys } from "@/hooks/useTasks";
import {
  TaskRerunDialog,
  type TaskRerunResult,
} from "@/components/tasks/TaskRerunDialog";
import { useGitDiff } from "@/hooks/useGitDiff";

interface CompletedTaskDetailProps {
  task: Task;
  /** True when viewing a historical state - disables action buttons */
  isHistorical?: boolean;
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
export function CompletedTaskDetail({ task, isHistorical = false }: CompletedTaskDetailProps) {
  const queryClient = useQueryClient();
  const { data: history, isLoading: historyLoading } = useTaskStateHistory(
    task.id
  );

  // Dialog state
  const [isRerunDialogOpen, setIsRerunDialogOpen] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Git diff data for commit info
  const { commits } = useGitDiff({ taskId: task.id });

  // Build commitInfo from the latest commit
  const latestCommit = commits[0];
  const commitInfo = {
    sha: latestCommit?.shortSha ?? "unknown",
    message: latestCommit?.message ?? "No commit info available",
    hasDependentCommits: commits.length > 1,
  };

  const { humanApproval } = getApprovalInfo(history);

  const handleViewDiff = () => {
    // Diff viewer not yet implemented
  };

  // Open the dialog instead of directly moving the task
  const handleReopenTask = () => {
    setError(null);
    setIsRerunDialogOpen(true);
  };

  // Handle dialog close
  const handleDialogClose = useCallback(() => {
    if (!isProcessing) {
      setIsRerunDialogOpen(false);
      setError(null);
    }
  }, [isProcessing]);

  // Handle rerun confirmation
  const handleRerunConfirm = useCallback(
    async (result: TaskRerunResult) => {
      setIsProcessing(true);
      setError(null);

      try {
        // All options currently move to ready (full revert/duplicate is future work)
        switch (result.option) {
          case "keep_changes":
          case "revert_commit":
          case "create_new":
            await api.tasks.move(task.id, "ready");
            break;
        }

        await queryClient.invalidateQueries({
          queryKey: taskKeys.list(task.projectId),
        });
        setIsRerunDialogOpen(false);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to reopen task");
      } finally {
        setIsProcessing(false);
      }
    },
    [task.id, task.projectId, queryClient]
  );

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
            <ReviewTimeline history={history} />
          </div>
        </div>
      )}

      {/* Action Buttons - hidden in historical mode */}
      {!isHistorical && (
        <ActionButtons onViewDiff={handleViewDiff} onReopenTask={handleReopenTask} />
      )}

      {/* Task Rerun Dialog */}
      <TaskRerunDialog
        isOpen={isRerunDialogOpen}
        onClose={handleDialogClose}
        onConfirm={handleRerunConfirm}
        task={task}
        commitInfo={commitInfo}
        isProcessing={isProcessing}
        error={error}
      />
    </div>
  );
}
