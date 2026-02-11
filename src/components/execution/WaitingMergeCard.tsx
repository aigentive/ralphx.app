/**
 * WaitingMergeCard - Shows a merge waiting in the queue
 *
 * Displays task title, target branch, and deferred reason
 * explaining which specific branch is blocking this merge.
 */

import { Clock } from "lucide-react";
import type { MergePipelineTask } from "@/api/merge-pipeline";

interface WaitingMergeCardProps {
  task: MergePipelineTask;
}

export function WaitingMergeCard({ task }: WaitingMergeCardProps) {
  // Determine deferred reason
  const deferredReason = task.isDeferred
    ? task.blockingBranch
      ? `Waiting for ${task.blockingBranch} to merge`
      : "Waiting for active merge to complete"
    : "Pending merge";

  return (
    <div
      className="p-3 rounded-lg"
      style={{
        backgroundColor: "hsl(220 10% 14%)",
        border: "1px solid hsla(220 20% 100% / 0.08)",
      }}
    >
      {/* Header with title */}
      <div className="flex items-start gap-2 mb-2">
        <Clock
          className="w-4 h-4 shrink-0 mt-0.5"
          style={{ color: "hsl(220 10% 55%)" }}
        />
        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium truncate" style={{ color: "hsl(220 10% 90%)" }}>
            {task.title}
          </div>
        </div>
      </div>

      {/* Details */}
      <div className="space-y-1 text-xs" style={{ color: "hsl(220 10% 65%)" }}>
        <div className="flex items-center gap-1">
          <span>pending_merge →</span>
          <span className="font-mono">{task.targetBranch}</span>
        </div>
        <div>┈ {deferredReason}</div>
      </div>
    </div>
  );
}
