use super::git_cmd;
use super::*;

impl GitService {
    // =========================================================================
    // Branch Operations (both modes)
    // =========================================================================

    /// Create a new branch from a base branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `branch` - Name of the new branch to create
    /// * `base` - Name of the base branch to branch from
    pub async fn create_branch(repo: &Path, branch: &str, base: &str) -> AppResult<()> {
        debug!("Creating branch '{}' from '{}' in {:?}", branch, base, repo);

        let output = git_cmd::run(&["branch", branch, base], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create branch '{}': {}",
                branch, stderr
            )));
        }

        Ok(())
    }

    /// Create a new branch pointing at a specific commit SHA
    ///
    /// Unlike `create_branch` which branches from another branch name,
    /// this creates a branch at an exact commit. Used for conflict resolution
    /// worktrees after checkout-free merge detects conflicts.
    pub async fn create_branch_at(repo: &Path, branch: &str, sha: &str) -> AppResult<()> {
        debug!("Creating branch '{}' at {} in {:?}", branch, sha, repo);

        let output = git_cmd::run(&["branch", branch, sha], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create branch '{}' at {}: {}",
                branch, sha, stderr
            )));
        }

        Ok(())
    }

    /// Sync working tree to match the current branch HEAD
    ///
    /// Runs `git reset --hard HEAD` to atomically update all files.
    /// Used after checkout-free merge operations that advance the branch ref
    /// without touching the working tree.
    pub async fn hard_reset_to_head(repo: &Path) -> AppResult<()> {
        debug!("Resetting working tree to HEAD in {:?}", repo);

        let output = git_cmd::run(&["reset", "--hard", "HEAD"], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "git reset --hard HEAD failed: {}",
                stderr
            )));
        }

        debug!("Working tree synced to HEAD");
        Ok(())
    }

    /// Ensure a clean working tree by resetting tracked files and removing untracked files/dirs.
    ///
    /// This method:
    /// 1. Checks `git status --porcelain` — early return if already clean
    /// 2. Runs `hard_reset_to_head()` to reset tracked files to HEAD state
    /// 3. Runs `git clean -fd` to remove untracked files and directories
    ///    (not `-fdx` — preserves .gitignore'd files like node_modules and src-tauri/target)
    /// 4. Logs dirty entries before cleaning (capped at 20 lines)
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub async fn clean_working_tree(repo: &Path) -> AppResult<()> {
        debug!("Cleaning working tree in {:?}", repo);

        // Check if already clean (early return optimization)
        let status_output = git_cmd::run(&["status", "--porcelain"], repo).await?;

        if !status_output.status.success() {
            let stderr = String::from_utf8_lossy(&status_output.stderr);
            return Err(AppError::GitOperation(format!(
                "git status --porcelain failed: {}",
                stderr
            )));
        }

        let status_output_str = String::from_utf8_lossy(&status_output.stdout);

        // Early return if working tree is already clean
        if status_output_str.trim().is_empty() {
            debug!("Working tree is already clean");
            return Ok(());
        }

        // Log dirty entries (capped at 20 lines)
        let dirty_entries: Vec<&str> = status_output_str.lines().collect();
        let to_log: Vec<&&str> = if dirty_entries.len() > 20 {
            dirty_entries.iter().take(20).collect()
        } else {
            dirty_entries.iter().collect()
        };

        for entry in to_log {
            warn!("Dirty entry: {}", entry);
        }

        if dirty_entries.len() > 20 {
            warn!(
                "... and {} more dirty entries (total: {})",
                dirty_entries.len() - 20,
                dirty_entries.len()
            );
        }

        // Reset tracked files to HEAD state
        Self::hard_reset_to_head(repo).await?;

        // Remove untracked files and directories
        // Using -fd (not -fdx) to preserve .gitignore'd files
        let clean_output = git_cmd::run(&["clean", "-fd"], repo).await?;

        if !clean_output.status.success() {
            let stderr = String::from_utf8_lossy(&clean_output.stderr);
            return Err(AppError::GitOperation(format!(
                "git clean -fd failed: {}",
                stderr
            )));
        }

        debug!("Working tree cleaned successfully");
        Ok(())
    }

    /// Checkout an existing branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `branch` - Name of the branch to checkout
    pub async fn checkout_branch(repo: &Path, branch: &str) -> AppResult<()> {
        debug!("Checking out branch '{}' in {:?}", branch, repo);

        let output = git_cmd::run(&["checkout", branch], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to checkout branch '{}': {}",
                branch, stderr
            )));
        }

        Ok(())
    }

    /// Delete a branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `branch` - Name of the branch to delete
    /// * `force` - If true, use -D (force delete), otherwise -d (safe delete)
    pub async fn delete_branch(repo: &Path, branch: &str, force: bool) -> AppResult<()> {
        debug!(
            "Deleting branch '{}' (force={}) in {:?}",
            branch, force, repo
        );

        let flag = if force { "-D" } else { "-d" };
        let output = git_cmd::run(&["branch", flag, branch], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to delete branch '{}': {}",
                branch, stderr
            )));
        }

        Ok(())
    }

    /// Create a feature branch without checking it out.
    /// Used for plan group feature branches that isolate plan work.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `branch_name` - Name of the feature branch (e.g., "ralphx/my-app/plan-abc123")
    /// * `source_branch` - Branch to create from (e.g., "main")
    pub async fn create_feature_branch(
        repo_path: &Path,
        branch_name: &str,
        source_branch: &str,
    ) -> AppResult<()> {
        debug!(
            "Creating feature branch '{}' from '{}' in {:?}",
            branch_name, source_branch, repo_path
        );

        let output = git_cmd::run(&["branch", branch_name, source_branch], repo_path).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create feature branch '{}': {}",
                branch_name, stderr
            )));
        }

        debug!("Feature branch '{}' created successfully", branch_name);
        Ok(())
    }

    /// Delete a feature branch after it has been merged.
    /// Uses safe delete (-d) to prevent deleting unmerged branches.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `branch_name` - Name of the feature branch to delete
    pub async fn delete_feature_branch(repo_path: &Path, branch_name: &str) -> AppResult<()> {
        debug!(
            "Deleting feature branch '{}' in {:?}",
            branch_name, repo_path
        );

        let output = git_cmd::run(&["branch", "-d", branch_name], repo_path).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to delete feature branch '{}': {}",
                branch_name, stderr
            )));
        }

        debug!("Feature branch '{}' deleted successfully", branch_name);
        Ok(())
    }

    /// Get the current branch name
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub async fn get_current_branch(repo: &Path) -> AppResult<String> {
        let output = git_cmd::run(&["rev-parse", "--abbrev-ref", "HEAD"], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get current branch: {}",
                stderr
            )));
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(branch)
    }

    /// Check if a branch (local or remote-tracking) exists in the repo.
    pub async fn branch_exists(repo: &Path, branch: &str) -> bool {
        git_cmd::run_status(&["rev-parse", "--verify", branch], repo)
            .await
            .unwrap_or(false)
    }

    /// Validate that both source and target branches exist before merge.
    /// Returns `Some(BranchNotFound)` if either is missing, `None` if both exist.
    pub(super) async fn validate_merge_branches(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
    ) -> Option<MergeAttemptResult> {
        if !Self::branch_exists(repo, source_branch).await {
            warn!("Source branch '{}' does not exist", source_branch);
            return Some(MergeAttemptResult::BranchNotFound {
                branch: source_branch.to_string(),
            });
        }
        if !Self::branch_exists(repo, target_branch).await {
            warn!("Target branch '{}' does not exist", target_branch);
            return Some(MergeAttemptResult::BranchNotFound {
                branch: target_branch.to_string(),
            });
        }
        None
    }

    /// Check if two branches have identical content (tree-level diff).
    ///
    /// Uses `git diff --quiet` which exits 0 if identical, 1 if different.
    pub async fn branches_have_same_content(
        repo: &Path,
        branch_a: &str,
        branch_b: &str,
    ) -> AppResult<bool> {
        let output = git_cmd::run(&["diff", "--quiet", branch_a, branch_b], repo).await?;
        Ok(output.status.success())
    }
}
