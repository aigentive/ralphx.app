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

import { AlertTriangle, Pause, Play, Square, Loader2 } from "lucide-react";
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
import type { RunningProcess, RunningIdeationSession } from "@/api/running-processes";
import { MergePipelinePopover } from "./MergePipelinePopover";
import type { MergePipelineResponse } from "@/api/merge-pipeline";
import { QueuedTasksPopover } from "./QueuedTasksPopover";
import { PausedTasksPopover } from "./PausedTasksPopover";
import { InfoTooltip } from "./InfoTooltip";
import { getStatusIconConfig } from "@/types/status-icons";
import type { Task } from "@/types/task";
import type { ExecutionHaltMode } from "@/api/execution";

interface ExecutionControlBarProps {
  /** The project ID */
  projectId: string;
  /** Number of currently running tasks */
  runningCount: number;
  /** Maximum concurrent tasks allowed */
  maxConcurrent: number;
  /** Number of queued (planned) tasks */
  queuedCount: number;
  /** Number of queued agent messages held by pause/capacity barriers */
  queuedMessageCount?: number;
  /** Number of tasks paused due to provider errors */
  pausedCount?: number;
  /** Tasks paused due to provider errors (for popover) */
  pausedTasks?: Task[];
  /** Number of currently generating ideation sessions (consuming slots) */
  ideationActive?: number;
  /** Per-project maximum concurrent ideation sessions */
  ideationMax?: number;
  /** Number of ideation sessions waiting for capacity (pending_initial_prompt) */
  ideationWaiting?: number;
  /** Number of tasks in the merge pipeline */
  mergingCount: number;
  /** Whether any merge tasks need attention (conflict/incomplete) */
  hasAttentionMerges: boolean;
  /** Merge pipeline data for popover */
  mergePipelineData: MergePipelineResponse | null;
  /** Whether execution is paused */
  isPaused: boolean;
  /** Current halt mode for global execution controls */
  haltMode?: ExecutionHaltMode;
  /** Whether a control action is in progress */
  isLoading?: boolean;
  /** Name of the currently executing task (optional) */
  currentTaskName?: string;
  /** Called when pause/resume button clicked */
  onPauseToggle: () => void;
  /** Called when stop button clicked */
  onStop: () => void;
  /** List of running processes (for popover) */
  runningProcesses?: RunningProcess[];
  /** List of running ideation sessions (for popover) */
  ideationSessions?: RunningIdeationSession[];
  /** Called when pause button clicked for a specific process */
  onPauseProcess?: (taskId: string) => void;
  /** Called when stop button clicked for a specific process */
  onStopProcess?: (taskId: string) => void;
  /** Called when settings link clicked in popover */
  onOpenSettings?: () => void;
  /** Called when an ideation session is clicked in the running processes popover */
  onNavigateToSession?: (sessionId: string) => void;
}

/**
 * Get status indicator color based on execution state
 */
const STATUS_COLORS = {
  running: getStatusIconConfig("executing").color,
  paused: getStatusIconConfig("paused").color,
  idle: getStatusIconConfig("backlog").color,
  ready: getStatusIconConfig("ready").color,
  pendingMerge: getStatusIconConfig("pending_merge").color,
  mergeAttention: getStatusIconConfig("merge_incomplete").color,
  stop: getStatusIconConfig("stopped").color,
} as const;
const POPOVER_ALIGN_TO_SEPARATOR_DOT = -20;

