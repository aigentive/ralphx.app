/**
 * RunningProcessPopover - Displays all currently running processes
 *
 * Shows a scrollable list of ProcessCard components with a header
 * for concurrency control and an info footer explaining the system.
 */

import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Settings } from "lucide-react";
import { ProcessCard } from "./ProcessCard";
import type { RunningProcess } from "@/api/running-processes";
import { cn } from "@/lib/utils";

interface RunningProcessPopoverProps {
  /** List of currently running processes */
  processes: RunningProcess[];
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
}

export function RunningProcessPopover({
  processes,
  maxConcurrent,
  open,
  onOpenChange,
  onPauseProcess,
  onStopProcess,
  onOpenSettings,
  children,
}: RunningProcessPopoverProps) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        data-testid="running-process-popover"
        align="start"
        side="top"
        className="w-[420px] p-0 border-0"
        style={{
          backgroundColor: "hsl(220 10% 10%)",
          border: "1px solid hsla(220 20% 100% / 0.12)",
          borderRadius: "12px",
          boxShadow: `
            0 4px 16px hsla(220 20% 0% / 0.4),
            0 12px 32px hsla(220 20% 0% / 0.3)
          `,
        }}
      >
        {/* Header: Title + Concurrency Control */}
        <div
          className="flex items-center justify-between px-4 py-3"
          style={{
            borderBottom: "1px solid hsla(220 20% 100% / 0.08)",
          }}
        >
          <h3
            className="text-[13px] font-semibold"
            style={{ color: "hsl(220 10% 90%)" }}
          >
            Running Processes ({processes.length}/{maxConcurrent})
          </h3>

          <button
            data-testid="open-settings-button"
            onClick={onOpenSettings}
            className={cn(
              "flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] font-medium",
              "transition-all duration-150 hover:bg-white/5 active:scale-[0.96]"
            )}
            style={{
              color: "hsl(220 10% 65%)",
            }}
          >
            <Settings className="w-3.5 h-3.5" />
            Max: {maxConcurrent}
          </button>
        </div>

        {/* Process List (Scrollable) */}
        <div
          className="max-h-[400px] overflow-y-auto p-3 space-y-2"
          style={{
            scrollbarWidth: "thin",
            scrollbarColor: "hsl(220 10% 30%) transparent",
          }}
        >
          {processes.length === 0 ? (
            <div
              className="py-8 text-center text-[13px]"
              style={{ color: "hsl(220 10% 55%)" }}
            >
              No running processes
            </div>
          ) : (
            processes.map((process) => (
              <ProcessCard
                key={process.taskId}
                process={process}
                onPause={onPauseProcess}
                onStop={onStopProcess}
              />
            ))
          )}
        </div>

        {/* Info Footer */}
        <div
          className="px-4 py-3 text-[11px] leading-relaxed"
          style={{
            borderTop: "1px solid hsla(220 20% 100% / 0.08)",
            color: "hsl(220 10% 55%)",
          }}
        >
          <p className="mb-1">
            <span style={{ color: "hsl(220 10% 65%)" }}>ℹ</span> The
            concurrency system runs up to {maxConcurrent} tasks in parallel.
          </p>
          <button
            onClick={onOpenSettings}
            className="text-[11px] underline hover:no-underline transition-all"
            style={{ color: "hsl(14 100% 60%)" }}
          >
            Open Settings
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
