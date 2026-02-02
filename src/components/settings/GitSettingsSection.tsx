/**
 * GitSettingsSection - Git settings for project configuration
 *
 * Features:
 * - Git Mode selector: Worktree (Recommended) / Local Branches
 * - Base Branch display (read-only)
 * - Worktree Location setting (when in worktree mode)
 *
 * Follows SettingsView pattern using shared components.
 */

import { useState, useCallback } from "react";
import { GitBranch, AlertTriangle, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/tauri";
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
 * Display row for read-only values (like base branch)
 */
function DisplayRow({
  id,
  label,
  description,
  value,
}: {
  id: string;
  label: string;
  description: string;
  value: string | null;
}) {
  return (
    <SettingRow id={id} label={label} description={description}>
      <span className="text-sm text-[var(--text-secondary)]">
        {value || "Not set"}
      </span>
    </SettingRow>
  );
}

/**
 * Text input row for editable settings
 */
function TextSettingRow({
  id,
  label,
  description,
  value,
  placeholder,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  description: string;
  value: string;
  placeholder?: string;
  disabled: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <SettingRow id={id} label={label} description={description} isDisabled={disabled}>
      <Input
        id={id}
        data-testid={id}
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        className="w-[280px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)] text-sm"
      />
    </SettingRow>
  );
}

/**
 * GitSettingsSection component
 *
 * Allows users to configure git mode and worktree location for the active project.
 */
export function GitSettingsSection() {
  const project = useProjectStore(selectActiveProject);
  const updateProject = useProjectStore((s) => s.updateProject);

  // Local state for pending changes
  const [isUpdating, setIsUpdating] = useState(false);
  const [pendingWorktreeDir, setPendingWorktreeDir] = useState<string | null>(null);

  // Handlers must be defined before early return to follow rules of hooks
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

  const handleWorktreeDirChange = useCallback((value: string) => {
    setPendingWorktreeDir(value);
  }, []);

  // Early return if no project selected (after all hooks)
  if (!project) {
    return null;
  }

  const currentGitMode = project.gitMode;
  const baseBranch = project.baseBranch;
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

      <DisplayRow
        id="base-branch"
        label="Base Branch"
        description="The branch tasks are merged into"
        value={baseBranch}
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
