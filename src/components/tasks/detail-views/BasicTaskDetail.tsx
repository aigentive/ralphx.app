/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { useCallback, useMemo } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { formatDistanceToNow, parseISO } from "date-fns";
import { StepList } from "../StepList";
import { SectionTitle, TwoColumnLayout, DetailCard } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useConfirmation } from "@/hooks/useConfirmation";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { Loader2, RotateCcw, Clock } from "lucide-react";
import { Button } from "@/components/ui/button";
import { parseStopMetadata, type Task, type StopMetadata } from "@/types/task";

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Convert snake_case status to Title Case.
 * "merging" → "Merging"
 * "qa_testing" → "QA Testing"
 * "merge_conflict" → "Merge Conflict"
 */
function formatStatusLabel(status: string): string {
  return status
    .split("_")
    .map((word) => {
      // Handle common abbreviations
      if (word.toUpperCase() === word) return word;
      if (word === "qa") return "QA";
      return word.charAt(0).toUpperCase() + word.slice(1);
    })
    .join(" ");
}

/**
 * Get relative time string from ISO timestamp.
 * Returns "just now" for invalid dates.
 */
function getTimeAgo(isoString: string): string {
  try {
    const date = parseISO(isoString);
    return formatDistanceToNow(date, { addSuffix: true });
  } catch {
    return "just now";
  }
}

// Task statuses that can be restarted
const RESTARTABLE_STATUSES = new Set(["failed", "stopped", "cancelled", "paused"]);

interface BasicTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * StopHistorySection - Shows stop history for stopped tasks
 * Displays original state, stop reason, and time since stopped.
 */
function StopHistorySection({ stopMetadata }: { stopMetadata: StopMetadata }) {
  const fromStatusLabel = formatStatusLabel(stopMetadata.stoppedFromStatus);
  const timeAgo = getTimeAgo(stopMetadata.stoppedAt);

  return (
    <section data-testid="stop-history-section" className="space-y-2">
      <SectionTitle>Stop History</SectionTitle>
      <DetailCard>
        <div className="space-y-3">
          {/* Stopped From Status */}
          <div className="flex items-center gap-2">
            <span
              className="text-[11px] font-medium uppercase tracking-wider"
              style={{ color: "hsl(220 10% 50%)" }}
            >
              Stopped from
            </span>
            <span
              className="text-[13px] font-medium px-2 py-0.5 rounded"
              style={{
                backgroundColor: "hsl(38 92% 50% / 0.15)",
                color: "hsl(38 92% 65%)",
              }}
            >
              {fromStatusLabel}
            </span>
          </div>

          {/* Stop Reason (if provided) */}
          {stopMetadata.stopReason && (
            <div className="mt-2">
              <span
                className="text-[11px] font-medium uppercase tracking-wider block mb-1"
                style={{ color: "hsl(220 10% 50%)" }}
              >
                Reason
              </span>
              <p className="text-[13px]" style={{ color: "hsl(220 10% 80%)" }}>
                {stopMetadata.stopReason}
              </p>
            </div>
          )}

          {/* Time Ago */}
          <div className="flex items-center gap-2 mt-2">
            <Clock className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 50%)" }} />
            <span className="text-[12px]" style={{ color: "hsl(220 10% 60%)" }}>
              {timeAgo}
            </span>
          </div>
        </div>
      </DetailCard>
    </section>
  );
}

/**
 * ActionButtonsCard - Restart button for terminal/suspended states
 * For stopped tasks with stop metadata, shows enhanced confirmation dialog.
 */
function ActionButtonsCard({ task }: { task: Task }) {
  const queryClient = useQueryClient();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const taskId = task.id;
  const status = task.internalStatus;

  // Parse stop metadata for enhanced confirmation dialog
  const stopMetadata = useMemo(
    () => parseStopMetadata(task.metadata),
    [task.metadata]
  );

  const isStopped = status === "stopped" && stopMetadata !== null;

  const restartMutation = useMutation({
    mutationFn: async () => {
      if (isStopped) {
        // Use smart restart for stopped tasks via API layer
        const result = await api.tasks.restart(taskId, false);
        if (result.type === "ValidationFailed") {
          throw new Error(
            `Validation failed: ${result.warnings.map((w) => w.message).join(", ")}`
          );
        }
        return result;
      } else {
        // Fallback to move-to-ready for other restartable statuses
        return await api.tasks.move(taskId, "ready");
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
    },
  });

  const handleRestart = useCallback(async () => {
    const statusLabels: Record<string, string> = {
      failed: "Restart",
      stopped: "Restart",
      cancelled: "Restart",
      paused: "Resume",
    };
    const actionLabel = statusLabels[status] || "Restart";
    const taskLabel = status === "paused" ? "paused task" : "failed task";

    // Build enhanced confirmation for stopped tasks with metadata
    if (isStopped && stopMetadata) {
      const fromStatusLabel = formatStatusLabel(stopMetadata.stoppedFromStatus);
      const timeAgo = getTimeAgo(stopMetadata.stoppedAt);

      const descriptionParts = [
        `Original state: ${fromStatusLabel}`,
        stopMetadata.stopReason && `Reason: ${stopMetadata.stopReason}`,
        `Stopped ${timeAgo}`,
        "",
        "The task will resume with smart state restoration.",
      ];
      const description = descriptionParts.filter(Boolean).join("\n");

      const confirmed = await confirm({
        title: `Restart this stopped task?`,
        description,
        confirmText: actionLabel,
        variant: "default",
      });
      if (!confirmed) return;
    } else {
      const confirmed = await confirm({
        title: `${actionLabel} this ${taskLabel}?`,
        description: `The task will be moved to ready status and can be executed again.`,
        confirmText: actionLabel,
        variant: "default",
      });
      if (!confirmed) return;
    }

    restartMutation.mutate();
  }, [confirm, status, isStopped, stopMetadata, restartMutation]);

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
          disabled={restartMutation.isPending}
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px] transition-colors"
          style={{
            backgroundColor: "hsl(217 90% 60%)",
            color: "white",
          }}
        >
          {restartMutation.isPending ? (
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
    </DetailCard>
  );
}

export function BasicTaskDetail({ task, isHistorical = false }: BasicTaskDetailProps) {
  const { data: steps, isLoading: stepsLoading } = useTaskSteps(task.id);
  const hasSteps = (steps?.length ?? 0) > 0;
  const isRestartable = RESTARTABLE_STATUSES.has(task.internalStatus);

  // Parse stop metadata for stopped tasks
  const stopMetadata = useMemo(
    () => parseStopMetadata(task.metadata),
    [task.metadata]
  );
  const isStopped = task.internalStatus === "stopped" && stopMetadata !== null;

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
      {/* Stop History Section (for stopped tasks with metadata) */}
      {isStopped && stopMetadata && (
        <StopHistorySection stopMetadata={stopMetadata} />
      )}

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
          <ActionButtonsCard task={task} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
