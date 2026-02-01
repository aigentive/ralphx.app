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
 * Renders a visual progress indicator:
 * - Compact mode: thin progress bar with percentage (for TaskCard)
 * - Full mode: dots with count (for detail views)
 *
 * @example
 * ```tsx
 * // Compact mode for TaskCard - shows bar + percentage
 * <StepProgressBar taskId="task-123" compact />
 *
 * // Full mode with dots
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
  const percentComplete = Math.round((completedAndSkipped / total) * 100);

  // Compact mode: progress bar + percentage + dots for TaskCard
  if (compact) {
    return (
      <div className="flex-1 space-y-1.5">
        {/* Progress bar row with percentage */}
        <div className="flex items-center gap-2">
          <div
            className="flex-1 h-1 rounded-full overflow-hidden"
            style={{ backgroundColor: "hsl(220 10% 14%)" }}
          >
            <div
              className="h-full rounded-full transition-all duration-300"
              style={{
                width: `${percentComplete}%`,
                backgroundColor: failed > 0
                  ? "hsl(0 70% 55%)"
                  : inProgress > 0
                  ? "hsl(14 100% 60%)"
                  : percentComplete === 100
                  ? "hsl(145 60% 45%)"
                  : "hsl(220 10% 50%)",
              }}
            />
          </div>
          <span
            className="text-[10px] tabular-nums shrink-0"
            style={{
              color: failed > 0
                ? "hsl(0 70% 55%)"
                : percentComplete === 100
                ? "hsl(145 60% 45%)"
                : "hsl(220 10% 40%)",
            }}
          >
            {percentComplete}%
          </span>
        </div>
        {/* Dots row */}
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
      </div>
    );
  }

  // Full mode: dots with count
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
      <span className="text-xs text-text-muted">
        {completedAndSkipped}/{total}
      </span>
    </div>
  );
}
