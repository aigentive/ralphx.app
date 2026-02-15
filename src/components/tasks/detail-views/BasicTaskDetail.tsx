/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { useCallback, useState, useMemo } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { StepList } from "../StepList";
import { SectionTitle, TwoColumnLayout, DetailCard } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useConfirmation } from "@/hooks/useConfirmation";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { Loader2, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  ResumeValidationDialog,
  type ValidationWarning,
} from "@/components/ui/ResumeValidationDialog";
import type { Task, StopMetadata, TaskMetadata } from "@/types/task";

// Task statuses that can be restarted
const RESTARTABLE_STATUSES = new Set(["failed", "stopped", "cancelled", "paused"]);

// States that need validation before resuming (merge-related states)
const VALIDATED_RESUME_STATES = new Set([
  "merging",
  "pending_merge",
  "merge_conflict",
  "merge_incomplete",
]);

interface BasicTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * ActionButtonsCard - Restart button for terminal/suspended states
 */
function ActionButtonsCard({
  taskId,
  taskTitle,
  status,
  stopMetadata,
}: {
  taskId: string;
  taskTitle: string;
  status: string;
  stopMetadata?: StopMetadata | null;
}) {
  const queryClient = useQueryClient();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [showValidationDialog, setShowValidationDialog] = useState(false);
  const [isResuming, setIsResuming] = useState(false);

  const restartMutation = useMutation({
    mutationFn: async () => {
      await api.tasks.move(taskId, "ready");
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
    },
  });

  // Generate validation warnings based on stopped-from state
  const validationWarnings = useMemo((): ValidationWarning[] => {
    if (!stopMetadata) return [];

    const warnings: ValidationWarning[] = [];
    const stoppedFrom = stopMetadata.stopped_from_status;

    // Check if this was a merge-related state
    if (VALIDATED_RESUME_STATES.has(stoppedFrom)) {
      warnings.push({
        id: "git-state",
        message: `Task was stopped during ${stoppedFrom.replace("_", " ")} phase. Git state may have changed since then.`,
        severity: "warning",
      });

      warnings.push({
        id: "branch-check",
        message: "The task branch and worktree should be verified before resuming.",
        severity: "warning",
      });
    }

    // Add stop reason as context if available
    if (stopMetadata.stop_reason) {
      warnings.push({
        id: "stop-reason",
        message: `Original stop reason: "${stopMetadata.stop_reason}"`,
        severity: "warning",
      });
    }

    return warnings;
  }, [stopMetadata]);

  const handleRestart = useCallback(async () => {
    // If task was stopped from a validated state, show validation dialog
    if (stopMetadata && validationWarnings.length > 0) {
      setShowValidationDialog(true);
      return;
    }

    // Otherwise, use standard confirmation
    const statusLabels: Record<string, string> = {
      failed: "Restart",
      stopped: "Restart",
      cancelled: "Restart",
      paused: "Resume",
    };
    const actionLabel = statusLabels[status] || "Restart";
    const taskLabel = status === "paused" ? "paused task" : "failed task";

    const confirmed = await confirm({
      title: `${actionLabel} this ${taskLabel}?`,
      description: `The task will be moved to ready status and can be executed again.`,
      confirmText: actionLabel,
      variant: "default",
    });
    if (!confirmed) return;
    restartMutation.mutate();
  }, [confirm, status, restartMutation, stopMetadata, validationWarnings.length]);

  // Handle force resume from validation dialog
  const handleForceResume = useCallback(async () => {
    setIsResuming(true);
    try {
      await api.tasks.move(taskId, "ready");
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowValidationDialog(false);
    } finally {
      setIsResuming(false);
    }
  }, [taskId, queryClient]);

  // Handle go to ready from validation dialog
  const handleGoToReady = useCallback(async () => {
    setIsResuming(true);
    try {
      await api.tasks.move(taskId, "ready");
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowValidationDialog(false);
    } finally {
      setIsResuming(false);
    }
  }, [taskId, queryClient]);

  return (
    <DetailCard data-testid="action-buttons">
      <div className="flex items-center justify-between">
        <span
          className="text-[11px] font-semibold uppercase tracking-wider"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          Actions
        </span>
        <Button
          data-testid="restart-button"
          onClick={handleRestart}
          disabled={restartMutation.isPending || isResuming}
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px] transition-colors"
          style={{
            backgroundColor: "hsl(217 90% 60%)",
            color: "white",
          }}
        >
          {restartMutation.isPending || isResuming ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <RotateCcw className="w-4 h-4" />
          )}
          {status === "paused" ? "Resume" : "Restart"}
        </Button>
      </div>

      {/* Error display */}
      {restartMutation.error && (
        <p className="mt-3 text-[12px]" style={{ color: "#ff453a" }}>
          {restartMutation.error.message}
        </p>
      )}

      <ConfirmationDialog {...confirmationDialogProps} />

      {/* Resume Validation Dialog */}
      <ResumeValidationDialog
        isOpen={showValidationDialog}
        onClose={() => setShowValidationDialog(false)}
        onForceResume={handleForceResume}
        onGoToReady={handleGoToReady}
        taskTitle={taskTitle}
        stoppedFromStatus={stopMetadata?.stopped_from_status}
        warnings={validationWarnings}
        isLoading={isResuming}
      />
    </DetailCard>
  );
}

