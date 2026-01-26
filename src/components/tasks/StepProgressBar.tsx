/**
 * StepProgressBar component
 *
 * Displays visual progress dots for task steps with optional text summary.
 * Compact mode shows just dots and count, full mode includes text.
 */

import { useStepProgress } from "@/hooks/useTaskSteps";

interface StepProgressBarProps {
  taskId: string;
  compact?: boolean;
}

/**
 * Get background color class for step dot based on status
 */
function getStepDotColor(
  index: number,
  completed: number,
  skipped: number,
  failed: number,
  inProgress: number
): string {
  const completedAndSkipped = completed + skipped;
  const failedStart = completedAndSkipped;
  const failedEnd = failedStart + failed;
  const inProgressStart = failedEnd;
  const inProgressEnd = inProgressStart + inProgress;

  if (index < completed) {
    // Completed steps
    return "bg-status-success";
  } else if (index < completedAndSkipped) {
    // Skipped steps
    return "bg-text-muted";
  } else if (index < failedEnd) {
    // Failed steps
    return "bg-status-error";
  } else if (index < inProgressEnd) {
    // In progress steps
    return "bg-accent-primary animate-pulse";
  } else {
    // Pending steps
    return "bg-border-default";
  }
}

/**
 * StepProgressBar Component
 *
 * Renders a visual progress indicator using dots, where each dot represents
 * a step with color-coded status.
 *
 * @example
 * ```tsx
 * // Compact mode for TaskCard
 * <StepProgressBar taskId="task-123" compact />
 *
 * // Full mode with text
 * <StepProgressBar taskId="task-123" />
 * ```
 */
export function StepProgressBar({ taskId, compact = false }: StepProgressBarProps) {
  const { data: progress, isLoading } = useStepProgress(taskId);

  // Don't render anything while loading, if no data, or if there are no steps
  if (isLoading || !progress || progress.total === 0) {
    return null;
  }

  const { total, completed, skipped, failed, inProgress } = progress;
  const completedAndSkipped = completed + skipped;

  return (
    <div className="flex items-center gap-2">
      {/* Progress dots */}
      <div className="flex items-center gap-1">
        {Array.from({ length: total }).map((_, index) => (
          <div
            key={index}
            className={`h-1.5 w-1.5 rounded-full transition-colors ${getStepDotColor(
              index,
              completed,
              skipped,
              failed,
              inProgress
            )}`}
            aria-hidden="true"
          />
        ))}
      </div>

      {/* Text summary */}
      {!compact && (
        <span className="text-xs text-text-muted">
          {completedAndSkipped}/{total}
        </span>
      )}
    </div>
  );
}
