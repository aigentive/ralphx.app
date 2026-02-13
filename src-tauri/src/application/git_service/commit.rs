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
    pub fn commit_all(path: &Path, message: &str) -> AppResult<Option<String>> {
        debug!(
            "Committing all changes in {:?} with message: {}",
            path, message
        );

        // Stage all changes
        // git add -A respects .gitignore, so node_modules and src-tauri/target
        // are automatically excluded without needing pathspec guards.
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git add: {}", e)))?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to stage changes: {}",
                stderr
            )));
        }

        // Check if there's anything to commit
        if !Self::has_staged_changes(path)? {
            debug!("No changes to commit");
            return Ok(None);
        }

        // Create the commit
        let commit_output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git commit: {}", e)))?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to commit: {}",
                stderr
            )));
        }

        // Get the commit SHA
        let sha = Self::get_head_sha(path)?;
        Ok(Some(sha))
    }

    /// Check if there are uncommitted changes in the working directory
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    pub fn has_uncommitted_changes(path: &Path) -> AppResult<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git status: {}", e)))?;

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
    fn has_staged_changes(path: &Path) -> AppResult<bool> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git diff: {}", e)))?;

        // Exit code 1 means there are differences (staged changes)
        Ok(!output.status.success())
    }

    /// Get the SHA of HEAD
    pub fn get_head_sha(path: &Path) -> AppResult<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rev-parse HEAD: {}", e))
            })?;

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
    pub fn get_branch_sha(repo: &Path, branch: &str) -> AppResult<String> {
        let output = Command::new("git")
            .args(["rev-parse", branch])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rev-parse {}: {}", branch, e))
            })?;

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
