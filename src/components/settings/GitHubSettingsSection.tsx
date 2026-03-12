/**
 * GitHubSettingsSection - GitHub integration settings for project configuration
 *
 * Features:
 * - Remote URL display (read-only)
 * - gh CLI auth status indicator
 * - GitHub PR mode toggle (disabled when remote is not GitHub)
 *
 * Follows SettingsView pattern using shared components.
 */

import { Github, CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";
import {
  useGitRemoteUrl,
  useGhAuthStatus,
  useUpdateGithubPrEnabled,
} from "@/hooks/useGithubSettings";
import { SectionCard, ToggleSettingRow, SettingRow } from "./SettingsView.shared";

/**
 * GitHubSettingsSection component
 *
 * Displays GitHub remote URL, gh CLI auth status, and PR mode toggle.
 */
export function GitHubSettingsSection() {
  const project = useProjectStore(selectActiveProject);
  const { data: remoteUrl, isLoading: isLoadingRemote } = useGitRemoteUrl(
    project?.id ?? null
  );
  const { data: isGhAuthed, isLoading: isLoadingAuth } = useGhAuthStatus();
  const updatePrEnabled = useUpdateGithubPrEnabled();

  // GitHub remote detection: remote URL must contain 'github.com'
  const isGithubRemote = !!remoteUrl && remoteUrl.includes("github.com");
  const isToggleDisabled = !isGithubRemote || updatePrEnabled.isPending;

  const handleToggle = async () => {
    if (!project) return;
    try {
      await updatePrEnabled.mutateAsync({
        projectId: project.id,
        enabled: !project.githubPrEnabled,
      });
      toast.success(
        project.githubPrEnabled ? "PR mode disabled" : "PR mode enabled"
      );
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Failed to update PR mode"
      );
    }
  };

  if (!project) return null;

  return (
    <SectionCard
      icon={<Github className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="GitHub"
      description="Pull request workflow integration"
    >
      {/* Remote URL row - display only */}
      <SettingRow
        id="github-remote-url"
        label="Remote URL"
        description="Git remote origin for this project"
      >
        {isLoadingRemote ? (
          <Loader2 className="w-4 h-4 animate-spin text-[var(--text-muted)]" />
        ) : (
          <span className="text-xs text-[var(--text-secondary)] font-mono max-w-[200px] truncate">
            {remoteUrl ?? "Not configured"}
          </span>
        )}
      </SettingRow>

      {/* gh CLI auth status - display only */}
      <SettingRow
        id="gh-auth-status"
        label="GitHub CLI"
        description="gh auth status — required for PR operations"
      >
        {isLoadingAuth ? (
          <Loader2 className="w-4 h-4 animate-spin text-[var(--text-muted)]" />
        ) : isGhAuthed ? (
          <div className="flex items-center gap-1.5 text-xs text-green-400">
            <CheckCircle2 className="w-3.5 h-3.5" />
            <span>Authenticated</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-xs text-[var(--status-warning)]">
            <XCircle className="w-3.5 h-3.5" />
            <span>Not authenticated</span>
          </div>
        )}
      </SettingRow>

      {/* PR mode toggle */}
      <ToggleSettingRow
        id="github-pr-enabled"
        label="GitHub PR Mode"
        description={
          !isGithubRemote
            ? "Remote is not GitHub — PR mode unavailable"
            : !isGhAuthed
            ? "Enable to create draft PRs when plans execute (gh auth required for PR operations)"
            : "Create draft PRs when plans execute instead of merging directly"
        }
        checked={isGithubRemote && (project.githubPrEnabled ?? false)}
        disabled={isToggleDisabled}
        onChange={handleToggle}
      />

      {/* Saving indicator */}
      {updatePrEnabled.isPending && (
        <div className="flex items-center gap-2 mt-2 text-xs text-[var(--text-muted)]">
          <Loader2 className="w-3 h-3 animate-spin" />
          <span>Saving...</span>
        </div>
      )}
    </SectionCard>
  );
}
