/**
 * ExecutionControlBar - Premium execution status and controls
 *
 * Fixed bottom bar displaying running/queued tasks count with animated status indicator
 * and pause/stop controls. Follows the design spec from specs/design/pages/execution-control-bar.md
 */

import { Pause, Play, Square, Loader2, Swords } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { InfoTooltip } from "./InfoTooltip";

interface ExecutionControlBarProps {
  /** Number of currently running tasks */
  runningCount: number;
  /** Maximum concurrent tasks allowed */
  maxConcurrent: number;
  /** Number of queued (planned) tasks */
  queuedCount: number;
  /** Number of tasks in the merge pipeline */
  mergingCount?: number;
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
  /** Whether battle mode is active */
  battleModeActive?: boolean;
  /** Called when battle mode button clicked */
  onBattleModeToggle?: () => void;
  /** Whether to show battle mode button */
  showBattleModeToggle?: boolean;
}

/**
 * Get status indicator color based on execution state
 */
function getStatusColor(running: number, paused: boolean): string {
  if (paused) return "hsl(45 90% 55%)"; /* warning */
  if (running > 0) return "hsl(145 60% 45%)"; /* success */
  return "hsl(220 10% 45%)"; /* muted */
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
  mergingCount = 0,
  isPaused,
  isLoading = false,
  currentTaskName,
  onPauseToggle,
  onStop,
  battleModeActive = false,
  onBattleModeToggle,
  showBattleModeToggle = false,
}: ExecutionControlBarProps) {
  const canStop = runningCount > 0 && !isLoading;
  const statusColor = getStatusColor(runningCount, isPaused);
  const statusState = getStatusState(runningCount, isPaused);
  const isRunning = runningCount > 0 && !isPaused;

  return (
    <TooltipProvider>
      {/* Outer container with padding for floating effect */}
      <div className="p-2" style={{ backgroundColor: "hsl(220 10% 8%)" }}>
        {/* Inner floating glass container */}
        <div
          data-testid="execution-control-bar"
          data-paused={isPaused ? "true" : "false"}
          data-running={runningCount}
          data-loading={isLoading ? "true" : undefined}
          data-status={statusState}
          role="region"
          aria-label="Execution controls"
          aria-live="polite"
          className="flex py-3 items-center justify-between px-4 z-10"
          style={{
            /* macOS Tahoe: floating panel - FLAT with blur */
            borderRadius: "10px",
            background: "hsla(220 10% 10% / 0.92)",
            backdropFilter: "blur(20px) saturate(180%)",
            WebkitBackdropFilter: "blur(20px) saturate(180%)",
            border: "1px solid hsla(220 20% 100% / 0.08)",
            boxShadow: `
              0 4px 16px hsla(220 20% 0% / 0.4),
              0 12px 32px hsla(220 20% 0% / 0.3)
            `,
          }}
        >
        {/* Status Section (Left) */}
        <div
          className="flex items-center gap-4"
          aria-label={`${runningCount} tasks running out of ${maxConcurrent}, ${queuedCount} queued, ${mergingCount} merging`}
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
          <div className="flex items-center gap-1.5">
            <span
              data-testid="running-count"
              className="text-[13px] font-medium"
              style={{ color: "hsl(220 10% 90%)" }}
            >
              Running: {runningCount}/{maxConcurrent}
            </span>
            <InfoTooltip
              testId="running-info-tooltip"
              content={
                <div className="space-y-2">
                  <div>
                    <strong className="block mb-1" style={{ color: "hsl(220 10% 95%)" }}>
                      Concurrent Execution
                    </strong>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Tasks running in parallel. Currently limited to{" "}
                      <strong>{maxConcurrent}</strong> per project, 20 globally.
                    </p>
                  </div>
                  <div>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Includes: executing, reviewing, re-executing, QA, and merging agents.
                    </p>
                  </div>
                  <div className="pt-1 border-t" style={{ borderColor: "hsla(220 20% 100% / 0.08)" }}>
                    <p className="text-xs" style={{ color: "hsl(220 10% 60%)" }}>
                      Change limits → Settings
                    </p>
                  </div>
                </div>
              }
            />
          </div>

          {/* Separator */}
          <span style={{ color: "hsl(220 10% 45%)" }}>•</span>

          {/* Queued Count */}
          <div className="flex items-center gap-1.5">
            <span
              data-testid="queued-count"
              className="text-[13px]"
              style={{ color: "hsl(220 10% 65%)" }}
            >
              Queued: {queuedCount}
            </span>
            <InfoTooltip
              testId="queued-info-tooltip"
              content={
                <div className="space-y-2">
                  <div>
                    <strong className="block mb-1" style={{ color: "hsl(220 10% 95%)" }}>
                      Task Queue
                    </strong>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Tasks in "ready" status waiting for an open execution slot.
                      Processed by priority then age (oldest first).
                    </p>
                  </div>
                  <div>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Blocked tasks are NOT counted here.
                    </p>
                  </div>
                </div>
              }
            />
          </div>

          {/* Separator */}
          <span style={{ color: "hsl(220 10% 45%)" }}>•</span>

          {/* Merging Count */}
          <div className="flex items-center gap-1.5">
            <span
              data-testid="merging-count"
              className="text-[13px]"
              style={{ color: "hsl(220 10% 65%)" }}
            >
              Merging: {mergingCount}
            </span>
            <InfoTooltip
              testId="merging-info-tooltip"
              content={
                <div className="space-y-2">
                  <div>
                    <strong className="block mb-1" style={{ color: "hsl(220 10% 95%)" }}>
                      Merge Pipeline
                    </strong>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Two-phase: fast programmatic merge first, then AI agent for conflicts.
                      Merges run one at a time per target branch to avoid concurrent git conflicts.
                    </p>
                  </div>
                  <div>
                    <p style={{ color: "hsl(220 10% 75%)" }}>
                      Merges to different branches run in parallel.
                      Deferred merges auto-retry when the current merge completes.
                    </p>
                  </div>
                </div>
              }
            />
          </div>
        </div>

        {/* Progress Section (Center) - Conditional */}
        {isRunning && currentTaskName && (
          <div
            data-testid="current-task"
            className="flex items-center gap-2 max-w-[40%] task-name-enter"
          >
            <Loader2
              className="w-4 h-4 animate-spin shrink-0"
              style={{ color: "hsl(14 100% 60%)" }}
            />
            <span
              className="text-[13px] truncate"
              style={{ color: "hsl(220 10% 65%)" }}
            >
              {currentTaskName}
            </span>
          </div>
        )}

        {/* Control Section (Right) */}
        <div className="flex items-center gap-2">
          {/* Battle Mode Button (Graph view only) */}
          {showBattleModeToggle && onBattleModeToggle && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  data-testid="battle-mode-toggle-button"
                  variant="ghost"
                  size="default"
                  onClick={onBattleModeToggle}
                  aria-label={battleModeActive ? "Exit Battle Mode" : "Enter Battle Mode"}
                  aria-pressed={battleModeActive}
                  className="gap-2 h-9 px-4 transition-all duration-150 active:scale-[0.96] rounded-lg text-[13px]"
                  style={{
                    backgroundColor: battleModeActive
                      ? "hsla(14 100% 60% / 0.2)"
                      : "hsl(220 10% 18%)",
                    color: battleModeActive ? "hsl(14 100% 60%)" : "hsl(220 10% 90%)",
                    border: "none",
                  }}
                >
                  <Swords className="w-4 h-4" />
                  <span className="hidden sm:inline">
                    Battle Mode
                  </span>
                </Button>
              </TooltipTrigger>
              <TooltipContent side="top">
                <p>{battleModeActive ? "Return to graph mode" : "Launch battle mode overlay"}</p>
              </TooltipContent>
            </Tooltip>
          )}

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
                className="gap-2 h-9 px-4 transition-all duration-150 active:scale-[0.96] rounded-lg text-[13px]"
                style={{
                  /* macOS Tahoe: flat button styling */
                  backgroundColor: isPaused ? "hsla(14 100% 60% / 0.15)" : "hsl(220 10% 18%)",
                  color: isPaused ? "hsl(14 100% 60%)" : "hsl(220 10% 90%)",
                  border: "none",
                }}
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
                  ? "Resume paused tasks and queue ⌘P"
                  : "Pause execution (running tasks will pause) ⌘P"}
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
                className="gap-2 h-9 px-4 transition-all duration-150 active:scale-[0.96] rounded-lg text-[13px]"
                style={{
                  /* macOS Tahoe: flat button styling */
                  backgroundColor: canStop ? "hsla(0 70% 55% / 0.15)" : "hsl(220 10% 18%)",
                  color: canStop ? "hsl(0 70% 60%)" : "hsl(220 10% 45%)",
                  border: "none",
                  opacity: canStop ? 1 : 0.5,
                }}
              >
                <Square className="w-4 h-4 fill-current" />
                <span className="hidden sm:inline">Stop</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>
                {canStop
                  ? "Stop all running tasks (manual restart required) ⌘⇧S"
                  : "No tasks currently running"}
              </p>
            </TooltipContent>
          </Tooltip>
        </div>
        </div>
      </div>
    </TooltipProvider>
  );
}
