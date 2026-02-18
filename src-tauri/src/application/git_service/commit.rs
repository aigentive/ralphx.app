use super::git_cmd;
use super::*;

impl GitService {
    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Stage all changes and create a commit
    ///
    /// Returns the commit SHA if a commit was created, None if there was nothing to commit.
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    /// * `message` - Commit message
    pub async fn commit_all(path: &Path, message: &str) -> AppResult<Option<String>> {
        debug!(
            "Committing all changes in {:?} with message: {}",
            path, message
        );

        // Stage all changes
        // git add -A respects .gitignore, so node_modules and src-tauri/target
        // are automatically excluded without needing pathspec guards.
        let add_output = git_cmd::run(&["add", "-A"], path).await?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to stage changes: {}",
                stderr
            )));
        }

        // Check if there's anything to commit
        if !Self::has_staged_changes(path).await? {
            debug!("No changes to commit");
            return Ok(None);
        }

        // Create the commit
        let commit_output = git_cmd::run(&["commit", "-m", message], path).await?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to commit: {}",
                stderr
            )));
        }

        // Get the commit SHA
        let sha = Self::get_head_sha(path).await?;
        Ok(Some(sha))
    }

    /// Check if there are uncommitted changes in the working directory
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    pub async fn has_uncommitted_changes(path: &Path) -> AppResult<bool> {
        let output = git_cmd::run(&["status", "--porcelain"], path).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to check status: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    /// Check if there are staged changes ready to commit
    async fn has_staged_changes(path: &Path) -> AppResult<bool> {
        let output = git_cmd::run(&["diff", "--cached", "--quiet"], path).await?;

        // Exit code 1 means there are differences (staged changes)
        Ok(!output.status.success())
    }

    /// Get the SHA of HEAD
    pub async fn get_head_sha(path: &Path) -> AppResult<String> {
        let output = git_cmd::run(&["rev-parse", "HEAD"], path).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get HEAD SHA: {}",
                stderr
            )));
        }

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(sha)
    }

    /// Get the SHA of a specific branch tip (without checking it out).
    pub async fn get_branch_sha(repo: &Path, branch: &str) -> AppResult<String> {
        let output = git_cmd::run(&["rev-parse", branch], repo).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get SHA for branch {}: {}",
                branch, stderr
            )));
        }

        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(sha)
    }
}
