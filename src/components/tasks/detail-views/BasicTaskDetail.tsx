/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { useCallback, useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { formatDistanceToNow, parseISO } from "date-fns";
import { StepList } from "../StepList";
import { SectionTitle, TwoColumnLayout, DetailCard } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useConfirmation } from "@/hooks/useConfirmation";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { Loader2, Play, RotateCcw, Clock, User, Users, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  ResumeValidationDialog,
  type ValidationWarning,
} from "@/components/ui/ResumeValidationDialog";
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

type ExecutionMode = "solo" | "team";

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
 * ExecutionModeSelector - Solo/Team radio toggle for execution mode
 */
function ExecutionModeSelector({
  mode,
  onChange,
  disabled,
}: {
  mode: ExecutionMode;
  onChange: (mode: ExecutionMode) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center gap-1" data-testid="execution-mode-selector">
      <span
        className="text-[11px] font-medium mr-1.5"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        Mode
      </span>
      {(["solo", "team"] as const).map((m) => {
        const isSelected = mode === m;
        const Icon = m === "solo" ? User : Users;
        return (
          <button
            key={m}
            data-testid={`mode-${m}`}
            type="button"
            disabled={disabled}
            onClick={() => onChange(m)}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[12px] font-medium transition-colors disabled:opacity-40"
            style={{
              backgroundColor: isSelected
                ? m === "team"
                  ? "hsla(14 100% 60% / 0.15)"
                  : "hsla(220 10% 100% / 0.08)"
                : "transparent",
              color: isSelected
                ? m === "team"
                  ? "hsl(14 100% 60%)"
                  : "hsl(220 10% 80%)"
                : "hsl(220 10% 45%)",
              border: `1px solid ${isSelected ? (m === "team" ? "hsla(14 100% 60% / 0.3)" : "hsla(220 10% 100% / 0.12)") : "transparent"}`,
            }}
          >
            <Icon className="w-3 h-3" />
            {m === "solo" ? "Solo" : "Team"}
          </button>
        );
      })}
    </div>
  );
}

/**
 * ActionButtonsCard - Restart button for terminal/suspended states
 * For stopped tasks with stop metadata, shows enhanced confirmation dialog.
 * Supports execution mode selection (solo/team) for start/restart operations.
 */
