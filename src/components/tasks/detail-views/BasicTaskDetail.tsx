/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { useCallback, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { StepList } from "../StepList";
import { SectionTitle, TwoColumnLayout, DetailCard } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useConfirmation } from "@/hooks/useConfirmation";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { Loader2, RotateCcw, User, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Task } from "@/types/task";

type ExecutionMode = "solo" | "team";

// Task statuses that can be restarted
const RESTARTABLE_STATUSES = new Set(["failed", "stopped", "cancelled", "paused"]);

interface BasicTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * ActionButtonsCard - Restart button for terminal/suspended states
 */
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

function ActionButtonsCard({
  taskId,
  status,
}: {
  taskId: string;
  status: string;
}) {
  const queryClient = useQueryClient();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [executionMode, setExecutionMode] = useState<ExecutionMode>("solo");

  const restartMutation = useMutation({
    mutationFn: async () => {
      await api.tasks.move(
        taskId,
        "ready",
        executionMode === "team" ? "team" : undefined
      );
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
    const modeNote = executionMode === "team" ? " in team mode" : "";

    const confirmed = await confirm({
      title: `${actionLabel} this ${taskLabel}?`,
      description: `The task will be moved to ready status and can be executed again${modeNote}.`,
      confirmText: actionLabel,
      variant: "default",
    });
    if (!confirmed) return;
    restartMutation.mutate();
  }, [confirm, status, restartMutation, executionMode]);

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

      {/* Execution Mode Selector */}
      <div className="mt-3 pt-3" style={{ borderTop: "1px solid hsla(220 10% 100% / 0.06)" }}>
        <ExecutionModeSelector
          mode={executionMode}
          onChange={setExecutionMode}
          disabled={restartMutation.isPending}
        />
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
          <ActionButtonsCard taskId={task.id} status={task.internalStatus} />
        </section>
      )}
    </TwoColumnLayout>
  );
}