export function BasicTaskDetail({ task, isHistorical = false }: BasicTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;
  const isRestartable = RESTARTABLE_STATUSES.has(task.internalStatus);

  // Parse stop metadata from task (for smart resume validation)
  let stopMetadata: StopMetadata | null = null;
  if (task.metadata) {
    try {
      const metadata: TaskMetadata = JSON.parse(task.metadata);
      if (metadata.stop) {
        stopMetadata = metadata.stop;
      }
    } catch {
      // JSON parse failed - ignore
    }
  }

  // Parse failure info from task metadata when task is failed or qa_failed
  let failureInfo: {
    failure_error: string;
    failure_details?: string;
    is_timeout: boolean;
  } | null = null;

  const isFailed = task.internalStatus === "failed" || task.internalStatus === "qa_failed";

  if (isFailed) {
    if (task.metadata) {
      try {
        const metadata = JSON.parse(task.metadata);
        if (metadata.failure_error) {
          failureInfo = {
            failure_error: metadata.failure_error,
            failure_details: metadata.failure_details,
            is_timeout: metadata.is_timeout || false,
          };
        }
      } catch {
        // JSON parse failed - fall through to fallback
      }
    }

    // Fallback handling for null/invalid metadata
    if (!failureInfo) {
      // Try blockedReason first (set by ExecutionBlocked handler)
      const errorMessage = task.blockedReason ||
        "Task execution failed. Error details were not recorded during the state transition.";
      failureInfo = {
        failure_error: errorMessage,
        is_timeout: false,
      };
    }
  }

  return (
    <TwoColumnLayout
      description={task.description}
      testId="basic-task-detail"
    >
      {/* Failure Reason Banner */}
      {failureInfo && (
        <section data-testid="failure-reason-section" className="space-y-2">
          <SectionTitle>Failure Reason</SectionTitle>
          <div className="rounded-md bg-red-500/10 p-3 text-[13px] text-red-400">
            <div className="flex items-start gap-2">
              <div className="flex-1">
                {failureInfo.failure_error}
                {failureInfo.is_timeout && (
                  <span className="ml-2 inline-block text-[11px] bg-red-500/20 px-2 py-0.5 rounded">
                    timeout
                  </span>
                )}
              </div>
            </div>
            {failureInfo.failure_details && (
              <p className="mt-2 text-[12px] text-red-400/70">
                {failureInfo.failure_details}
              </p>
            )}
          </div>
        </section>
      )}

      {/* Steps Section */}
      {stepsLoading && (
        <div
          data-testid="basic-task-steps-loading"
          className="flex items-center justify-center py-8"
        >
          <Loader2
            className="w-5 h-5 animate-spin"
            style={{ color: "rgba(255,255,255,0.3)" }}
          />
        </div>
      )}

      {!stepsLoading && hasSteps && (
        <section data-testid="basic-task-steps-section">
          <SectionTitle>Steps</SectionTitle>
          <StepList taskId={task.id} editable={false} hideCompletionNotes={isHistorical} />
        </section>
      )}

      {!stepsLoading && !hasSteps && (
        <div className="text-[13px] text-white/40 italic py-4">
          No steps defined yet
        </div>
      )}

      {/* Action Buttons (hidden in historical mode) */}
      {!isHistorical && isRestartable && (
        <section>
          <ActionButtonsCard
            taskId={task.id}
            taskTitle={task.title}
            status={task.internalStatus}
            stopMetadata={stopMetadata}
          />
        </section>
      )}
    </TwoColumnLayout>
  );
}