function ActionButtonsCard({ task }: { task: Task }) {
  const queryClient = useQueryClient();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [executionMode, setExecutionMode] = useState<ExecutionMode>("solo");
  const [showValidationDialog, setShowValidationDialog] = useState(false);
  const [isResuming, setIsResuming] = useState(false);
  const [restartNote, setRestartNote] = useState("");
  const taskId = task.id;
  const status = task.internalStatus;

  // Parse stop metadata for enhanced confirmation dialog
  const stopMetadata = useMemo(
    () => parseStopMetadata(task.metadata),
    [task.metadata]
  );

  const isStopped = status === "stopped" && stopMetadata !== null;
  const isReady = status === "ready";

  // Generate validation warnings based on stopped-from state
  const validationWarnings = useMemo((): ValidationWarning[] => {
    if (!stopMetadata) return [];

    const warnings: ValidationWarning[] = [];
    const stoppedFrom = stopMetadata.stoppedFromStatus;

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
    if (stopMetadata.stopReason) {
      warnings.push({
        id: "stop-reason",
        message: `Original stop reason: "${stopMetadata.stopReason}"`,
        severity: "warning",
      });
    }

    return warnings;
  }, [stopMetadata]);

  const restartMutation = useMutation({
    mutationFn: async () => {
      const note = restartNote.trim() || undefined;
      if (isStopped) {
        // Use smart restart for stopped tasks via API layer
        const result = await api.tasks.restart(taskId, false, note);
        if (result.type === "ValidationFailed") {
          throw new Error(
            `Validation failed: ${result.warnings.map((w) => w.message).join(", ")}`
          );
        }
        return result;
      } else {
        // Fallback to move-to-ready for other restartable statuses
        return await api.tasks.move(
          taskId,
          "ready",
          executionMode === "team" ? "team" : undefined,
          note
        );
      }
    },
    onSuccess: () => {
      setRestartNote("");
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
    },
  });

  // Handle force resume from validation dialog - restores to original state (smart resume)
  const handleForceResume = useCallback(async () => {
    if (!stopMetadata?.stoppedFromStatus) return;
    setIsResuming(true);
    try {
      const note = restartNote.trim() || undefined;
      await api.tasks.move(taskId, stopMetadata.stoppedFromStatus, undefined, note);
      setRestartNote("");
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowValidationDialog(false);
    } finally {
      setIsResuming(false);
    }
  }, [taskId, queryClient, stopMetadata, restartNote]);

  // Handle go to ready from validation dialog
  const handleGoToReady = useCallback(async () => {
    setIsResuming(true);
    try {
      const note = restartNote.trim() || undefined;
      await api.tasks.move(taskId, "ready", undefined, note);
      setRestartNote("");
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      setShowValidationDialog(false);
    } finally {
      setIsResuming(false);
    }
  }, [taskId, queryClient, restartNote]);

  const handleAction = useCallback(async () => {
    // If task was stopped from a validated state, show validation dialog
    if (stopMetadata && validationWarnings.length > 0) {
      setShowValidationDialog(true);
      return;
    }

    const statusLabels: Record<string, string> = {
      ready: "Start",
      failed: "Restart",
      stopped: "Restart",
      cancelled: "Restart",
      paused: "Resume",
    };
    const actionLabel = statusLabels[status] || "Restart";
    const taskLabel = isReady
      ? "task"
      : status === "paused"
        ? "paused task"
        : "failed task";
    const modeNote = executionMode === "team" ? " in team mode" : "";

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
        description: isReady
          ? `The task will be started${modeNote}.`
          : `The task will be moved to ready status and can be executed again${modeNote}.`,
        confirmText: actionLabel,
        variant: "default",
      });
      if (!confirmed) return;
    }

    restartMutation.mutate();
  }, [confirm, status, isReady, isStopped, stopMetadata, restartMutation, executionMode, validationWarnings.length]);

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
          data-testid={isReady ? "start-button" : "restart-button"}
          onClick={handleAction}
          disabled={restartMutation.isPending || isResuming}
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px] transition-colors"
          style={{
            backgroundColor: isReady ? "hsl(14 100% 60%)" : "hsl(217 90% 60%)",
            color: "white",
          }}
        >
          {restartMutation.isPending || isResuming ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : isReady ? (
            <Play className="w-4 h-4" />
          ) : (
            <RotateCcw className="w-4 h-4" />
          )}
          {isReady ? "Start" : status === "paused" ? "Resume" : "Restart"}
        </Button>
      </div>

      {/* Execution Mode Selector */}
      <div className="mt-3 pt-3" style={{ borderTop: "1px solid hsla(220 10% 100% / 0.06)" }}>
        <ExecutionModeSelector
          mode={executionMode}
          onChange={setExecutionMode}
          disabled={restartMutation.isPending || isResuming}
        />
      </div>

      {/* Restart Note textarea (for restartable states only, not shown for start) */}
      {!isReady && (
        <div className="mt-3 pt-3" style={{ borderTop: "1px solid hsla(220 10% 100% / 0.06)" }}>
          <textarea
            data-testid="restart-note-textarea"
            value={restartNote}
            onChange={(e) => setRestartNote(e.target.value)}
            disabled={restartMutation.isPending || isResuming}
            placeholder="Optional: tell the agent what to do differently..."
            rows={3}
            className="w-full resize-none rounded-md px-3 py-2 text-[12px] transition-colors disabled:opacity-40 outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
            style={{
              backgroundColor: "hsla(220 10% 100% / 0.05)",
              color: "hsl(220 10% 80%)",
              boxShadow: "none",
              outline: "none",
            }}
          />
        </div>
      )}

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
        taskTitle={task.title}
        stoppedFromStatus={stopMetadata?.stoppedFromStatus}
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
  const showsActions = isRestartable || task.internalStatus === "ready";

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

  // Parse last_agent_error from metadata for any status
  const agentError = useMemo(() => {
    if (!task.metadata) return null;
    try {
      const metadata = JSON.parse(task.metadata);
      const lastError = metadata.last_agent_error;
      if (!lastError) return null;
      const errorContext: string | undefined = metadata.last_agent_error_context;
      const contextLabel =
        errorContext === "review" ? "Reviewer"
        : errorContext === "execution" ? "Worker"
        : "Agent";
      return {
        message: lastError as string,
        contextLabel,
        errorAt: metadata.last_agent_error_at as string | undefined,
      };
    } catch {
      return null;
    }
  }, [task.metadata]);

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

      {/* Agent Error Banner - shows last_agent_error for any status */}
      {agentError && (
        <section data-testid="agent-error-section" className="space-y-2">
          <SectionTitle>{agentError.contextLabel} Error</SectionTitle>
          <DetailCard variant="warning">
            <div className="flex items-start gap-2.5">
              <AlertTriangle
                className="w-4 h-4 mt-0.5 shrink-0"
                style={{ color: "hsl(35 100% 60%)" }}
              />
              <div className="flex-1 min-w-0">
                <p className="text-[13px]" style={{ color: "hsl(35 100% 75%)" }}>
                  {agentError.message}
                </p>
                {agentError.errorAt && (
                  <span
                    className="text-[11px] mt-1.5 block"
                    style={{ color: "hsl(220 10% 50%)" }}
                  >
                    {getTimeAgo(agentError.errorAt)}
                  </span>
                )}
              </div>
            </div>
          </DetailCard>
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
      {!isHistorical && showsActions && (
        <section>
          <ActionButtonsCard task={task} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
