use super::*;

impl GitService {
    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Get the number of commits on a branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `branch` - Name of the branch to count commits on
    ///
    /// # Returns
    /// The number of commits on the branch
    pub fn get_commit_count(repo: &Path, branch: &str) -> AppResult<u32> {
        debug!("Getting commit count for branch '{}' in {:?}", branch, repo);

        let output = Command::new("git")
            .args(["rev-list", "--count", branch])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rev-list --count: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to count commits on '{}': {}",
                branch, stderr
            )));
        }

        let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let count = count_str.parse::<u32>().map_err(|e| {
            AppError::GitOperation(format!(
                "Failed to parse commit count '{}': {}",
                count_str, e
            ))
        })?;

        Ok(count)
    }

    /// Check if a commit is an ancestor of (or equal to) a branch head
    ///
    /// Uses `git merge-base --is-ancestor` to verify that commit_sha is reachable
    /// from the specified branch. This is useful for verifying that a merge
    /// actually happened on a target branch.
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `commit_sha` - SHA of the commit to check
    /// * `branch` - Name of the branch to check against
    ///
    /// # Returns
    /// * `Ok(true)` if commit is on the branch (is an ancestor or equal)
    /// * `Ok(false)` if commit is not on the branch
    /// * `Err` if git command fails
    pub fn is_commit_on_branch(repo: &Path, commit_sha: &str, branch: &str) -> AppResult<bool> {
        debug!(
            "Checking if commit {} is on branch '{}' in {:?}",
            commit_sha, branch, repo
        );

        let output = Command::new("git")
            .args(["merge-base", "--is-ancestor", commit_sha, branch])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git merge-base: {}", e)))?;

        // Exit code 0 = commit is ancestor (on branch), 1 = not ancestor (not on branch)
        // Other exit codes indicate errors
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            Some(code) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(AppError::GitOperation(format!(
                    "git merge-base failed with exit code {}: {}",
                    code, stderr
                )))
            }
            None => Err(AppError::GitOperation(
                "git merge-base was terminated by signal".to_string(),
            )),
        }
    }

    /// Get commits on the current branch since it diverged from base
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    /// * `base` - Name of the base branch
    pub fn get_commits_since(path: &Path, base: &str) -> AppResult<Vec<CommitInfo>> {
        let output = Command::new("git")
            .args([
                "log",
                &format!("{}..HEAD", base),
                "--pretty=format:%H|%h|%s|%an|%aI",
            ])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git log: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get commits: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits: Vec<CommitInfo> = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 5 {
                    Some(CommitInfo {
                        sha: parts[0].to_string(),
                        short_sha: parts[1].to_string(),
                        message: parts[2].to_string(),
                        author: parts[3].to_string(),
                        timestamp: parts[4].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(commits)
    }

    /// Get commits that were part of a merged task
    ///
    /// After a task is merged, the task branch/worktree is deleted, so we can't use
    /// `base..HEAD` from the working path. Instead, we query commits between the
    /// merge-base and the merge commit SHA.
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `base_branch` - Name of the base branch
    /// * `merge_commit_sha` - SHA of the merge commit
    pub fn get_merged_task_commits(
        repo: &Path,
        base_branch: &str,
        merge_commit_sha: &str,
    ) -> AppResult<Vec<CommitInfo>> {
        // Get the merge-base (where the task branch diverged from base)
        // We need the first parent of the merge commit, which is the base branch
        // Then get commits from merge-base to the merge commit's second parent (the task branch)
        //
        // For a merge commit:
        // - First parent (^1) is the base branch commit
        // - Second parent (^2) is the task branch tip
        //
        // We want commits that were on the task branch: merge_commit^1..merge_commit^2
        // But if it was a fast-forward, there's no second parent, so we use merge_commit^1..merge_commit

        // First, check if merge commit has a second parent (true merge vs fast-forward)
        let check_output = Command::new("git")
            .args(["rev-parse", "--verify", &format!("{}^2", merge_commit_sha)])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git rev-parse: {}", e)))?;

        let range = if check_output.status.success() {
            // True merge - get commits from first parent to second parent
            format!("{}^1..{}^2", merge_commit_sha, merge_commit_sha)
        } else {
            // Fast-forward - get commits from before merge to merge commit
            // Find merge-base between base branch and merge commit
            let merge_base_output = Command::new("git")
                .args(["merge-base", base_branch, merge_commit_sha])
                .current_dir(repo)
                .output()
                .map_err(|e| {
                    AppError::GitOperation(format!("Failed to run git merge-base: {}", e))
                })?;

            if !merge_base_output.status.success() {
                // If we can't find merge-base, just use base_branch..merge_commit
                format!("{}..{}", base_branch, merge_commit_sha)
            } else {
                let merge_base = String::from_utf8_lossy(&merge_base_output.stdout)
                    .trim()
                    .to_string();
                format!("{}..{}", merge_base, merge_commit_sha)
            }
        };

        let mut output = Command::new("git")
            .args(["log", &range, "--pretty=format:%H|%h|%s|%an|%aI"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git log: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to get merged commits: {}",
                stderr
            )));
        }

        if output.stdout.is_empty() {
            // Fallback: include the merge commit itself (e.g., manual resolution commit on base)
            let single_range = format!("{}^..{}", merge_commit_sha, merge_commit_sha);
            output = Command::new("git")
                .args(["log", &single_range, "--pretty=format:%H|%h|%s|%an|%aI"])
                .current_dir(repo)
                .output()
                .map_err(|e| AppError::GitOperation(format!("Failed to run git log: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(AppError::GitOperation(format!(
                    "Failed to get merged commits: {}",
                    stderr
                )));
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits: Vec<CommitInfo> = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 5 {
                    Some(CommitInfo {
                        sha: parts[0].to_string(),
                        short_sha: parts[1].to_string(),
                        message: parts[2].to_string(),
                        author: parts[3].to_string(),
                        timestamp: parts[4].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(commits)
    }

    /// Get diff statistics between current branch and base
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    /// * `base` - Name of the base branch
    pub fn get_diff_stats(path: &Path, base: &str) -> AppResult<DiffStats> {
        // Get shortstat for summary
        let stat_output = Command::new("git")
            .args(["diff", "--shortstat", base])
            .current_dir(path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git diff --shortstat: {}", e))
            })?;

        let stat_stdout = String::from_utf8_lossy(&stat_output.stdout);

        // Parse "N files changed, M insertions(+), K deletions(-)"
        let (files_changed, insertions, deletions) = Self::parse_shortstat(&stat_stdout);

        // Get list of changed files
        let name_output = Command::new("git")
            .args(["diff", "--name-only", base])
            .current_dir(path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git diff --name-only: {}", e))
            })?;

        let name_stdout = String::from_utf8_lossy(&name_output.stdout);
        let changed_files: Vec<String> = name_stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect();

        Ok(DiffStats {
            files_changed,
            insertions,
            deletions,
            changed_files,
        })
    }

    /// Parse git shortstat output
    pub(super) fn parse_shortstat(output: &str) -> (u32, u32, u32) {
        let mut files = 0u32;
        let mut insertions = 0u32;
        let mut deletions = 0u32;

        // Format: " N files changed, M insertions(+), K deletions(-)"
        for part in output.split(',') {
            let part = part.trim();
            if part.contains("file") {
                if let Some(num_str) = part.split_whitespace().next() {
                    files = num_str.parse().unwrap_or(0);
                }
            } else if part.contains("insertion") {
                if let Some(num_str) = part.split_whitespace().next() {
                    insertions = num_str.parse().unwrap_or(0);
                }
            } else if part.contains("deletion") {
                if let Some(num_str) = part.split_whitespace().next() {
                    deletions = num_str.parse().unwrap_or(0);
                }
            }
        }

        (files, insertions, deletions)
    }
}
