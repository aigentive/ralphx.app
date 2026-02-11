/**
 * ActiveMergeCard - Compact row for actively merging task
 *
 * Single-line row: spinner | title | branch (short) | elapsed | stop button
 */

import { Square, Loader2 } from "lucide-react";
import type { MergePipelineTask } from "@/api/merge-pipeline";
import { useCallback, useState } from "react";

interface ActiveMergeCardProps {
  task: MergePipelineTask;
  onStop: (taskId: string) => void;
}

function formatElapsedTime(startTime: Date): string {
  const now = new Date();
  const elapsed = Math.floor((now.getTime() - startTime.getTime()) / 1000);

  if (elapsed < 60) return `${elapsed}s`;
  if (elapsed < 3600) return `${Math.floor(elapsed / 60)}m ${elapsed % 60}s`;

  const hours = Math.floor(elapsed / 3600);
  const minutes = Math.floor((elapsed % 3600) / 60);
  return `${hours}h ${minutes}m`;
}

export function ActiveMergeCard({ task, onStop }: ActiveMergeCardProps) {
  const [startTime] = useState(() => new Date(Date.now() - 30000));
  const elapsedTime = formatElapsedTime(startTime);

  const handleStop = useCallback(() => {
    onStop(task.taskId);
  }, [onStop, task.taskId]);

  const branchShort = task.targetBranch?.split("/").pop() ?? task.targetBranch;
  const conflictInfo = task.conflictFiles && task.conflictFiles.length > 0
    ? ` | ${task.conflictFiles.length} conflicts`
    : "";

  return (
    <div
      className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-white/[0.04] transition-colors"
      title={`merging → ${task.targetBranch}${conflictInfo}\nAgent resolving | ${elapsedTime}`}
    >
      <Loader2
        className="w-3.5 h-3.5 animate-spin shrink-0"
        style={{ color: "hsl(180 60% 50%)" }}
      />
      <span
        className="flex-1 text-xs font-medium truncate min-w-0"
        style={{ color: "hsl(220 10% 88%)" }}
      >
        {task.title}
      </span>
      <span
        className="text-[11px] font-mono shrink-0 max-w-[100px] truncate"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        {branchShort}
      </span>
      {task.conflictFiles && task.conflictFiles.length > 0 && (
        <span
          className="text-[11px] shrink-0"
          style={{ color: "hsl(45 90% 55%)" }}
        >
          {task.conflictFiles.length}cf
        </span>
      )}
      <span
        className="text-[11px] shrink-0 tabular-nums"
        style={{ color: "hsl(220 10% 42%)" }}
      >
        {elapsedTime}
      </span>
      <button
        onClick={handleStop}
        className="w-6 h-6 flex items-center justify-center rounded hover:bg-white/[0.08] transition-colors shrink-0"
        style={{ color: "hsl(0 70% 60%)" }}
        title="Stop merge"
      >
        <Square className="w-2.5 h-2.5 fill-current" />
      </button>
    </div>
  );
}
