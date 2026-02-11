/**
 * ProcessCard - Individual running process card
 *
 * Displays task title, status badge with color coding, step progress,
 * elapsed time ticker, trigger origin badge, branch name, and pause/stop buttons.
 */

import { Pause, Square, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { RunningProcess } from "@/api/running-processes";
import { useEffect, useState } from "react";

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
  icon: "spinner" | "clock" | "warning";
  label: string;
} {
  const statusLower = status.toLowerCase();

  switch (statusLower) {
    case "executing":
      return {
        color: "hsl(145 60% 45%)",
        bgColor: "hsla(145 60% 45% / 0.15)",
        icon: "spinner",
        label: "Executing",
      };
    case "re_executing":
      return {
        color: "hsl(25 90% 55%)",
        bgColor: "hsla(25 90% 55% / 0.15)",
        icon: "spinner",
        label: "Re-executing",
      };
    case "reviewing":
      return {
        color: "hsl(210 90% 60%)",
        bgColor: "hsla(210 90% 60% / 0.15)",
        icon: "spinner",
        label: "Reviewing",
      };
    case "qa_refining":
      return {
        color: "hsl(270 60% 60%)",
        bgColor: "hsla(270 60% 60% / 0.15)",
        icon: "spinner",
        label: "QA Refining",
      };
    case "qa_testing":
      return {
        color: "hsl(270 60% 60%)",
        bgColor: "hsla(270 60% 60% / 0.15)",
        icon: "spinner",
        label: "QA Testing",
      };
    case "merging":
      return {
        color: "hsl(180 60% 50%)",
        bgColor: "hsla(180 60% 50% / 0.15)",
        icon: "spinner",
        label: "Merging",
      };
    default:
      return {
        color: "hsl(220 10% 65%)",
        bgColor: "hsla(220 10% 65% / 0.15)",
        icon: "spinner",
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

/**
 * Format elapsed seconds as human-readable duration
 */
function formatElapsedTime(seconds: number | null): string {
  if (seconds === null) return "—";

  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;

  if (mins === 0) {
    return `${secs}s`;
  }

  return `${mins}m ${secs}s`;
}

export function ProcessCard({
  process,
  onPause,
  onStop,
  isLoading = false,
}: ProcessCardProps) {
  const statusStyle = getStatusBadgeStyle(process.internalStatus);
  const originStyle = getOriginBadgeStyle(process.triggerOrigin);

  // Live elapsed time ticker (updates every second)
  const [elapsedTime, setElapsedTime] = useState(process.elapsedSeconds);

  useEffect(() => {
    if (process.elapsedSeconds === null) return;

    // Start from the backend-provided elapsed time
    setElapsedTime(process.elapsedSeconds);

    // Increment every second
    const interval = setInterval(() => {
      setElapsedTime((prev) => (prev !== null ? prev + 1 : null));
    }, 1000);

    return () => clearInterval(interval);
  }, [process.elapsedSeconds, process.taskId]);

  return (
    <div
      data-testid={`process-card-${process.taskId}`}
      className="p-3 rounded-lg"
      style={{
        backgroundColor: "hsl(220 10% 12%)",
        border: "1px solid hsla(220 20% 100% / 0.08)",
      }}
    >
      {/* Header: Title and Status */}
      <div className="flex items-start justify-between gap-2 mb-2">
        <div className="flex-1 min-w-0">
          <h4
            className="text-[13px] font-medium truncate"
            style={{ color: "hsl(220 10% 90%)" }}
            title={process.title}
          >
            {process.title}
          </h4>
        </div>

        {/* Status Badge */}
        <div
          className="flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-medium shrink-0"
          style={{
            color: statusStyle.color,
            backgroundColor: statusStyle.bgColor,
          }}
        >
          {statusStyle.icon === "spinner" && (
            <Loader2 className="w-3 h-3 animate-spin" />
          )}
          {statusStyle.label}
        </div>
      </div>

      {/* Progress Info */}
      <div
        className="flex items-center gap-2 text-[12px] mb-2"
        style={{ color: "hsl(220 10% 65%)" }}
      >
        {/* Step Progress */}
        {process.stepProgress && (
          <span>
            Step {process.stepProgress.inProgress > 0 ? process.stepProgress.completed + 1 : process.stepProgress.completed}/{process.stepProgress.total}
          </span>
        )}

        {process.stepProgress && <span>•</span>}

        {/* Elapsed Time */}
        <span>{formatElapsedTime(elapsedTime)}</span>
      </div>

      {/* Meta Info */}
      <div className="flex items-center gap-2 mb-3">
        {/* Origin Badge */}
        {originStyle && (
          <div
            className="px-2 py-0.5 rounded text-[11px] font-medium"
            style={{
              color: originStyle.color,
              backgroundColor: originStyle.bgColor,
            }}
          >
            {originStyle.label}
          </div>
        )}

        {/* Branch Name */}
        {process.taskBranch && (
          <span
            className="text-[11px] truncate"
            style={{ color: "hsl(220 10% 55%)" }}
            title={process.taskBranch}
          >
            {process.taskBranch}
          </span>
        )}
      </div>

      {/* Action Buttons */}
      <TooltipProvider>
        <div className="flex items-center gap-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                data-testid={`pause-button-${process.taskId}`}
                variant="ghost"
                size="sm"
                onClick={() => onPause(process.taskId)}
                disabled={isLoading}
                className={cn(
                  "h-7 px-2 rounded-md text-[11px]",
                  "transition-all duration-150 active:scale-[0.96]"
                )}
                style={{
                  backgroundColor: "hsl(220 10% 18%)",
                  color: "hsl(220 10% 90%)",
                  border: "none",
                }}
              >
                {isLoading ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Pause className="w-3.5 h-3.5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>Pause this task</p>
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                data-testid={`stop-button-${process.taskId}`}
                variant="ghost"
                size="sm"
                onClick={() => onStop(process.taskId)}
                disabled={isLoading}
                className={cn(
                  "h-7 px-2 rounded-md text-[11px]",
                  "transition-all duration-150 active:scale-[0.96]"
                )}
                style={{
                  backgroundColor: "hsla(0 70% 55% / 0.15)",
                  color: "hsl(0 70% 60%)",
                  border: "none",
                }}
              >
                {isLoading ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Square className="w-3.5 h-3.5 fill-current" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="top">
              <p>Stop this task</p>
            </TooltipContent>
          </Tooltip>
        </div>
      </TooltipProvider>
    </div>
  );
}
