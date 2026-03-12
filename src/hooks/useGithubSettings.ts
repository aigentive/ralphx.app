/**
 * GitHub settings hooks — wraps Tauri commands for GitHub integration UI.
 *
 * Hooks:
 * - useGitRemoteUrl: fetch remote URL for a project
 * - useGhAuthStatus: check if `gh` CLI is authenticated
 * - useUpdateGithubPrEnabled: mutation to toggle PR mode on a project
 */

import { invoke } from "@tauri-apps/api/core";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useProjectStore } from "@/stores/projectStore";

// ============================================================================
// useGitRemoteUrl
// ============================================================================

/**
 * Fetch the git remote URL for a project.
 * Returns null when no remote is configured or projectId is null.
 *
 * @param projectId - The project ID to fetch the remote URL for, or null to disable
 */
export function useGitRemoteUrl(projectId: string | null) {
  return useQuery<string | null>({
    queryKey: ["git-remote-url", projectId],
    queryFn: () =>
      invoke<string | null>("get_git_remote_url", { projectId }),
    enabled: projectId !== null,
  });
}

// ============================================================================
// useGhAuthStatus
// ============================================================================

/**
 * Check whether the `gh` CLI is authenticated.
 * Returns true when authenticated, false otherwise.
 */
export function useGhAuthStatus() {
  return useQuery<boolean>({
    queryKey: ["gh-auth-status"],
    queryFn: () => invoke<boolean>("check_gh_auth", {}),
  });
}

// ============================================================================
// useUpdateGithubPrEnabled
// ============================================================================

interface UpdateGithubPrEnabledArgs {
  projectId: string;
  enabled: boolean;
}

/**
 * Mutation to toggle GitHub PR mode for a project.
 * On success, updates the projectStore and invalidates the projects query.
 */
export function useUpdateGithubPrEnabled() {
  const queryClient = useQueryClient();
  const updateProject = useProjectStore((s) => s.updateProject);

  return useMutation({
    mutationFn: ({ projectId, enabled }: UpdateGithubPrEnabledArgs) =>
      invoke("update_github_pr_enabled", { projectId, enabled }),
    onSuccess: (_data, { projectId, enabled }) => {
      updateProject(projectId, { githubPrEnabled: enabled });
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
  });
}
