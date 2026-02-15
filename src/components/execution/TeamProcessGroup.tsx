/**
 * TeamProcessGroup - Collapsible group for team-mode running processes
 *
 * Shows a team header with agent count and collapsible teammate sub-entries.
 * Each teammate shows status dot, name, step info, and model.
 */

import { useState } from "react";
import { ChevronDown, ChevronRight, Pause, Square, Loader2 } from "lucide-react";
import type { RunningProcess } from "@/api/running-processes";

interface TeamProcessGroupProps {
  /** The parent process representing the team task */
  process: RunningProcess;
  /** Called when pause button clicked */
  onPause: (taskId: string) => void;
  /** Called when stop button clicked */
  onStop: (taskId: string) => void;
}

/** Status dot color for teammate status */
function getTeammateDotColor(status: string): string {
  switch (status.toLowerCase()) {
    case "active":
    case "executing":
    case "running":
      return "hsl(142 70% 50%)";
    case "completed":
    case "done":
      return "hsl(220 10% 45%)";
    case "failed":
    case "error":
      return "hsl(0 70% 55%)";
    case "idle":
    case "waiting":
      return "hsl(45 90% 55%)";
    default:
      return "hsl(220 10% 45%)";
  }
}

export function TeamProcessGroup({
  process,
  onPause,
  onStop,
}: TeamProcessGroupProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const teammates = process.teammates ?? [];
  const activeCount = teammates.filter(
    (t) => !["completed", "done"].includes(t.status.toLowerCase())
  ).length;

  const ChevronIcon = isExpanded ? ChevronDown : ChevronRight;

  return (
    <div
      data-testid={`team-group-${process.taskId}`}
      className="rounded-md"
      style={{ backgroundColor: "hsla(220 10% 100% / 0.02)" }}
    >
      {/* Team Header */}
      <div className="flex items-center gap-2 px-2 py-1.5 hover:bg-white/[0.04] transition-colors rounded-t-md">
        <button
          data-testid={`team-toggle-${process.taskId}`}
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center justify-center w-4 h-4 shrink-0"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          <ChevronIcon className="w-3 h-3" />
        </button>

        <Loader2
          className="w-3.5 h-3.5 animate-spin shrink-0"
          style={{ color: "hsl(14 100% 60%)" }}
        />

        <span
          className="flex-1 text-xs font-medium truncate min-w-0"
          style={{ color: "hsl(220 10% 88%)" }}
          title={process.title}
        >
          {process.title}
        </span>

        <span
          className="text-[10px] font-medium px-1.5 py-0.5 rounded shrink-0"
          style={{
            color: "hsl(14 100% 60%)",
            backgroundColor: "hsla(14 100% 60% / 0.12)",
          }}
        >
          Team: {activeCount}/{teammates.length}
        </span>

        {/* Pause/Stop for the whole team task */}
        <div className="flex items-center shrink-0">
          <button
            data-testid={`pause-button-${process.taskId}`}
            onClick={() => onPause(process.taskId)}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors"
            style={{ color: "hsl(220 10% 55%)" }}
            title="Pause team"
          >
            <Pause className="w-3 h-3" />
          </button>
          <button
            data-testid={`stop-button-${process.taskId}`}
            onClick={() => onStop(process.taskId)}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors"
            style={{ color: "hsl(0 70% 60%)" }}
            title="Stop team"
          >
            <Square className="w-2.5 h-2.5 fill-current" />
          </button>
        </div>
      </div>

      {/* Teammate Sub-entries */}
      {isExpanded && teammates.length > 0 && (
        <div
          className="pb-1"
          style={{ borderTop: "1px solid hsla(220 10% 100% / 0.04)" }}
        >
          {teammates.map((teammate, idx) => {
            const dotColor = teammate.color ?? getTeammateDotColor(teammate.status);
            const isDone = ["completed", "done"].includes(teammate.status.toLowerCase());

            return (
              <div
                key={teammate.name + idx}
                data-testid={`teammate-${teammate.name}`}
                className="flex items-center gap-2 px-2 py-1 pl-8"
                style={{ opacity: isDone ? 0.5 : 1 }}
              >
                {/* Status dot */}
                <span
                  className="w-1.5 h-1.5 rounded-full shrink-0"
                  style={{ backgroundColor: dotColor }}
                />

                {/* Name */}
                <span
                  className="text-[11px] font-medium shrink-0"
                  style={{ color: "hsl(220 10% 75%)" }}
                >
                  {teammate.name}
                </span>

                {/* Step info */}
                {teammate.step && (
                  <>
                    <span
                      className="shrink-0 text-[11px]"
                      style={{ color: "hsl(220 10% 30%)" }}
                    >
                      ·
                    </span>
                    <span
                      className="text-[10px] truncate min-w-0"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
                      {teammate.step}
                    </span>
                  </>
                )}

                {/* Model badge */}
                {teammate.model && (
                  <>
                    <span className="flex-1" />
                    <span
                      className="text-[9px] font-medium px-1 rounded shrink-0"
                      style={{
                        color: "hsl(220 10% 50%)",
                        backgroundColor: "hsla(220 10% 100% / 0.05)",
                      }}
                    >
                      {teammate.model}
                    </span>
                  </>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
