/**
 * AcceptedSessionBanner - Shows acceptance status, live task counts, and "View Work" CTA
 *
 * Rendered at the top of PlanningView when session.status === "accepted".
 * Task counts are live/reactive via the existing useTasks query.
 */

import { useMemo } from "react";
import { CheckCircle2, ArrowRight, Clock, Zap, CircleCheck } from "lucide-react";
import { useTasks } from "@/hooks/useTasks";
import type { TaskProposal } from "@/types/ideation";
import type { Task } from "@/types/task";
import { IDLE_STATUSES, TERMINAL_STATUSES } from "@/types/status";
import type { InternalStatus } from "@/types/status";

interface AcceptedSessionBannerProps {
  projectId: string;
  proposals: TaskProposal[];
  convertedAt: string | null;
  onViewWork: () => void;
}

interface StatusCounts {
  idle: number;
  active: number;
  done: number;
  total: number;
}

function categorizeStatus(status: InternalStatus): "idle" | "active" | "done" {
  if ((IDLE_STATUSES as readonly string[]).includes(status)) return "idle";
  if ((TERMINAL_STATUSES as readonly string[]).includes(status)) return "done";
  return "active";
}

function getStatusCounts(tasks: Task[]): StatusCounts {
  const counts: StatusCounts = { idle: 0, active: 0, done: 0, total: tasks.length };
  for (const task of tasks) {
    const category = categorizeStatus(task.internalStatus);
    counts[category]++;
  }
  return counts;
}

function formatTimestamp(iso: string): string {
  try {
    const date = new Date(iso);
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return "";
  }
}

export function AcceptedSessionBanner({
  projectId,
  proposals,
  convertedAt,
  onViewWork,
}: AcceptedSessionBannerProps) {
  const { data: allTasks } = useTasks(projectId);

  const createdTaskIds = useMemo(
    () => new Set(proposals.filter((p) => p.createdTaskId != null).map((p) => p.createdTaskId!)),
    [proposals]
  );

  const sessionTasks = useMemo(
    () => (allTasks ?? []).filter((t) => createdTaskIds.has(t.id)),
    [allTasks, createdTaskIds]
  );

  const counts = useMemo(() => getStatusCounts(sessionTasks), [sessionTasks]);

  if (createdTaskIds.size === 0) return null;

  return (
    <div
      data-testid="accepted-session-banner"
      className="mb-4 rounded-xl overflow-hidden"
      style={{
        background: "hsla(220 10% 14% / 0.8)",
        border: "1px solid hsla(14 100% 60% / 0.15)",
        boxShadow: "0 0 24px hsla(14 100% 60% / 0.04)",
      }}
    >
      {/* Top accent line */}
      <div
        className="h-[2px] w-full"
        style={{
          background: "linear-gradient(90deg, hsla(14 100% 60% / 0.6) 0%, hsla(14 100% 60% / 0.1) 100%)",
        }}
      />

      <div className="px-4 py-3.5">
        {/* Header row */}
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div
              className="w-5 h-5 rounded-full flex items-center justify-center"
              style={{ background: "hsla(145 70% 45% / 0.15)" }}
            >
              <CheckCircle2 className="w-3 h-3" style={{ color: "hsl(145 70% 45%)" }} />
            </div>
            <span className="text-[13px] font-medium" style={{ color: "hsl(220 10% 90%)" }}>
              Plan accepted
            </span>
            {convertedAt && (
              <span className="text-[11px]" style={{ color: "hsl(220 10% 45%)" }}>
                {formatTimestamp(convertedAt)}
              </span>
            )}
          </div>

          <button
            data-testid="view-work-button"
            onClick={onViewWork}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-semibold transition-all duration-150"
            style={{
              background: "hsl(14 100% 60%)",
              color: "white",
              boxShadow: "0 1px 4px hsla(14 100% 40% / 0.3)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 55%)";
              e.currentTarget.style.boxShadow = "0 2px 8px hsla(14 100% 40% / 0.4)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 60%)";
              e.currentTarget.style.boxShadow = "0 1px 4px hsla(14 100% 40% / 0.3)";
            }}
          >
            View Work
            <ArrowRight className="w-3 h-3" />
          </button>
        </div>

        {/* Status summary */}
        <div className="flex items-center gap-4">
          <span className="text-[12px]" style={{ color: "hsl(220 10% 50%)" }}>
            {counts.total} {counts.total === 1 ? "task" : "tasks"}
          </span>

          {counts.active > 0 && (
            <div className="flex items-center gap-1">
              <Zap className="w-3 h-3" style={{ color: "hsl(14 100% 60%)" }} />
              <span className="text-[11px]" style={{ color: "hsl(14 100% 65%)" }}>
                {counts.active} in progress
              </span>
            </div>
          )}

          {counts.done > 0 && (
            <div className="flex items-center gap-1">
              <CircleCheck className="w-3 h-3" style={{ color: "hsl(145 70% 45%)" }} />
              <span className="text-[11px]" style={{ color: "hsl(145 70% 50%)" }}>
                {counts.done} completed
              </span>
            </div>
          )}

          {counts.idle > 0 && counts.active === 0 && counts.done === 0 && (
            <div className="flex items-center gap-1">
              <Clock className="w-3 h-3" style={{ color: "hsl(220 10% 45%)" }} />
              <span className="text-[11px]" style={{ color: "hsl(220 10% 50%)" }}>
                {counts.idle} queued
              </span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
