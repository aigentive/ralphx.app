/**
 * ExecutionControlBar - Displays execution status and controls
 * Shows running/queued tasks count with pause and stop buttons
 */

interface ExecutionControlBarProps {
  /** Number of currently running tasks */
  runningCount: number;
  /** Maximum concurrent tasks allowed */
  maxConcurrent: number;
  /** Number of queued (planned) tasks */
  queuedCount: number;
  /** Whether execution is paused */
  isPaused: boolean;
  /** Whether a control action is in progress */
  isLoading?: boolean;
  /** Called when pause/resume button clicked */
  onPauseToggle: () => void;
  /** Called when stop button clicked */
  onStop: () => void;
}

const btnBase = "px-3 py-1.5 rounded text-sm font-medium transition-colors flex items-center gap-1";

function getStatusColor(running: number, paused: boolean): string {
  if (paused) return "var(--status-warning)";
  if (running > 0) return "var(--status-success)";
  return "var(--text-muted)";
}

export function ExecutionControlBar({
  runningCount,
  maxConcurrent,
  queuedCount,
  isPaused,
  isLoading = false,
  onPauseToggle,
  onStop,
}: ExecutionControlBarProps) {
  const canStop = runningCount > 0 && !isLoading;
  const statusColor = getStatusColor(runningCount, isPaused);

  return (
    <div
      data-testid="execution-control-bar"
      data-paused={isPaused ? "true" : "false"}
      data-running={runningCount}
      data-loading={isLoading ? "true" : undefined}
      className="flex items-center justify-between px-4 py-2 border rounded-lg"
      style={{
        backgroundColor: "var(--bg-elevated)",
        borderColor: "var(--border-subtle)",
      }}
    >
      <div className="flex items-center gap-4">
        <div
          data-testid="status-indicator"
          className="w-2 h-2 rounded-full"
          style={{ backgroundColor: statusColor }}
        />
        <span
          data-testid="running-count"
          className="text-sm font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          Running: {runningCount}/{maxConcurrent}
        </span>
        <span
          data-testid="queued-count"
          className="text-sm"
          style={{ color: "var(--text-secondary)" }}
        >
          Queued: {queuedCount}
        </span>
      </div>

      <div className="flex items-center gap-2">
        <button
          data-testid="pause-toggle-button"
          onClick={onPauseToggle}
          disabled={isLoading}
          className={btnBase}
          style={{
            backgroundColor: "var(--bg-hover)",
            color: "var(--text-primary)",
            cursor: isLoading ? "not-allowed" : "pointer",
            opacity: isLoading ? 0.5 : 1,
          }}
        >
          {isPaused ? "▶ Resume" : "⏸ Pause"}
        </button>
        <button
          data-testid="stop-button"
          onClick={onStop}
          disabled={!canStop}
          className={btnBase}
          style={{
            backgroundColor: canStop ? "var(--status-error)" : "var(--bg-hover)",
            color: canStop ? "var(--bg-base)" : "var(--text-secondary)",
            cursor: canStop ? "pointer" : "not-allowed",
            opacity: canStop ? 1 : 0.5,
          }}
        >
          ⏹ Stop
        </button>
      </div>
    </div>
  );
}
