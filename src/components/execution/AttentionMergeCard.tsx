/**
 * AttentionMergeCard - Shows a merge needing attention
 *
 * Displays task title, error context (conflict files or git error),
 * with View Details and Retry action buttons.
 */

import { AlertTriangle, ExternalLink, RotateCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { MergePipelineTask } from "@/api/merge-pipeline";
import { useCallback } from "react";

interface AttentionMergeCardProps {
  task: MergePipelineTask;
  onViewDetails: (taskId: string) => void;
  onRetry: (taskId: string) => void;
}

export function AttentionMergeCard({ task, onViewDetails, onRetry }: AttentionMergeCardProps) {
  const handleViewDetails = useCallback(() => {
    onViewDetails(task.taskId);
  }, [onViewDetails, task.taskId]);

  const handleRetry = useCallback(() => {
    onRetry(task.taskId);
  }, [onRetry, task.taskId]);

  // Determine error context
  const errorContext = task.conflictFiles && task.conflictFiles.length > 0
    ? `${task.conflictFiles.length} conflicted files • Agent couldn't fix`
    : task.errorContext || "Git error occurred";

  return (
    <div
      className="p-3 rounded-lg"
      style={{
        backgroundColor: "hsla(0 70% 55% / 0.1)",
        border: "1px solid hsla(0 70% 55% / 0.2)",
      }}
    >
      {/* Header with title */}
      <div className="flex items-start gap-2 mb-2">
        <AlertTriangle
          className="w-4 h-4 shrink-0 mt-0.5"
          style={{ color: "hsl(0 70% 60%)" }}
        />
        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium truncate" style={{ color: "hsl(220 10% 90%)" }}>
            {task.title}
          </div>
        </div>
      </div>

      {/* Details */}
      <div className="space-y-2 text-xs">
        <div style={{ color: "hsl(220 10% 65%)" }}>
          <div className="flex items-center gap-1 mb-1">
            <span>{task.internalStatus} →</span>
            <span className="font-mono">{task.targetBranch}</span>
          </div>
          <div>┈ {errorContext}</div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-2 pt-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleViewDetails}
            className="h-7 px-2 text-xs gap-1"
            style={{
              backgroundColor: "hsl(220 10% 18%)",
              color: "hsl(220 10% 90%)",
            }}
          >
            View Details
            <ExternalLink className="w-3 h-3" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleRetry}
            className="h-7 px-2 text-xs gap-1"
            style={{
              backgroundColor: "hsla(14 100% 60% / 0.15)",
              color: "hsl(14 100% 60%)",
            }}
          >
            Retry
            <RotateCw className="w-3 h-3" />
          </Button>
        </div>
      </div>
    </div>
  );
}
