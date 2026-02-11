/**
 * WaitingMergeCard - Compact row for merge waiting in queue
 *
 * Single-line row: clock icon | title | branch (short)
 * Deferred reason available via native tooltip.
 */

import { Clock } from "lucide-react";
import type { MergePipelineTask } from "@/api/merge-pipeline";
import { getStatusIconConfig } from "@/types/status-icons";

interface WaitingMergeCardProps {
  task: MergePipelineTask;
}

export function WaitingMergeCard({ task }: WaitingMergeCardProps) {
  const pendingMergeStyle = getStatusIconConfig("pending_merge");

  const deferredReason = task.isDeferred
    ? task.blockingBranch
      ? `Waiting for ${task.blockingBranch} to merge`
      : "Waiting for active merge to complete"
    : "Pending merge";

  const branchShort = task.targetBranch?.split("/").pop() ?? task.targetBranch;

  return (
    <div
      className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-white/[0.04] transition-colors"
      title={`pending_merge → ${task.targetBranch}\n${deferredReason}`}
    >
      <Clock
        className="w-3.5 h-3.5 shrink-0"
        style={{ color: pendingMergeStyle.color }}
      />
      <span
        className="flex-1 text-xs font-medium truncate min-w-0"
        style={{ color: "hsl(220 10% 70%)" }}
      >
        {task.title}
      </span>
      <span
        className="text-[11px] font-mono shrink-0 max-w-[100px] truncate"
        style={{ color: "hsl(220 10% 38%)" }}
      >
        {branchShort}
      </span>
    </div>
  );
}
