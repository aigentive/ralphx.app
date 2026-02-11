/**
 * ActiveMergeCard - Shows an actively merging task
 *
 * Displays task title, target branch, conflict file count (if any),
 * elapsed time, and a stop button to halt the merge process.
 */

import { Square, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { MergePipelineTask } from "@/api/merge-pipeline";
import { useCallback, useState } from "react";

interface ActiveMergeCardProps {
  task: MergePipelineTask;
  onStop: (taskId: string) => void;
}

/**
 * Format elapsed time from a date
 */
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
  // Mock start time - in production this would come from the task data
  // Using useState with lazy initializer to avoid calling Date.now() during render
  const [startTime] = useState(() => new Date(Date.now() - 30000)); // 30 seconds ago for demo
  const elapsedTime = formatElapsedTime(startTime);

  const handleStop = useCallback(() => {
    onStop(task.taskId);
  }, [onStop, task.taskId]);

  return (
    <div
      className="p-3 rounded-lg"
      style={{
        backgroundColor: "hsl(220 10% 14%)",
        border: "1px solid hsla(220 20% 100% / 0.08)",
      }}
    >
      {/* Header with title and stop button */}
      <div className="flex items-start justify-between gap-2 mb-2">
        <div className="flex items-start gap-2 flex-1 min-w-0">
          <Loader2
            className="w-4 h-4 animate-spin shrink-0 mt-0.5"
            style={{ color: "hsl(180 60% 50%)" }}
          />
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium truncate" style={{ color: "hsl(220 10% 90%)" }}>
              {task.title}
            </div>
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleStop}
          className="h-7 px-2 shrink-0"
          style={{
            backgroundColor: "hsla(0 70% 55% / 0.15)",
            color: "hsl(0 70% 60%)",
          }}
        >
          <Square className="w-3 h-3 fill-current" />
        </Button>
      </div>

      {/* Details */}
      <div className="space-y-1 text-xs" style={{ color: "hsl(220 10% 65%)" }}>
        <div className="flex items-center gap-1">
          <span>merging →</span>
          <span className="font-mono">{task.targetBranch}</span>
          {task.conflictFiles && task.conflictFiles.length > 0 && (
            <span style={{ color: "hsl(45 90% 55%)" }}>
              ({task.conflictFiles.length} conflicts)
            </span>
          )}
        </div>
        <div>┈ Agent resolving • {elapsedTime}</div>
      </div>
    </div>
  );
}
