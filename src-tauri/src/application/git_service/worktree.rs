use super::git_cmd;
use super::*;

impl GitService {
    // =========================================================================
    // Worktree Operations (Worktree mode only)
    // =========================================================================

    /// Create a new worktree with a new branch
    ///
    /// NOTE: This method creates parent directories if they don't exist.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `worktree` - Path where the worktree should be created
    /// * `branch` - Name of the new branch to create in the worktree
    /// * `base` - Name of the base branch to branch from
    pub async fn create_worktree(
        repo: &Path,
        worktree: &Path,
        branch: &str,
        base: &str,
    ) -> AppResult<()> {
        debug!(
            "Creating worktree at {:?} with branch '{}' from '{}' in {:?}",
            worktree, branch, base, repo
        );

        // Ensure parent directory exists (per plan: auto-create if needed)
        if let Some(parent) = worktree.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::GitOperation(format!(
                    "Failed to create worktree parent directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        let output = git_cmd::run(
            &[
                "worktree",
                "add",
                "-b",
                branch,
                worktree.to_str().unwrap_or_default(),
                base,
            ],
            repo,
        )
        .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create worktree at {:?}: {}",
                worktree, stderr
            )));
        }

        Ok(())
    }

    /// Delete a worktree
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `worktree` - Path of the worktree to delete
    pub async fn delete_worktree(repo: &Path, worktree: &Path) -> AppResult<()> {
        debug!("Deleting worktree at {:?} from {:?}", worktree, repo);

        let output = git_cmd::run(
            &[
                "worktree",
                "remove",
                "--force",
                worktree.to_str().unwrap_or_default(),
            ],
            repo,
        )
        .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to delete worktree at {:?}: {}", worktree, stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to delete worktree at '{}': {}",
                worktree.to_string_lossy(),
                stderr.trim()
            )));
        }

        Ok(())
    }

    /// Create a worktree that checks out an existing branch (no new branch creation)
    ///
    /// Unlike `create_worktree` which uses `git worktree add -b <new_branch>`,
    /// this method uses `git worktree add <path> <existing_branch>` to check out
    /// a branch that already exists. Used for merge worktrees where we need to
    /// check out the target branch in an isolated directory.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `worktree` - Path where the worktree should be created
    /// * `branch` - Name of the existing branch to check out
    pub async fn checkout_existing_branch_worktree(
        repo: &Path,
        worktree: &Path,
        branch: &str,
    ) -> AppResult<()> {
        debug!(
            "Creating worktree at {:?} checking out existing branch '{}' in {:?}",
            worktree, branch, repo
        );

        // Ensure parent directory exists
        if let Some(parent) = worktree.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::GitOperation(format!(
                    "Failed to create worktree parent directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        let output = git_cmd::run(
            &[
                "worktree",
                "add",
                worktree.to_str().unwrap_or_default(),
                branch,
            ],
            repo,
        )
        .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create worktree at {:?} for branch '{}': {}",
                worktree, branch, stderr
            )));
        }

        Ok(())
    }

    // =========================================================================
    // Worktree Query Operations
    // =========================================================================

    /// List all worktrees in the repository
    ///
    /// Runs `git worktree list --porcelain` and parses the output into
    /// structured `WorktreeInfo` entries.
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub async fn list_worktrees(repo: &Path) -> AppResult<Vec<WorktreeInfo>> {
        debug!("Listing worktrees in {:?}", repo);

        let output = git_cmd::run(&["worktree", "list", "--porcelain"], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to list worktrees: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_worktree_porcelain(&stdout))
    }

    /// Parse `git worktree list --porcelain` output into `WorktreeInfo` entries
    ///
    /// Porcelain format outputs blocks separated by blank lines. Each block has:
    /// - `worktree <path>` (always present)
    /// - `HEAD <sha>` (absent for bare repos)
    /// - `branch refs/heads/<name>` (absent for detached HEAD or bare)
    /// - Optional flags: `bare`, `detached`, `prunable`
    pub(super) fn parse_worktree_porcelain(output: &str) -> Vec<WorktreeInfo> {
        let mut worktrees = Vec::new();
        let mut path: Option<String> = None;
        let mut head: Option<String> = None;
        let mut branch: Option<String> = None;

        for line in output.lines() {
            if line.is_empty() {
                // End of a worktree block — flush
                if let Some(p) = path.take() {
                    worktrees.push(WorktreeInfo {
                        path: p,
                        branch: branch.take(),
                        head: head.take(),
                    });
                }
                head = None;
                branch = None;
            } else if let Some(rest) = line.strip_prefix("worktree ") {
                path = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("HEAD ") {
                head = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("branch ") {
                // Strip refs/heads/ prefix to get the short branch name
                let short = rest.strip_prefix("refs/heads/").unwrap_or(rest);
                branch = Some(short.to_string());
            }
            // Ignore `bare`, `detached`, `prunable` lines
        }

        // Flush last block (porcelain output may not end with a blank line)
        if let Some(p) = path.take() {
            worktrees.push(WorktreeInfo {
                path: p,
                branch: branch.take(),
                head: head.take(),
            });
        }

        worktrees
    }

    /// Prune stale worktree references
    ///
    /// Runs `git worktree prune` to clean up stale worktree metadata entries
    /// that reference directories that no longer exist on disk.
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub async fn prune_worktrees(repo: &Path) -> AppResult<()> {
        debug!("Pruning stale worktrees in {:?}", repo);

        let output = git_cmd::run(&["worktree", "prune"], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to prune worktrees in {:?}: {}", repo, stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to prune worktrees in '{}': {}",
                repo.to_string_lossy(),
                stderr.trim()
            )));
        }

        Ok(())
    }
}
