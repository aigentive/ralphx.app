/**
 * ProcessCard - Compact two-line row for a running process
 *
 * Line 1: status spinner | title | status badge | pause/stop
 * Line 2: step progress · elapsed · origin · branch
 */

import { Pause, Square, Loader2 } from "lucide-react";
import type { RunningProcess } from "@/api/running-processes";
import { getStatusIconConfig } from "@/types/status-icons";
import { useUiStore } from "@/stores/uiStore";
import { useElapsedTimer } from "@/hooks/useElapsedTimer";
import { formatElapsedTime } from "@/lib/formatters";

interface ProcessCardProps {
  /** The running process data */
  process: RunningProcess;
  /** Called when pause button clicked */
  onPause: (taskId: string) => void;
  /** Called when stop button clicked */
  onStop: (taskId: string) => void;
  /** Whether pause/stop actions are in progress */
  isLoading?: boolean;
}

/**
 * Get status badge color and icon based on internal status
 */
function getStatusBadgeStyle(status: string): {
  color: string;
  bgColor: string;
  label: string;
} {
  const statusConfig = getStatusIconConfig(status);
  const statusLower = status.toLowerCase();
  const hslMatch = statusConfig.color.match(/^hsl\((.+)\)$/);
  const bgColor = hslMatch
    ? `hsla(${hslMatch[1]} / ${statusConfig.bgOpacity})`
    : statusConfig.color;

  switch (statusLower) {
    case "executing":
      return {
        color: statusConfig.color,
        bgColor,
        label: "Executing",
      };
    case "re_executing":
      return {
        color: statusConfig.color,
        bgColor,
        label: "Re-executing",
      };
    case "reviewing":
      return {
        color: statusConfig.color,
        bgColor,
        label: "Reviewing",
      };
    case "qa_refining":
      return {
        color: statusConfig.color,
        bgColor,
        label: "QA Refining",
      };
    case "qa_testing":
      return {
        color: statusConfig.color,
        bgColor,
        label: "QA Testing",
      };
    case "merging":
      return {
        color: statusConfig.color,
        bgColor,
        label: "Merging",
      };
    default:
      return {
        color: statusConfig.color,
        bgColor,
        label: status,
      };
  }
}

/**
 * Get origin badge style based on trigger origin
 */
function getOriginBadgeStyle(origin: string | null): {
  color: string;
  bgColor: string;
  label: string;
} | null {
  if (!origin) return null;

  const originLower = origin.toLowerCase();

  switch (originLower) {
    case "scheduler":
      return {
        color: "hsl(220 10% 65%)",
        bgColor: "hsla(220 10% 65% / 0.15)",
        label: "Scheduled",
      };
    case "revision":
      return {
        color: "hsl(25 90% 55%)",
        bgColor: "hsla(25 90% 55% / 0.15)",
        label: "Revision",
      };
    case "recovery":
      return {
        color: "hsl(45 90% 55%)",
        bgColor: "hsla(45 90% 55% / 0.15)",
        label: "Recovered",
      };
    case "retry":
      return {
        color: "hsl(210 90% 60%)",
        bgColor: "hsla(210 90% 60% / 0.15)",
        label: "Retried",
      };
    case "qa":
      return {
        color: "hsl(270 60% 60%)",
        bgColor: "hsla(270 60% 60% / 0.15)",
        label: "QA Cycle",
      };
    default:
      return {
        color: "hsl(220 10% 65%)",
        bgColor: "hsla(220 10% 65% / 0.15)",
        label: origin,
      };
  }
}

export function ProcessCard({
  process,
  onPause,
  onStop,
  isLoading = false,
}: ProcessCardProps) {
  const statusStyle = getStatusBadgeStyle(process.internalStatus);
  const originStyle = getOriginBadgeStyle(process.triggerOrigin);
  const navigateToTask = useUiStore((s) => s.navigateToTask);

  // Live elapsed time ticker (updates every second)
  const elapsedTime = useElapsedTimer(process.elapsedSeconds, process.taskId);

  const stepInfo = process.stepProgress
    ? `Step ${process.stepProgress.currentStep ? process.stepProgress.currentStep.sortOrder + 1 : process.stepProgress.completed}/${process.stepProgress.total}`
    : null;

  return (
    <div
      data-testid={`process-card-${process.taskId}`}
      className="px-2 py-1.5 rounded-md hover:bg-white/[0.04] transition-colors"
    >
      {/* Line 1: Spinner | Title | Status Badge | Actions */}
      <div className="flex items-center gap-2">
        <Loader2
          className="w-3.5 h-3.5 animate-spin shrink-0"
          style={{ color: statusStyle.color }}
        />
        <button
          className="flex-1 text-xs font-medium truncate min-w-0 text-left cursor-pointer hover:opacity-75 transition-opacity"
          style={{ color: "hsl(220 10% 88%)" }}
          title={process.title}
          onClick={() => navigateToTask(process.taskId)}
        >
          {process.title}
        </button>
        <span
          className="text-[10px] font-medium px-1.5 py-0.5 rounded shrink-0"
          style={{
            color: statusStyle.color,
            backgroundColor: statusStyle.bgColor,
          }}
        >
          {statusStyle.label}
        </span>
        <div className="flex items-center shrink-0">
          <button
            data-testid={`pause-button-${process.taskId}`}
            onClick={() => onPause(process.taskId)}
            disabled={isLoading}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors disabled:opacity-40"
            style={{ color: "hsl(220 10% 55%)" }}
            title="Pause task"
          >
            {isLoading ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <Pause className="w-3 h-3" />
            )}
          </button>
          <button
            data-testid={`stop-button-${process.taskId}`}
            onClick={() => onStop(process.taskId)}
            disabled={isLoading}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors disabled:opacity-40"
            style={{ color: "hsl(0 70% 60%)" }}
            title="Stop task"
          >
            {isLoading ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <Square className="w-2.5 h-2.5 fill-current" />
            )}
          </button>
        </div>
      </div>

      {/* Line 2: Step · Elapsed · Origin · Branch */}
      <div
        className="flex items-center gap-1.5 mt-0.5 pl-[22px] text-[11px] min-w-0"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        {stepInfo && <span className="shrink-0">{stepInfo}</span>}
        {stepInfo && (
          <span className="shrink-0" style={{ color: "hsl(220 10% 30%)" }}>
            ·
          </span>
        )}
        <span className="shrink-0 tabular-nums">
          {formatElapsedTime(elapsedTime)}
        </span>
        {originStyle && (
          <>
            <span
              className="shrink-0"
              style={{ color: "hsl(220 10% 30%)" }}
            >
              ·
            </span>
            <span
              className="text-[10px] font-medium px-1 rounded shrink-0"
              style={{
                color: originStyle.color,
                backgroundColor: originStyle.bgColor,
              }}
            >
              {originStyle.label}
            </span>
          </>
        )}
        {process.taskBranch && (
          <>
            <span
              className="shrink-0"
              style={{ color: "hsl(220 10% 30%)" }}
            >
              ·
            </span>
            <span
              className="font-mono text-[10px] truncate min-w-0"
              style={{ color: "hsl(220 10% 40%)" }}
              title={process.taskBranch}
            >
              {process.taskBranch}
            </span>
          </>
        )}
      </div>
    </div>
  );
}
