/**
 * BasicTaskDetail - macOS Tahoe-inspired basic task view
 *
 * Clean, spacious layout for simple task states (backlog, ready, blocked).
 * Features native vibrancy materials and refined typography.
 */

import { useCallback } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { StepList } from "../StepList";
import { SectionTitle, TwoColumnLayout, DetailCard } from "./shared";
import { useTaskSteps } from "@/hooks/useTaskSteps";
import { useConfirmation } from "@/hooks/useConfirmation";
import { taskKeys } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import { Loader2, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Task } from "@/types/task";

// Task statuses that can be restarted
const RESTARTABLE_STATUSES = new Set(["failed", "stopped", "cancelled", "paused"]);

interface BasicTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

/**
 * ActionButtonsCard - Restart button for terminal/suspended states
 */
function ActionButtonsCard({
  taskId,
  status,
}: {
  taskId: string;
  status: string;
}) {
  const queryClient = useQueryClient();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

  const restartMutation = useMutation({
    mutationFn: async () => {
      await api.tasks.move(taskId, "ready");
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

    const confirmed = await confirm({
      title: `${actionLabel} this ${taskLabel}?`,
      description: `The task will be moved to ready status and can be executed again.`,
      confirmText: actionLabel,
      variant: "default",
    });
    if (!confirmed) return;
    restartMutation.mutate();
  }, [confirm, status, restartMutation]);

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

  return (
    <TwoColumnLayout
      description={task.description}
      testId="basic-task-detail"
    >
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
