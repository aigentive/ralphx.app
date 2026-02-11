/**
 * ExecutionControlBar - Premium execution status and controls
 *
 * Fixed bottom bar displaying running/queued/merging tasks count with animated status indicator
 * and pause/stop controls. Follows the design spec from specs/design/pages/execution-control-bar.md
 *
 * Responsive breakpoints:
 * - Wide (>1200px): Full labels "Running: 2/3", "Queued: 5", "Merging: 1"
 * - Medium (800-1200px): Abbreviated "R: 2/3", "Q: 5", "M: 1"
 * - Narrow (<800px): Counts only "2/3", "5", "1"
 */

import { Pause, Play, Square, Loader2, Swords } from "lucide-react";
import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { RunningProcessPopover } from "./RunningProcessPopover";
import type { RunningProcess } from "@/api/running-processes";
import { MergePipelinePopover } from "./MergePipelinePopover";
import type { MergePipelineResponse } from "@/api/merge-pipeline";
import { QueuedTasksPopover } from "./QueuedTasksPopover";
import { InfoTooltip } from "./InfoTooltip";

interface ExecutionControlBarProps {
  /** The project ID */
  projectId: string;
  /** Number of currently running tasks */
  runningCount: number;
  /** Maximum concurrent tasks allowed */
  maxConcurrent: number;
  /** Number of queued (planned) tasks */
  queuedCount: number;
  /** Number of tasks in the merge pipeline */
  mergingCount: number;
  /** Whether any merge tasks need attention (conflict/incomplete) */
  hasAttentionMerges: boolean;
  /** Merge pipeline data for popover */
  mergePipelineData: MergePipelineResponse | null;
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
  /** List of running processes (for popover) */
  runningProcesses?: RunningProcess[];
  /** Called when pause button clicked for a specific process */
  onPauseProcess?: (taskId: string) => void;
  /** Called when stop button clicked for a specific process */
  onStopProcess?: (taskId: string) => void;
  /** Called when settings link clicked in popover */
  onOpenSettings?: () => void;
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
  projectId,
  runningCount,
  maxConcurrent,
  queuedCount,
  mergingCount,
  hasAttentionMerges,
  mergePipelineData,
  isPaused,
  isLoading = false,
  currentTaskName,
  onPauseToggle,
  onStop,
  battleModeActive = false,
  onBattleModeToggle,
  showBattleModeToggle = false,
  runningProcesses = [],
  onPauseProcess = () => {},
  onStopProcess = () => {},
  onOpenSettings = () => {},
}: ExecutionControlBarProps) {
  const canStop = runningCount > 0 && !isLoading;
  const statusColor = getStatusColor(runningCount, isPaused);
  const statusState = getStatusState(runningCount, isPaused);
  const isRunning = runningCount > 0 && !isPaused;
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);

  // Responsive breakpoint tracking
  const [breakpoint, setBreakpoint] = useState<"wide" | "medium" | "narrow">("wide");

  useEffect(() => {
    const updateBreakpoint = () => {
      const width = window.innerWidth;
      if (width > 1200) {
        setBreakpoint("wide");
      } else if (width >= 800) {
        setBreakpoint("medium");
      } else {
        setBreakpoint("narrow");
      }
    };

    updateBreakpoint();
    window.addEventListener("resize", updateBreakpoint);
    return () => window.removeEventListener("resize", updateBreakpoint);
  }, []);

  // Label formatting based on breakpoint
  const runningLabel = breakpoint === "wide" ? "Running: " : breakpoint === "medium" ? "R: " : "";
  const queuedLabel = breakpoint === "wide" ? "Queued: " : breakpoint === "medium" ? "Q: " : "";
  const mergingLabel = breakpoint === "wide" ? "Merging: " : breakpoint === "medium" ? "M: " : "";

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

          {/* Running Count (Clickable - opens popover) */}
          <div className="flex items-center gap-1.5">
            <RunningProcessPopover
              processes={runningProcesses}
              maxConcurrent={maxConcurrent}
              open={isPopoverOpen}
              onOpenChange={setIsPopoverOpen}
              onPauseProcess={onPauseProcess}
              onStopProcess={onStopProcess}
              onOpenSettings={onOpenSettings}
            >
              <button
                data-testid="running-count"
                className="text-[13px] font-medium cursor-pointer hover:opacity-80 transition-opacity"
                style={{ color: "hsl(220 10% 90%)" }}
                onClick={() => setIsPopoverOpen(!isPopoverOpen)}
              >
                {runningLabel}{runningCount}/{maxConcurrent}
              </button>
            </RunningProcessPopover>
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

          {/* Queued Count (Clickable Popover) */}
          <div className="flex items-center gap-1.5">
            <QueuedTasksPopover
              projectId={projectId}
              queuedCount={queuedCount}
            >
              <button
                data-testid="queued-count"
                className="text-[13px] cursor-pointer hover:underline transition-all"
                style={{ color: "hsl(220 10% 65%)" }}
                aria-label="View queued tasks"
                aria-haspopup="dialog"
              >
                {queuedLabel}{queuedCount}
              </button>
            </QueuedTasksPopover>
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

          {/* Merging Count with Popover */}
          <div className="flex items-center gap-1.5">
            {mergePipelineData ? (
              <MergePipelinePopover
                active={mergePipelineData.active}
                waiting={mergePipelineData.waiting}
                needsAttention={mergePipelineData.needsAttention}
              >
                <button
                  data-testid="merging-count"
                  className="flex items-center gap-1.5 text-[13px] cursor-pointer hover:opacity-80 transition-opacity"
                  style={{ color: "hsl(220 10% 65%)" }}
                >
                  {mergingLabel}{mergingCount}
                  {hasAttentionMerges && (
                    <span
                      data-testid="merge-attention-indicator"
                      className="text-sm"
                      style={{ color: "hsl(45 90% 55%)" }}
                      title="Some merges need attention"
                    >
                      ⚠
                    </span>
                  )}
                </button>
              </MergePipelinePopover>
            ) : (
              <span
                data-testid="merging-count"
                className="flex items-center gap-1.5 text-[13px]"
                style={{ color: "hsl(220 10% 65%)" }}
              >
                {mergingLabel}{mergingCount}
                {hasAttentionMerges && (
                  <span
                    data-testid="merge-attention-indicator"
                    className="text-sm"
                    style={{ color: "hsl(45 90% 55%)" }}
                    title="Some merges need attention"
                  >
                    ⚠
                  </span>
                )}
              </span>
            )}
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
