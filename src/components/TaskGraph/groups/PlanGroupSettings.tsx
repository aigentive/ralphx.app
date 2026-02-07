/**
 * PlanGroupSettings.tsx - Settings popover for plan groups in the Task Graph
 *
 * Shows feature branch toggle, branch info, status, and merge task link.
 * Opens from the gear icon in PlanGroupHeader.
 */

import { memo, useCallback, useState } from "react";
import {
  GitBranch,
  Check,
  X,
  AlertTriangle,
  ExternalLink,
  Loader2,
  Trash2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/tauri";
import type { PlanBranch } from "@/api/plan-branch.types";

// ============================================================================
// Types
// ============================================================================

export interface PlanGroupSettingsProps {
  planArtifactId: string;
  sessionId: string;
  projectId: string;
  /** Current plan branch data (null if no feature branch) */
  planBranch: PlanBranch | null;
  /** Whether any tasks have been merged (prevents disable) */
  hasMergedTasks: boolean;
  /** Callback after enable/disable to refresh data */
  onBranchChange?: () => void;
  /** Navigate to merge task in graph */
  onNavigateToMergeTask?: (taskId: string) => void;
  /** Delete this plan (shows confirmation dialog) */
  onDeletePlan?: () => void;
}

// ============================================================================
// Status indicator component
// ============================================================================

const BranchStatusIndicator = memo(function BranchStatusIndicator({
  status,
}: {
  status: PlanBranch["status"];
}) {
  switch (status) {
    case "active":
      return (
        <span className="inline-flex items-center gap-1 text-[11px] text-[hsl(145,60%,45%)]">
          <span className="w-1.5 h-1.5 rounded-full bg-[hsl(145,60%,45%)]" />
          Active
        </span>
      );
    case "merged":
      return (
        <span className="inline-flex items-center gap-1 text-[11px] text-[hsl(220,80%,60%)]">
          <Check className="w-3 h-3" />
          Merged
        </span>
      );
    case "abandoned":
      return (
        <span className="inline-flex items-center gap-1 text-[11px] text-[hsl(var(--text-muted))]">
          <X className="w-3 h-3" />
          Abandoned
        </span>
      );
  }
});

// ============================================================================
// Main Component
// ============================================================================

export const PlanGroupSettings = memo(function PlanGroupSettings({
  planArtifactId,
  sessionId,
  projectId,
  planBranch,
  hasMergedTasks,
  onBranchChange,
  onNavigateToMergeTask,
  onDeletePlan,
}: PlanGroupSettingsProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isEnabled = planBranch !== null && planBranch.status !== "abandoned";
  const canDisable = isEnabled && !hasMergedTasks && planBranch?.status === "active";

  const handleToggle = useCallback(async (checked: boolean) => {
    setError(null);
    setIsLoading(true);
    try {
      if (checked) {
        await api.planBranches.enable({
          planArtifactId,
          sessionId,
          projectId,
        });
      } else {
        await api.planBranches.disable(planArtifactId);
      }
    } catch (err) {
      const message = typeof err === "string"
        ? err
        : err instanceof Error
          ? err.message
          : "Failed to update feature branch";
      // Silence "already exists" — just stale UI state, refetch will correct it
      if (!message.toLowerCase().includes("already exists")) {
        setError(message);
      }
    } finally {
      setIsLoading(false);
      onBranchChange?.();
    }
  }, [planArtifactId, sessionId, projectId, onBranchChange]);

  return (
    <div className="flex flex-col gap-3 min-w-[240px]">
      {/* Header */}
      <div className="flex items-center gap-2">
        <GitBranch className="w-3.5 h-3.5 text-[hsl(var(--text-muted))]" />
        <span className="text-xs font-medium text-[hsl(var(--text-primary))]">
          Feature Branch
        </span>
      </div>

      {/* Toggle row */}
      <div className="flex items-center justify-between">
        <span className="text-[11px] text-[hsl(var(--text-secondary))]">
          Isolate plan work
        </span>
        <div className="flex items-center gap-2">
          {isLoading && (
            <Loader2 className="w-3 h-3 text-[hsl(var(--text-muted))] animate-spin" />
          )}
          <Switch
            checked={isEnabled}
            onCheckedChange={handleToggle}
            disabled={isLoading || (isEnabled && !canDisable)}
            className="scale-90"
          />
        </div>
      </div>

      {/* Warning if tasks merged (can't disable) */}
      {isEnabled && hasMergedTasks && (
        <div className="flex items-start gap-1.5 text-[11px] text-[hsl(45,90%,55%)]">
          <AlertTriangle className="w-3 h-3 mt-0.5 flex-shrink-0" />
          <span>Tasks already merged to this branch. Cannot disable.</span>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="text-[11px] text-[hsl(0,70%,55%)]">
          {error}
        </div>
      )}

      {/* Branch info (when enabled) */}
      {planBranch && (
        <div className="flex flex-col gap-2 pt-2 border-t border-[hsl(var(--border-subtle))]">
          {/* Branch name */}
          <div className="flex flex-col gap-0.5">
            <span className="text-[10px] uppercase tracking-wider text-[hsl(var(--text-muted))]">
              Branch
            </span>
            <span
              className={cn(
                "text-[11px] font-mono text-[hsl(var(--text-secondary))]",
                "bg-[hsl(var(--bg-surface))] px-1.5 py-0.5 rounded"
              )}
            >
              {planBranch.branchName}
            </span>
          </div>

          {/* Status */}
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase tracking-wider text-[hsl(var(--text-muted))]">
              Status
            </span>
            <BranchStatusIndicator status={planBranch.status} />
          </div>

          {/* Source branch */}
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase tracking-wider text-[hsl(var(--text-muted))]">
              Source
            </span>
            <span className="text-[11px] font-mono text-[hsl(var(--text-muted))]">
              {planBranch.sourceBranch}
            </span>
          </div>

          {/* Merge task link */}
          {planBranch.mergeTaskId && onNavigateToMergeTask && (
            <button
              className={cn(
                "flex items-center gap-1.5 mt-1",
                "text-[11px] text-[hsl(var(--accent-primary))]",
                "hover:underline cursor-pointer"
              )}
              onClick={() => onNavigateToMergeTask(planBranch.mergeTaskId!)}
            >
              <ExternalLink className="w-3 h-3" />
              View merge task
            </button>
          )}
        </div>
      )}

      {/* Delete plan */}
      {onDeletePlan && (
        <div className="pt-2 border-t border-[hsl(var(--border-subtle))]">
          <button
            className={cn(
              "flex items-center gap-1.5 w-full px-2 py-1.5 rounded text-[11px]",
              "text-[hsl(0,70%,55%)] hover:bg-[hsla(0,70%,55%,0.1)]",
              "transition-colors cursor-pointer"
            )}
            onClick={onDeletePlan}
          >
            <Trash2 className="w-3 h-3" />
            Delete Plan
          </button>
        </div>
      )}
    </div>
  );
});
