use super::git_cmd;
use super::*;

impl GitService {
    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Stage modified/new files (excluding deletions) and create a commit.
    ///
    /// SAFETY: This intentionally does NOT stage file deletions. Using `git add -A`
    /// in worktrees would stage deletions for every file absent from the worktree
    /// but present in the repo, causing catastrophic data loss on auto-commit.
    ///
    /// For merge conflict resolution where deletions are intentional, use
    /// `commit_all_including_deletions` instead.
    ///
    /// # Errors
    /// Returns `AppError::GitOperation` if git commands fail.
    pub async fn commit_all(path: &Path, message: &str) -> AppResult<Option<String>> {
        debug!(
            "Committing all changes in {:?} with message: {}",
            path, message
        );

        Self::stage_non_deletion_changes(path).await?;

        Self::commit_staged(path, message).await
    }

    /// Stage ALL changes including deletions and create a commit.
    ///
    /// Only use this for merge conflict resolution where the user has intentionally
    /// resolved conflicts (which may include file deletions).
    ///
    /// # Errors
    /// Returns `AppError::GitOperation` if git commands fail.
    pub async fn commit_all_including_deletions(
        path: &Path,
        message: &str,
    ) -> AppResult<Option<String>> {
        debug!(
            "Committing all changes (including deletions) in {:?} with message: {}",
            path, message
        );

        // Use git status --porcelain -z for safe, .gitignore-respecting staging
        // (instead of `git add -A` which can stage build artifacts)
        let status_output = git_cmd::run(&["status", "--porcelain", "-z"], path).await?;
        if !status_output.status.success() {
            let stderr = String::from_utf8_lossy(&status_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get git status: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&status_output.stdout);
        let mut files_to_stage: Vec<String> = Vec::new();

        // Format: "XY filename\0" — renames have two entries: "R  newname\0oldname\0"
        let entries: Vec<&str> = stdout.split('\0').collect();
        let mut i = 0;
        while i < entries.len() {
            let entry = entries[i];
            if entry.is_empty() {
                i += 1;
                continue;
            }

            let status_code = entry.get(..2).unwrap_or("");
            let filename = entry.get(3..).unwrap_or("");

            // Renames (R) and copies (C) have a second NUL-separated entry (old name)
            if status_code.starts_with('R') || status_code.starts_with('C') {
                files_to_stage.push(filename.to_string());
                i += 2; // skip the old-name entry
                continue;
            }

            files_to_stage.push(filename.to_string());
            i += 1;
        }

        // Batch git add in chunks of 100
        for chunk in files_to_stage.chunks(100) {
            let mut args: Vec<&str> = vec!["add", "--"];
            args.extend(chunk.iter().map(|s| s.as_str()));
            let add_output = git_cmd::run(&args, path).await?;
            if !add_output.status.success() {
                let stderr = String::from_utf8_lossy(&add_output.stderr);
                return Err(AppError::GitOperation(format!(
                    "Failed to stage batch: {}",
                    stderr
                )));
            }
        }

        Self::commit_staged(path, message).await
    }

    /// Stage modified and new files, skipping deletions.
    ///
    /// Uses `git status --porcelain -z` for NUL-separated output that handles
    /// filenames with spaces, quotes, and special characters without quoting.
    async fn stage_non_deletion_changes(path: &Path) -> AppResult<()> {
        // -z: NUL-separated, no quoting — safe for all filenames
        let status_output = git_cmd::run(&["status", "--porcelain", "-z"], path).await?;
        if !status_output.status.success() {
            let stderr = String::from_utf8_lossy(&status_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get git status: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&status_output.stdout);
        let mut files_to_stage: Vec<String> = Vec::new();
        let mut skipped_deletions: Vec<String> = Vec::new();

        // Format: "XY filename\0" — renames have two entries: "R  newname\0oldname\0"
        let entries: Vec<&str> = stdout.split('\0').collect();
        let mut i = 0;
        while i < entries.len() {
            let entry = entries[i];
            if entry.is_empty() {
                i += 1;
                continue;
            }

            let status_code = entry.get(..2).unwrap_or("");
            let filename = entry.get(3..).unwrap_or("");

            if status_code.contains('D') {
                skipped_deletions.push(filename.to_string());
                i += 1;
                continue;
            }

            // Renames (R) and copies (C) have a second NUL-separated entry (old name)
            if status_code.starts_with('R') || status_code.starts_with('C') {
                files_to_stage.push(filename.to_string());
                i += 2; // skip the old-name entry
                continue;
            }

            files_to_stage.push(filename.to_string());
            i += 1;
        }

        if !skipped_deletions.is_empty() {
            tracing::warn!(
                count = skipped_deletions.len(),
                files = ?skipped_deletions,
                "Skipped staging {} deleted file(s) in auto-commit (safety: prevents worktree deletion propagation)",
                skipped_deletions.len()
            );
        }

        for file in &files_to_stage {
            let add_output = git_cmd::run(&["add", "--", file], path).await?;
            if !add_output.status.success() {
                let stderr = String::from_utf8_lossy(&add_output.stderr);
                tracing::warn!("Failed to stage file {}: {}", file, stderr);
            }
        }

        Ok(())
    }

    /// Commit whatever is currently staged, returning the SHA or None if nothing staged.
    async fn commit_staged(path: &Path, message: &str) -> AppResult<Option<String>> {
        if !Self::has_staged_changes(path).await? {
            debug!("No changes to commit");
            return Ok(None);
        }

        let commit_output = git_cmd::run(&["commit", "-m", message], path).await?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to commit: {}",
                stderr
            )));
        }

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
