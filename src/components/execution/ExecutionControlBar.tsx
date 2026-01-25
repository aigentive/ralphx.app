/**
 * ExecutionControlBar - Premium execution status and controls
 *
 * Fixed bottom bar displaying running/queued tasks count with animated status indicator
 * and pause/stop controls. Follows the design spec from specs/design/pages/execution-control-bar.md
 */

import { Pause, Play, Square, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

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
  /** Name of the currently executing task (optional) */
  currentTaskName?: string;
  /** Called when pause/resume button clicked */
  onPauseToggle: () => void;
  /** Called when stop button clicked */
  onStop: () => void;
}

/**
 * Get status indicator color based on execution state
 */
function getStatusColor(running: number, paused: boolean): string {
  if (paused) return "var(--status-warning)";
  if (running > 0) return "var(--status-success)";
  return "var(--text-muted)";
}

/**
 * Get status state for data attributes and animation class
 */
function getStatusState(running: number, paused: boolean): "running" | "paused" | "idle" {
  if (paused) return "paused";
  if (running > 0) return "running";
  return "idle";
}

export function ExecutionControlBar({
  runningCount,
  maxConcurrent,
  queuedCount,
  isPaused,
  isLoading = false,
  currentTaskName,
  onPauseToggle,
  onStop,
}: ExecutionControlBarProps) {
  const canStop = runningCount > 0 && !isLoading;
  const statusColor = getStatusColor(runningCount, isPaused);
  const statusState = getStatusState(runningCount, isPaused);
  const isRunning = runningCount > 0 && !isPaused;

  return (
    <TooltipProvider>
      <div
        data-testid="execution-control-bar"
        data-paused={isPaused ? "true" : "false"}
        data-running={runningCount}
        data-loading={isLoading ? "true" : undefined}
        data-status={statusState}
        role="region"
        aria-label="Execution controls"
        aria-live="polite"
        className="flex h-12 items-center justify-between px-4 border-t z-10"
        style={{
          backgroundColor: "var(--bg-surface)",
          borderColor: "var(--border-subtle)",
          boxShadow: "0 -2px 8px rgba(0,0,0,0.15)",
        }}
      >
        {/* Status Section (Left) */}
        <div
          className="flex items-center gap-4"
          aria-label={`${runningCount} tasks running out of ${maxConcurrent}, ${queuedCount} queued`}
        >
          {/* Animated Status Indicator */}
          <div
            data-testid="status-indicator"
            className={cn(
              "w-2 h-2 rounded-full transition-colors duration-200",
              isRunning && "status-indicator-running"
            )}
            style={{ backgroundColor: statusColor }}
          />

          {/* Running Count */}
          <span
            data-testid="running-count"
            className="text-sm font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            Running: {runningCount}/{maxConcurrent}
          </span>

          {/* Separator */}
          <span style={{ color: "var(--text-muted)" }}>•</span>

          {/* Queued Count */}
          <span
            data-testid="queued-count"
            className="text-sm"
            style={{ color: "var(--text-secondary)" }}
          >
            Queued: {queuedCount}
          </span>
        </div>

        {/* Progress Section (Center) - Conditional */}
        {isRunning && currentTaskName && (
          <div
            data-testid="current-task"
            className="flex items-center gap-2 max-w-[40%] task-name-enter"
          >
            <Loader2
              className="w-4 h-4 animate-spin shrink-0"
              style={{ color: "var(--accent-primary)" }}
            />
            <span
              className="text-sm truncate"
              style={{ color: "var(--text-secondary)" }}
            >
              {currentTaskName}
            </span>
          </div>
        )}

        {/* Control Section (Right) */}
        <div className="flex items-center gap-2">
          {/* Pause/Resume Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                data-testid="pause-toggle-button"
                variant="ghost"
                size="default"
                onClick={onPauseToggle}
                disabled={isLoading}
                aria-label={isPaused ? "Resume execution" : "Pause execution"}
                aria-pressed={isPaused}
                className={cn(
                  "gap-2 border h-9 px-4 transition-all duration-150 active:scale-[0.96]",
                  isPaused
                    ? "bg-[var(--accent-muted)] border-[var(--accent-primary)]/30 text-[var(--accent-primary)] hover:bg-[var(--accent-muted)] hover:border-[var(--accent-primary)]/50"
                    : "border-[var(--border-default)] text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
                )}
              >
                {isLoading ? (
                  <Loader2 className="w-[18px] h-[18px] animate-spin" />
                ) : isPaused ? (
                  <Play className="w-[18px] h-[18px]" />
                ) : (
                  <Pause className="w-[18px] h-[18px]" />
                )}
                <span className="hidden sm:inline">
                  {isPaused ? "Resume" : "Pause"}
                </span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>
                {isPaused
                  ? "Resume execution from queue ⌘P"
                  : "Pause execution (tasks in progress will complete) ⌘P"}
              </p>
            </TooltipContent>
          </Tooltip>

          {/* Stop Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                data-testid="stop-button"
                variant="ghost"
                size="default"
                onClick={onStop}
                disabled={!canStop}
                aria-label="Stop all running tasks"
                aria-disabled={!canStop}
                className={cn(
                  "gap-2 border h-9 px-4 transition-all duration-150 active:scale-[0.96]",
                  canStop
                    ? "bg-[rgba(239,68,68,0.15)] border-[var(--status-error)]/30 text-[var(--status-error)] hover:bg-[rgba(239,68,68,0.25)] hover:border-[var(--status-error)]/50"
                    : "bg-[var(--bg-hover)] border-[var(--border-subtle)] text-[var(--text-muted)] opacity-50"
                )}
              >
                <Square className="w-4 h-4 fill-current" />
                <span className="hidden sm:inline">Stop</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>
                {canStop
                  ? "Stop all running tasks immediately ⌘⇧S"
                  : "No tasks currently running"}
              </p>
            </TooltipContent>
          </Tooltip>
        </div>
      </div>
    </TooltipProvider>
  );
}
