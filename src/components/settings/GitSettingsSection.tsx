/**
 * GitSettingsSection - Git settings for project configuration
 *
 * Features:
 * - Git Mode selector: Worktree (Recommended) / Local Branches
 * - Editable Base Branch with "Detect Default" action
 * - Worktree Location setting (when in worktree mode), persisted per project
 *
 * Follows SettingsView pattern using shared components.
 */

import { useState, useCallback, useEffect } from "react";
import { GitBranch, AlertTriangle, Loader2, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { api, getGitDefaultBranch } from "@/lib/tauri";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import type { GitMode, Project } from "@/types/project";
import { SectionCard, SelectSettingRow, SettingRow } from "./SettingsView.shared";

/**
 * Git mode options for the select dropdown
 */
const GIT_MODE_OPTIONS: {
  value: GitMode;
  label: string;
  description: string;
}[] = [
  {
    value: "worktree",
    label: "Isolated Worktrees (Recommended)",
    description: "Each task gets a separate working directory",
  },
  {
    value: "local",
    label: "Local Branches",
    description: "Single directory, one task at a time",
  },
];

/**
 * Text input row for editable settings with optional action button
 */
function TextSettingRow({
  id,
  label,
  description,
  value,
  placeholder,
  disabled,
  onChange,
  onBlur,
  actionLabel,
  actionIcon,
  onAction,
  actionLoading,
}: {
  id: string;
  label: string;
  description: string;
  value: string;
  placeholder?: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onBlur?: () => void;
  actionLabel?: string;
  actionIcon?: React.ReactNode;
  onAction?: () => void;
  actionLoading?: boolean;
}) {
  return (
    <SettingRow id={id} label={label} description={description} isDisabled={disabled}>
      <div className="flex items-center gap-2">
        <Input
          id={id}
          data-testid={id}
          value={value}
          placeholder={placeholder}
          disabled={disabled}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          className="w-[200px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)] text-sm"
        />
        {onAction && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onAction}
            disabled={disabled || actionLoading}
            className="h-8 px-2 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
            title={actionLabel}
          >
            {actionLoading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              actionIcon
            )}
            {actionLabel && <span className="ml-1">{actionLabel}</span>}
          </Button>
        )}
      </div>
    </SettingRow>
  );
}

/**
 * GitSettingsSection component
 *
 * Allows users to configure git mode, base branch, and worktree location for the active project.
 */
