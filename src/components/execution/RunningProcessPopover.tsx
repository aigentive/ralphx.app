/**
 * RunningProcessPopover - Compact running processes list
 *
 * Dense row-based layout matching macOS Activity Monitor style.
 * Each process is a two-line compact row.
 */

import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Settings } from "lucide-react";
import { ProcessCard } from "./ProcessCard";
import { TeamProcessGroup } from "./TeamProcessGroup";
import type { RunningProcess } from "@/api/running-processes";
import { cn } from "@/lib/utils";

interface RunningProcessPopoverProps {
  /** List of currently running processes */
  processes: RunningProcess[];
  /** Global running count from execution status (source of truth for capacity) */
  runningCount?: number;
  /** Current max concurrent tasks */
  maxConcurrent: number;
  /** Whether popover is open (controlled) */
  open: boolean;
  /** Called when open state changes */
  onOpenChange: (open: boolean) => void;
  /** Called when pause button clicked for a process */
  onPauseProcess: (taskId: string) => void;
  /** Called when stop button clicked for a process */
  onStopProcess: (taskId: string) => void;
  /** Called when settings link clicked */
  onOpenSettings: () => void;
  /** Children (trigger element) */
  children: React.ReactNode;
  /** Optional horizontal alignment offset for popover content */
  alignOffset?: number;
}

export function RunningProcessPopover({
  processes,
  runningCount,
  maxConcurrent,
  open,
  onOpenChange,
  onPauseProcess,
  onStopProcess,
  onOpenSettings,
  children,
  alignOffset = -24,
}: RunningProcessPopoverProps) {
  const effectiveRunningCount = runningCount ?? processes.length;

  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        data-testid="running-process-popover"
        align="start"
        alignOffset={alignOffset}
        side="top"
        sideOffset={24}
        className="w-[420px] p-0 border-0"
        style={{
          backgroundColor: "hsl(220 10% 11%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          borderRadius: "10px",
          boxShadow:
            "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-3 py-2.5"
          style={{
            borderBottom: "1px solid hsla(220 20% 100% / 0.06)",
          }}
        >
          <h3
            className="text-xs font-semibold"
            style={{ color: "hsl(220 10% 80%)" }}
          >
            Running Processes ({effectiveRunningCount}/{maxConcurrent})
          </h3>

          <button
            data-testid="open-settings-button"
            onClick={onOpenSettings}
            className={cn(
              "flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px]",
              "transition-colors hover:bg-white/[0.05]"
            )}
            style={{ color: "hsl(220 10% 50%)" }}
          >
            <Settings className="w-3 h-3" />
            Max: {maxConcurrent}
          </button>
        </div>

        {/* Process List */}
        <div
          className="max-h-[320px] overflow-y-auto p-1.5"
          style={{
            scrollbarWidth: "thin",
            scrollbarColor: "hsla(220 10% 100% / 0.1) transparent",
          }}
        >
          {processes.length === 0 ? (
            <div
              className="py-6 text-center text-xs"
              style={{ color: "hsl(220 10% 42%)" }}
            >
              No running processes
            </div>
          ) : (
            processes.map((process) =>
              process.teamName ? (
                <TeamProcessGroup
                  key={process.taskId}
                  process={process}
                  onPause={onPauseProcess}
                  onStop={onStopProcess}
                />
              ) : (
                <ProcessCard
                  key={process.taskId}
                  process={process}
                  onPause={onPauseProcess}
                  onStop={onStopProcess}
                />
              )
            )
          )}
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-between px-3 py-2 text-[11px]"
          style={{
            borderTop: "1px solid hsla(220 20% 100% / 0.06)",
            color: "hsl(220 10% 42%)",
          }}
        >
          <span>
            Concurrency runs up to {maxConcurrent} tasks in parallel.
          </span>
          <button
            onClick={onOpenSettings}
            className="hover:underline transition-colors shrink-0 ml-2"
            style={{ color: "hsl(14 100% 60%)" }}
          >
            Open Settings
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
