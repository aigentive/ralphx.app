/**
 * ResearchProgress - Displays research process progress with controls
 *
 * Features:
 * - Process name and status badge
 * - Progress bar (currentIteration / maxIterations)
 * - Pause/Resume/Stop buttons
 * - Status-based styling
 */

import type { ResearchProcess, ResearchProcessStatus } from "@/types/research";
import { resolveDepth, calculateProgressPercentage } from "@/types/research";

// ============================================================================
// Types
// ============================================================================

interface ResearchProgressProps {
  process: ResearchProcess;
  onPause: (processId: string) => void;
  onResume: (processId: string) => void;
  onStop: (processId: string) => void;
  isActionPending?: boolean;
}

// ============================================================================
// Helpers
// ============================================================================

const STATUS_LABELS: Record<ResearchProcessStatus, string> = {
  pending: "Pending",
  running: "Running",
  paused: "Paused",
  completed: "Completed",
  failed: "Failed",
};

const STATUS_COLORS: Record<ResearchProcessStatus, string> = {
  pending: "var(--text-muted)",
  running: "var(--status-info)",
  paused: "var(--status-warning)",
  completed: "var(--status-success)",
  failed: "var(--status-error)",
};

// ============================================================================
// Component
// ============================================================================

export function ResearchProgress({ process, onPause, onResume, onStop, isActionPending = false }: ResearchProgressProps) {
  const { currentIteration, status } = process.progress;
  const { maxIterations } = resolveDepth(process.depth);
  const percentage = calculateProgressPercentage(currentIteration, maxIterations);

  const isActive = status === "running" || status === "pending";
  const isPaused = status === "paused";
  const isTerminal = status === "completed" || status === "failed";
  const showControls = !isTerminal;

  return (
    <div data-testid="research-progress" className="p-3 rounded border space-y-3" style={{ backgroundColor: "var(--bg-surface)", borderColor: "var(--border-subtle)" }}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <span data-testid="process-name" className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>{process.name}</span>
        <span data-testid="status-badge" className="text-xs px-1.5 py-0.5 rounded"
          style={{ color: STATUS_COLORS[status], backgroundColor: "var(--bg-base)" }}>{STATUS_LABELS[status]}</span>
      </div>

      {/* Progress Bar */}
      <div className="space-y-1">
        <div className="flex justify-between text-xs" style={{ color: "var(--text-muted)" }}>
          <span data-testid="iteration-count">{currentIteration} / {maxIterations}</span>
          <span>{Math.round(percentage)}%</span>
        </div>
        <div data-testid="progress-bar" role="progressbar" aria-valuenow={Math.round(percentage)} aria-valuemin={0} aria-valuemax={100}
          className="h-2 rounded-full overflow-hidden" style={{ backgroundColor: "var(--bg-base)" }}>
          <div data-testid="progress-fill" className="h-full transition-all" style={{ width: `${percentage}%`, backgroundColor: "var(--accent-primary)" }} />
        </div>
      </div>

      {/* Controls */}
      {showControls && (
        <div className="flex gap-2">
          {isActive && (
            <button data-testid="pause-button" onClick={() => onPause(process.id)} disabled={isActionPending}
              className="px-3 py-1 text-xs rounded disabled:opacity-50" style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}>Pause</button>
          )}
          {isPaused && (
            <button data-testid="resume-button" onClick={() => onResume(process.id)} disabled={isActionPending}
              className="px-3 py-1 text-xs rounded disabled:opacity-50" style={{ backgroundColor: "var(--accent-primary)", color: "var(--bg-base)" }}>Resume</button>
          )}
          {(isActive || isPaused) && (
            <button data-testid="stop-button" onClick={() => onStop(process.id)} disabled={isActionPending}
              className="px-3 py-1 text-xs rounded disabled:opacity-50" style={{ backgroundColor: "var(--bg-hover)", color: "var(--status-error)" }}>Stop</button>
          )}
        </div>
      )}
    </div>
  );
}