function getStatusColor(
  running: number,
  paused: boolean,
  haltMode: ExecutionHaltMode
): string {
  if (haltMode === "stopped") return STATUS_COLORS.stop;
  if (paused) return STATUS_COLORS.paused;
  if (running > 0) return STATUS_COLORS.running;
  return STATUS_COLORS.idle;
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
  queuedMessageCount = 0,
  pausedCount = 0,
  pausedTasks = [],
  ideationActive = 0,
  ideationMax = 0,
  ideationWaiting = 0,
  mergingCount,
  hasAttentionMerges,
  mergePipelineData,
  isPaused,
  haltMode = isPaused ? "paused" : "running",
  isLoading = false,
  currentTaskName,
  onPauseToggle,
  onStop,
  runningProcesses = [],
  ideationSessions = [],
  onPauseProcess = () => {},
  onStopProcess = () => {},
  onOpenSettings = () => {},
  onNavigateToSession,
}: ExecutionControlBarProps) {
  const canStop = runningCount > 0 && !isLoading;
  const isStopped = haltMode === "stopped";
  const canPauseToggle = !isLoading;
  const statusColor = getStatusColor(runningCount, isPaused, haltMode);
  const statusState = isStopped ? "stopped" : getStatusState(runningCount, isPaused);
  const isRunning = runningCount > 0 && !isPaused;
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"execution" | "ideation">("execution");

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
  const queuedMessageLabel =
    breakpoint === "wide" ? "Msgs: " : breakpoint === "medium" ? "Msg: " : "";
  const pausedLabel = breakpoint === "wide" ? "Paused: " : breakpoint === "medium" ? "P: " : "";
  const mergingLabel = breakpoint === "wide" ? "Merging: " : breakpoint === "medium" ? "M: " : "";
  const ideationLabel = breakpoint === "wide" ? "Ideation: " : breakpoint === "medium" ? "I: " : "";

  // Only show ideation indicator when max > 0
  const showIdeation = ideationMax > 0;

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
          aria-label={`${runningCount} tasks running out of ${maxConcurrent}, ${queuedCount} queued tasks, ${queuedMessageCount} queued messages, ${pausedCount} paused, ${mergingCount} merging`}
        >
          {/* Animated Status Indicator (anchor for all popovers) */}
          <div
            data-testid="status-indicator"
            className={cn(
              "w-2 h-2 rounded-full transition-colors duration-200",
              isRunning && "status-indicator-running"
            )}
            style={{ backgroundColor: statusColor }}
          />

          {/* Running Count (Clickable - opens popover) + Info Tooltip */}
          <div className="flex items-center gap-1.5">
            <RunningProcessPopover
              processes={runningProcesses}
              ideationSessions={ideationSessions}
              runningCount={runningCount}
              maxConcurrent={maxConcurrent}
              ideationMax={ideationMax}
              open={isPopoverOpen}
              onOpenChange={setIsPopoverOpen}
              onPauseProcess={onPauseProcess}
              onStopProcess={onStopProcess}
              onOpenSettings={onOpenSettings}
              {...(onNavigateToSession !== undefined && { onNavigateToSession })}
              alignOffset={POPOVER_ALIGN_TO_SEPARATOR_DOT}
              initialTab={activeTab}
              showIdeation={showIdeation}
            >
              <button
                data-testid="running-count"
                className="text-[13px] font-medium cursor-pointer hover:opacity-80 transition-opacity"
                style={{ color: runningCount > 0 ? STATUS_COLORS.running : "hsl(220 10% 90%)" }}
                onClick={() => { setActiveTab("execution"); setIsPopoverOpen(true); }}
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

          {/* Queued Count (Clickable Popover) + Info Tooltip */}
          <div className="flex items-center gap-1.5">
            <QueuedTasksPopover
              projectId={projectId}
              queuedCount={queuedCount}
              alignOffset={POPOVER_ALIGN_TO_SEPARATOR_DOT}
            >
              <button
                data-testid="queued-count"
                className="text-[13px] cursor-pointer hover:underline transition-all"
                style={{ color: queuedCount > 0 ? STATUS_COLORS.ready : "hsl(220 10% 65%)" }}
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
                  {queuedMessageCount > 0 && (
                    <div>
                      <strong className="block mb-1" style={{ color: "hsl(220 10% 95%)" }}>
                        Pending Agent Messages
                      </strong>
                      <p style={{ color: "hsl(220 10% 75%)" }}>
                        {queuedMessageCount} prompt{queuedMessageCount === 1 ? "" : "s"} currently held by
                        pause/capacity barriers. They relaunch automatically on resume or when
                        capacity opens.
                      </p>
                    </div>
                  )}
                </div>
              }
            />
          </div>

          {queuedMessageCount > 0 && (
            <>
              <span style={{ color: "hsl(220 10% 45%)" }}>•</span>
              <div
                data-testid="queued-message-count"
                className="inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[12px]"
                style={{
                  color: "hsl(35 95% 68%)",
                  backgroundColor: "hsla(35 95% 55% / 0.14)",
                  border: "1px solid hsla(35 95% 65% / 0.22)",
                }}
                aria-label={`${queuedMessageCount} queued agent messages held by pause or capacity barriers`}
                title="Queued agent messages held by pause/capacity barriers"
              >
                <AlertTriangle className="h-3.5 w-3.5" />
                <span>
                  {queuedMessageLabel}
                  {queuedMessageCount}
                </span>
              </div>
            </>
          )}

          {/* Ideation Capacity Indicator - only visible when max > 0 */}
          {showIdeation && (
            <>
              <span style={{ color: "hsl(220 10% 45%)" }}>•</span>
              <div className="flex items-center gap-1.5">
                <button
                  data-testid="ideation-count"
                  data-ideation-trigger
                  className="text-[13px] font-medium cursor-pointer hover:opacity-80 transition-opacity"
                  style={{ color: ideationActive > 0 ? "var(--accent-primary)" : "hsl(220 10% 65%)" }}
                  onClick={() => { setActiveTab("ideation"); setIsPopoverOpen(true); }}
                  aria-label={`Ideation: ${ideationActive} active, ${ideationMax} max`}
                >
                  {ideationLabel}{ideationActive}/{ideationMax}
                </button>
                {ideationWaiting > 0 && (
                  <span
                    data-testid="ideation-waiting-badge"
                    className="inline-flex items-center rounded-full px-1.5 py-0.5 text-[11px] font-medium"
                    style={{
                      color: "hsl(35 95% 68%)",
                      backgroundColor: "hsla(35 95% 55% / 0.14)",
                      border: "1px solid hsla(35 95% 65% / 0.22)",
                    }}
                    title={`${ideationWaiting} ideation session${ideationWaiting === 1 ? "" : "s"} waiting for capacity`}
                  >
                    +{ideationWaiting}
                  </span>
                )}
              </div>
            </>
          )}

          {/* Paused Count (Clickable Popover) - only visible when > 0 */}
          {pausedCount > 0 && (
            <>
              <span style={{ color: "hsl(220 10% 45%)" }}>•</span>
              <PausedTasksPopover
                pausedTasks={pausedTasks}
                alignOffset={POPOVER_ALIGN_TO_SEPARATOR_DOT}
              >
                <button
                  data-testid="paused-count"
                  className="text-[13px] cursor-pointer hover:opacity-80 transition-opacity"
                  style={{ color: STATUS_COLORS.paused }}
                  aria-label="View paused tasks"
                  aria-haspopup="dialog"
                >
                  {pausedLabel}{pausedCount}
                </button>
              </PausedTasksPopover>
            </>
          )}

          {/* Separator */}
          <span style={{ color: "hsl(220 10% 45%)" }}>•</span>

          {/* Merging Count with Popover */}
          {mergePipelineData ? (
            <MergePipelinePopover
              active={mergePipelineData.active}
              waiting={mergePipelineData.waiting}
              needsAttention={mergePipelineData.needsAttention}
              runningCount={runningCount}
              alignOffset={POPOVER_ALIGN_TO_SEPARATOR_DOT}
            >
              <button
                data-testid="merging-count"
                className="flex items-center gap-1.5 text-[13px] cursor-pointer hover:opacity-80 transition-opacity"
                style={{ color: mergingCount > 0 ? STATUS_COLORS.pendingMerge : "hsl(220 10% 65%)" }}
              >
                {mergingLabel}{mergingCount}
                {hasAttentionMerges && (
                  <span
                    data-testid="merge-attention-indicator"
                    className="text-sm"
                    style={{ color: STATUS_COLORS.mergeAttention }}
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
                  style={{ color: STATUS_COLORS.mergeAttention }}
                  title="Some merges need attention"
                >
                  ⚠
                </span>
              )}
            </span>
          )}
        </div>

        {/* Progress Section (Center) - Conditional */}
        {isRunning && currentTaskName && (
          <div
            data-testid="current-task"
            className="flex items-center gap-2 max-w-[40%] task-name-enter"
          >
            <Loader2
              className="w-4 h-4 animate-spin shrink-0"
              style={{ color: STATUS_COLORS.running }}
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
          {/* Pause/Resume Button */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                data-testid="pause-toggle-button"
                variant="ghost"
                size="default"
                onClick={onPauseToggle}
                disabled={!canPauseToggle}
                aria-label={isStopped ? "Start execution" : isPaused ? "Resume execution" : "Pause execution"}
                aria-pressed={isPaused && !isStopped}
                className="gap-2 h-9 px-4 transition-all duration-150 active:scale-[0.96] rounded-lg text-[13px]"
                style={{
                  /* macOS Tahoe: flat button styling */
                  backgroundColor: isStopped
                    ? "hsla(14 100% 60% / 0.15)"
                    : isPaused
                      ? "hsla(45 90% 55% / 0.15)"
                      : "hsl(220 10% 18%)",
                  color: isStopped
                    ? "hsl(14 100% 68%)"
                    : isPaused
                      ? STATUS_COLORS.paused
                      : "hsl(220 10% 90%)",
                  border: "none",
                  opacity: canPauseToggle ? 1 : 0.5,
                }}
              >
                {isLoading ? (
                  <Loader2 className="w-[18px] h-[18px] animate-spin" />
                ) : isStopped ? (
                  <Play className="w-[18px] h-[18px]" />
                ) : isPaused ? (
                  <Play className="w-[18px] h-[18px]" />
                ) : (
                  <Pause className="w-[18px] h-[18px]" />
                )}
                <span className="hidden sm:inline">
                  {isStopped ? "Start" : isPaused ? "Resume" : "Pause"}
                </span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>
                {isStopped
                  ? "Start execution again. Stopped tasks remain stopped until you restart them."
                  : isPaused
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
                aria-label={isStopped ? "Execution already stopped" : "Stop all running tasks"}
                aria-disabled={!canStop}
                className="gap-2 h-9 px-4 transition-all duration-150 active:scale-[0.96] rounded-lg text-[13px]"
                style={{
                  /* macOS Tahoe: flat button styling */
                  backgroundColor: canStop ? "hsla(0 70% 55% / 0.15)" : "hsl(220 10% 18%)",
                  color: canStop ? STATUS_COLORS.stop : "hsl(220 10% 45%)",
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
                {isStopped
                  ? "Execution is halted. Press Start or restart a task to run ready work."
                  : canStop
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
