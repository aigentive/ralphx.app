/**
 * CompletedTaskDetail - macOS Tahoe-inspired completed task view
 *
 * Shows final summary, review history, and actions for completed tasks.
 */

import { useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { ReviewTimeline } from "./shared/ReviewTimeline";
import { useTaskStateHistory } from "@/hooks/useReviews";
import {
  CheckCircle2,
  Loader2,
  ExternalLink,
  RefreshCw,
  User,
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
  isHistorical?: boolean;
}

function formatRelativeTime(date: Date | string | null | undefined): string {
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
 * ApprovalDetailsCard - Shows approval info (who, when, notes)
 */
function ApprovalDetailsCard({
  humanApproval,
  aiApproval,
  completedAt,
}: {
  humanApproval: ReviewNoteResponse | null;
  aiApproval: ReviewNoteResponse | null;
  completedAt: Date | string | null | undefined;
}) {
  const approval = humanApproval ?? aiApproval;
  const isHuman = humanApproval !== null;
  const approvalTime = approval
    ? formatRelativeTime(approval.created_at)
    : formatRelativeTime(completedAt);

  return (
    <DetailCard>
      <div className="space-y-3">
        {/* Approved by */}
        <div className="flex items-center gap-3">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-xl shrink-0"
            style={{
              backgroundColor: isHuman
                ? "rgba(52, 199, 89, 0.15)"
                : "rgba(10, 132, 255, 0.15)",
            }}
          >
            {isHuman ? (
              <User className="w-4 h-4" style={{ color: "#34c759" }} />
            ) : (
              <CheckCircle2 className="w-4 h-4" style={{ color: "#0a84ff" }} />
            )}
          </div>
          <div>
            <span className="text-[11px] uppercase tracking-wider text-white/40 block">
              Approved by
            </span>
            <span className="text-[13px] text-white/70 font-medium">
              {isHuman ? "Human Reviewer" : "AI Reviewer"}
            </span>
          </div>
          <span className="ml-auto text-[12px] text-white/40">
            {approvalTime}
          </span>
        </div>

        {/* Approval notes if present */}
        {approval?.notes && (
          <>
            <div
              className="h-px"
              style={{ backgroundColor: "rgba(255,255,255,0.06)" }}
            />
            <p className="text-[13px] text-white/55 leading-relaxed pl-11">
              "{approval.notes}"
            </p>
          </>
        )}
      </div>
    </DetailCard>
  );
}

/**
 * ActionButtonsCard - View Diff and Reopen actions
 */
function ActionButtonsCard({
  onViewDiff,
  onReopenTask,
}: {
  onViewDiff?: () => void;
  onReopenTask?: () => void;
}) {
  return (
    <div className="flex gap-2 justify-end">
      <Button
        data-testid="view-diff-button"
        onClick={onViewDiff}
        variant="ghost"
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "hsl(220 10% 70%)",
          backgroundColor: "hsl(220 10% 16%)",
        }}
      >
        <ExternalLink className="w-4 h-4" />
        View Final Diff
      </Button>
      <Button
        data-testid="reopen-task-button"
        onClick={onReopenTask}
        variant="ghost"
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "hsl(220 10% 70%)",
          backgroundColor: "hsl(220 10% 16%)",
        }}
      >
        <RefreshCw className="w-4 h-4" />
        Reopen Task
      </Button>
    </div>
  );
}

export function CompletedTaskDetail({ task, isHistorical = false }: CompletedTaskDetailProps) {
  const queryClient = useQueryClient();
  const { data: history, isLoading } = useTaskStateHistory(task.id);

  const [isRerunDialogOpen, setIsRerunDialogOpen] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { commits } = useGitDiff({ taskId: task.id });
  const latestCommit = commits[0];
  const commitInfo = {
    sha: latestCommit?.shortSha ?? "unknown",
    message: latestCommit?.message ?? "No commit info available",
    hasDependentCommits: commits.length > 1,
  };

  const { humanApproval, aiApproval } = getApprovalInfo(history);

  const handleViewDiff = () => {
    // Diff viewer not yet implemented
  };

  const handleReopenTask = () => {
    setError(null);
    setIsRerunDialogOpen(true);
  };

  const handleDialogClose = useCallback(() => {
    if (!isProcessing) {
      setIsRerunDialogOpen(false);
      setError(null);
    }
  }, [isProcessing]);

  const handleRerunConfirm = useCallback(
    async (result: TaskRerunResult) => {
      setIsProcessing(true);
      setError(null);

      try {
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
    <>
      <TwoColumnLayout
        description={task.description}
        testId="completed-task-detail"
      >
        {/* Status Banner */}
        <StatusBanner
          icon={CheckCircle2}
          title="Task Completed"
          subtitle="All work has been reviewed and approved"
          variant="success"
          badge={
            <StatusPill
              icon={CheckCircle2}
              label="Done"
              variant="success"
              size="md"
            />
          }
        />

        {/* Approval Details */}
        <section>
          <SectionTitle>Approval</SectionTitle>
          <ApprovalDetailsCard
            humanApproval={humanApproval}
            aiApproval={aiApproval}
            completedAt={task.completedAt}
          />
        </section>

        {/* Review History */}
        <section data-testid="review-history-section">
          <SectionTitle>Review History</SectionTitle>
          <DetailCard>
            <ReviewTimeline history={history} />
          </DetailCard>
        </section>

        {/* Actions (hidden in historical mode) */}
        {!isHistorical && (
          <section data-testid="action-buttons">
            <ActionButtonsCard
              onViewDiff={handleViewDiff}
              onReopenTask={handleReopenTask}
            />
          </section>
        )}
      </TwoColumnLayout>

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
    </>
  );
}
