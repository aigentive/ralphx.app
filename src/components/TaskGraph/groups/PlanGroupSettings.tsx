/**
 * PlanGroupSettings.tsx - Settings popover for plan groups in the Task Graph
 *
 * Shows feature branch toggle, branch info, status, and merge task link.
 * Opens from the gear icon in PlanGroupHeader.
 */

import { memo, useCallback, useEffect, useRef, useState } from "react";
import {
  GitBranch,
  Check,
  X,
  AlertTriangle,
  ExternalLink,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/tauri";
import { getGitBranches } from "@/api/projects";
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
  /** Working directory for fetching git branches */
  workingDirectory?: string;
  /** Project's default base branch — initializes pendingBranch when feature branch is being enabled */
  baseBranch?: string;
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
  workingDirectory,
  baseBranch,
}: PlanGroupSettingsProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // hasUserSelected: prevents useEffect from overwriting user's manual branch selection when baseBranch prop re-hydrates from store
  const hasUserSelected = useRef(false);
  const [pendingBranch, setPendingBranch] = useState<string>(baseBranch ?? "main");
  const [branches, setBranches] = useState<string[]>([]);

  // Sync pendingBranch when baseBranch prop changes (handles async store hydration)
  useEffect(() => {
    if (baseBranch && !hasUserSelected.current) {
      setPendingBranch(baseBranch);
    }
  }, [baseBranch]);
  const [showBranchSelector, setShowBranchSelector] = useState(false);

  const isEnabled = planBranch !== null && planBranch.status !== "abandoned";
  const canDisable = isEnabled && !hasMergedTasks && planBranch?.status === "active";

  const handleToggle = useCallback(async (checked: boolean) => {
    setError(null);
    if (checked) {
      // Load branches and show selector
      setShowBranchSelector(true);
      if (workingDirectory) {
        try {
          const branchList = await getGitBranches(workingDirectory);
          setBranches(branchList);
          // pendingBranch is already initialized from baseBranch — no auto-select override
        } catch {
          // No git repo or inaccessible — user can type manually
          setBranches([]);
        }
      }
    } else {
      setIsLoading(true);
      try {
        await api.planBranches.disable(planArtifactId);
      } catch (err) {
        const message = typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : "Failed to disable feature branch";
        setError(message);
      } finally {
        setIsLoading(false);
        onBranchChange?.();
      }
    }
  }, [planArtifactId, workingDirectory, onBranchChange]);

  const handleConfirmEnable = useCallback(async () => {
    const trimmedBranch = pendingBranch.trim();
    setIsLoading(true);
    try {
      await api.planBranches.enable({
        planArtifactId,
        sessionId,
        projectId,
        ...(trimmedBranch ? { baseBranchOverride: trimmedBranch } : {}),
      });
      setShowBranchSelector(false);
    } catch (err) {
      const message = typeof err === "string"
        ? err
        : err instanceof Error
          ? err.message
          : "Failed to enable feature branch";
      if (!message.toLowerCase().includes("already exists")) {
        setError(message);
      }
    } finally {
      setIsLoading(false);
      onBranchChange?.();
    }
  }, [planArtifactId, sessionId, projectId, pendingBranch, onBranchChange]);

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

      {/* Branch selector when enabling */}
      {showBranchSelector && (
        <div className="flex flex-col gap-2 pt-1">
          <span className="text-[11px] text-[hsl(var(--text-secondary))]">
            Base branch to merge into:
          </span>
          <input
            id="plan-group-branch-input"
            type="text"
            list="plan-group-branch-datalist"
            value={pendingBranch}
            onChange={(e) => {
              hasUserSelected.current = true;
              setPendingBranch(e.target.value);
            }}
            placeholder="e.g. main"
            data-testid="plan-group-branch-input"
            className="w-full px-2 py-1 text-[11px] rounded border outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none"
            style={{
              backgroundColor: "var(--bg-surface)",
              borderColor: "var(--border-subtle)",
              color: "var(--text-primary)",
              boxShadow: "none",
            }}
          />
          <datalist id="plan-group-branch-datalist">
            {branches.map((b) => (
              <option key={b} value={b} />
            ))}
          </datalist>
          <div className="flex gap-2">
            <button
              onClick={handleConfirmEnable}
              disabled={!pendingBranch.trim()}
              className="text-[11px] px-2 py-1 rounded bg-[hsl(var(--accent-primary))] text-white disabled:opacity-50"
            >
              Enable
            </button>
            <button
              onClick={() => setShowBranchSelector(false)}
              className="text-[11px] px-2 py-1 rounded bg-[hsl(var(--bg-surface))] text-[hsl(var(--text-secondary))]"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

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

          {/* Merge target */}
          {planBranch.baseBranchOverride && (
            <div className="flex items-center justify-between">
              <span className="text-[10px] uppercase tracking-wider text-[hsl(var(--text-muted))]">
                Merge Target
              </span>
              <span className="text-[11px] font-mono text-[hsl(var(--text-muted))]">
                {planBranch.baseBranchOverride}
              </span>
            </div>
          )}

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

    </div>
  );
});