export function GitSettingsSection() {
  const project = useProjectStore(selectActiveProject);
  const updateProject = useProjectStore((s) => s.updateProject);

  // Local state for pending changes
  const [isUpdating, setIsUpdating] = useState(false);
  const [isDetectingDefault, setIsDetectingDefault] = useState(false);
  const [pendingBaseBranch, setPendingBaseBranch] = useState<string | null>(null);
  const [pendingWorktreeDir, setPendingWorktreeDir] = useState<string | null>(null);

  // Reset pending state when project changes
  useEffect(() => {
    setPendingBaseBranch(null);
    setPendingWorktreeDir(null);
  }, [project?.id]);

  // Handler for git mode change
  const handleGitModeChange = useCallback(
    async (newMode: GitMode, currentProject: Project | null) => {
      if (!currentProject || newMode === currentProject.gitMode) return;

      setIsUpdating(true);
      try {
        await api.projects.changeGitMode(
          currentProject.id,
          newMode,
          newMode === "worktree" ? pendingWorktreeDir ?? undefined : undefined
        );

        // Update local store
        updateProject(currentProject.id, { gitMode: newMode });
        toast.success(`Git mode changed to ${newMode === "worktree" ? "Isolated Worktrees" : "Local Branches"}`);
      } catch (error) {
        toast.error(
          error instanceof Error ? error.message : "Failed to change git mode"
        );
      } finally {
        setIsUpdating(false);
      }
    },
    [pendingWorktreeDir, updateProject]
  );

  // Handler for base branch change (local state)
  const handleBaseBranchChange = useCallback((value: string) => {
    setPendingBaseBranch(value);
  }, []);

  // Handler for persisting base branch on blur
  const handleBaseBranchBlur = useCallback(async () => {
    if (!project || pendingBaseBranch === null) return;

    const newValue = pendingBaseBranch.trim();
    // Only persist if value changed
    if (newValue === (project.baseBranch ?? "")) {
      setPendingBaseBranch(null);
      return;
    }

    setIsUpdating(true);
    try {
      await api.projects.update(project.id, {
        baseBranch: newValue || null,
      });
      updateProject(project.id, { baseBranch: newValue || null });
      setPendingBaseBranch(null);
      toast.success("Base branch updated");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to update base branch"
      );
    } finally {
      setIsUpdating(false);
    }
  }, [project, pendingBaseBranch, updateProject]);

  // Handler for detecting default branch
  const handleDetectDefaultBranch = useCallback(async () => {
    if (!project?.workingDirectory) {
      toast.error("No working directory set for this project");
      return;
    }

    setIsDetectingDefault(true);
    try {
      const defaultBranch = await getGitDefaultBranch(project.workingDirectory);

      // Update both local state and persist to backend
      setIsUpdating(true);
      await api.projects.update(project.id, {
        baseBranch: defaultBranch,
      });
      updateProject(project.id, { baseBranch: defaultBranch });
      setPendingBaseBranch(null);
      toast.success(`Detected default branch: ${defaultBranch}`);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to detect default branch"
      );
    } finally {
      setIsDetectingDefault(false);
      setIsUpdating(false);
    }
  }, [project, updateProject]);

  // Handler for worktree directory change (local state)
  const handleWorktreeDirChange = useCallback((value: string) => {
    setPendingWorktreeDir(value);
  }, []);

  // Handler for persisting worktree directory on blur
  const handleWorktreeDirBlur = useCallback(async () => {
    if (!project || pendingWorktreeDir === null) return;

    const newValue = pendingWorktreeDir.trim();
    // Only persist if value changed
    if (newValue === (project.worktreeParentDirectory ?? "~/ralphx-worktrees")) {
      setPendingWorktreeDir(null);
      return;
    }

    setIsUpdating(true);
    try {
      await api.projects.update(project.id, {
        worktreeParentDirectory: newValue || null,
      });
      updateProject(project.id, { worktreeParentDirectory: newValue || null });
      setPendingWorktreeDir(null);
      toast.success("Worktree location updated");
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Failed to update worktree location"
      );
    } finally {
      setIsUpdating(false);
    }
  }, [project, pendingWorktreeDir, updateProject]);

  // Early return if no project selected (after all hooks)
  if (!project) {
    return null;
  }

  const currentGitMode = project.gitMode;
  const baseBranch = pendingBaseBranch ?? project.baseBranch ?? "";
  const worktreeParentDirectory =
    pendingWorktreeDir ?? project.worktreeParentDirectory ?? "~/ralphx-worktrees";

  return (
    <SectionCard
      icon={<GitBranch className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Git"
      description="Version control settings"
    >
      <SelectSettingRow
        id="git-mode"
        label="Git Mode"
        description="How tasks are isolated during execution"
        value={currentGitMode}
        options={GIT_MODE_OPTIONS}
        disabled={isUpdating}
        onChange={(newMode) => handleGitModeChange(newMode, project)}
      />

      {/* Warning banner for local mode */}
      {currentGitMode === "local" && (
        <div
          className="flex items-start gap-2 p-3 rounded-md my-2"
          style={{
            background: "rgba(245, 158, 11, 0.08)",
            border: "1px solid rgba(245, 158, 11, 0.2)",
          }}
        >
          <AlertTriangle className="w-4 h-4 text-[var(--status-warning)] shrink-0 mt-0.5" />
          <p className="text-xs text-[var(--text-muted)]">
            Local mode allows only one task to execute at a time. Your uncommitted
            changes may be affected during execution.
          </p>
        </div>
      )}

      <TextSettingRow
        id="base-branch"
        label="Base Branch"
        description="The branch tasks are merged into"
        value={baseBranch}
        placeholder="main"
        disabled={isUpdating || isDetectingDefault}
        onChange={handleBaseBranchChange}
        onBlur={handleBaseBranchBlur}
        actionLabel="Detect"
        actionIcon={<RefreshCw className="w-3.5 h-3.5" />}
        onAction={handleDetectDefaultBranch}
        actionLoading={isDetectingDefault}
      />

      {currentGitMode === "worktree" && (
        <TextSettingRow
          id="worktree-location"
          label="Worktree Location"
          description="Directory where task worktrees are created"
          value={worktreeParentDirectory}
          placeholder="~/ralphx-worktrees"
          disabled={isUpdating}
          onChange={handleWorktreeDirChange}
          onBlur={handleWorktreeDirBlur}
        />
      )}

      {/* Show saving indicator */}
      {isUpdating && (
        <div className="flex items-center gap-2 mt-2 text-xs text-[var(--text-muted)]">
          <Loader2 className="w-3 h-3 animate-spin" />
          <span>Saving...</span>
        </div>
      )}
    </SectionCard>
  );
}
