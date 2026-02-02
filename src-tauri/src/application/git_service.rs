//! Git Service - Branch, worktree, and merge operations for task isolation
//!
//! Provides git operations for per-task branch isolation:
//! - Branch creation, checkout, and deletion (both modes)
//! - Worktree management (Worktree mode only)
//! - Commit operations with configurable messages
//! - Rebase and merge operations for the two-phase merge workflow
//! - Query operations for commits and diff stats

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, warn};

/// Result of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MergeResult {
    /// Merge succeeded with a new merge commit
    Success { commit_sha: String },
    /// Merge resulted in conflicts
    Conflict { files: Vec<PathBuf> },
    /// Fast-forward merge (no merge commit needed)
    FastForward { commit_sha: String },
}

/// Result of a rebase operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RebaseResult {
    /// Rebase completed successfully
    Success,
    /// Rebase resulted in conflicts
    Conflict { files: Vec<PathBuf> },
}

/// Result of the combined rebase + merge attempt (Phase 1 of merge workflow)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MergeAttemptResult {
    /// Rebase + merge succeeded (fast path)
    Success { commit_sha: String },
    /// Conflict detected, needs agent resolution (Phase 2)
    NeedsAgent { conflict_files: Vec<PathBuf> },
}

/// Information about a single commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit SHA (full 40 characters)
    pub sha: String,
    /// Short SHA (7 characters)
    pub short_sha: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author: String,
    /// Commit timestamp (RFC3339)
    pub timestamp: String,
}

/// Statistics about changes between branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    /// Number of files changed
    pub files_changed: u32,
    /// Number of lines added
    pub insertions: u32,
    /// Number of lines deleted
    pub deletions: u32,
    /// List of changed file paths
    pub changed_files: Vec<String>,
}

/// Git Service for branch, worktree, and merge operations
pub struct GitService;

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
    pub fn create_branch(repo: &Path, branch: &str, base: &str) -> AppResult<()> {
        debug!(
            "Creating branch '{}' from '{}' in {:?}",
            branch, base, repo
        );

        let output = Command::new("git")
            .args(["branch", branch, base])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git branch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to create branch '{}': {}",
                branch, stderr
            )));
        }

        Ok(())
    }

    /// Checkout an existing branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `branch` - Name of the branch to checkout
    pub fn checkout_branch(repo: &Path, branch: &str) -> AppResult<()> {
        debug!("Checking out branch '{}' in {:?}", branch, repo);

        let output = Command::new("git")
            .args(["checkout", branch])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git checkout: {}", e)))?;

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
    pub fn delete_branch(repo: &Path, branch: &str, force: bool) -> AppResult<()> {
        debug!(
            "Deleting branch '{}' (force={}) in {:?}",
            branch, force, repo
        );

        let flag = if force { "-D" } else { "-d" };
        let output = Command::new("git")
            .args(["branch", flag, branch])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git branch -d: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to delete branch '{}': {}",
                branch, stderr
            )));
        }

        Ok(())
    }

    /// Get the current branch name
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub fn get_current_branch(repo: &Path) -> AppResult<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rev-parse: {}", e))
            })?;

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
    pub fn create_worktree(
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

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                branch,
                worktree.to_str().unwrap_or_default(),
                base,
            ])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git worktree add: {}", e))
            })?;

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
    pub fn delete_worktree(repo: &Path, worktree: &Path) -> AppResult<()> {
        debug!("Deleting worktree at {:?} from {:?}", worktree, repo);

        let output = Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                worktree.to_str().unwrap_or_default(),
            ])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git worktree remove: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Log warning but don't fail - worktree might not exist or be already removed
            warn!(
                "Failed to delete worktree at {:?}: {}",
                worktree, stderr
            );
        }

        Ok(())
    }

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
        debug!("Committing all changes in {:?} with message: {}", path, message);

        // Stage all changes
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

    // =========================================================================
    // Rebase Operations (Phase 1 - fast path)
    // =========================================================================

    /// Fetch from origin (if remote exists)
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub fn fetch_origin(repo: &Path) -> AppResult<()> {
        debug!("Fetching from origin in {:?}", repo);

        // Check if origin exists first
        let remote_check = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(repo)
            .output();

        if let Ok(output) = remote_check {
            if !output.status.success() {
                debug!("No origin remote configured, skipping fetch");
                return Ok(());
            }
        }

        let output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git fetch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git fetch failed (non-fatal): {}", stderr);
        }

        Ok(())
    }

    /// Rebase current branch onto a base branch
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    /// * `base` - Name of the base branch to rebase onto
    pub fn rebase_onto(path: &Path, base: &str) -> AppResult<RebaseResult> {
        debug!("Rebasing onto '{}' in {:?}", base, path);

        let output = Command::new("git")
            .args(["rebase", base])
            .current_dir(path)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git rebase: {}", e)))?;

        if output.status.success() {
            return Ok(RebaseResult::Success);
        }

        // Check if it's a conflict
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("CONFLICT") || stderr.contains("conflict") {
            let conflict_files = Self::get_conflict_files(path)?;
            return Ok(RebaseResult::Conflict {
                files: conflict_files,
            });
        }

        Err(AppError::GitOperation(format!(
            "Rebase failed: {}",
            stderr
        )))
    }

    /// Abort an in-progress rebase
    ///
    /// # Arguments
    /// * `path` - Path to the git repository or worktree
    pub fn abort_rebase(path: &Path) -> AppResult<()> {
        debug!("Aborting rebase in {:?}", path);

        let output = Command::new("git")
            .args(["rebase", "--abort"])
            .current_dir(path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rebase --abort: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't error if no rebase in progress
            if !stderr.contains("No rebase in progress") {
                return Err(AppError::GitOperation(format!(
                    "Failed to abort rebase: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    // =========================================================================
    // Merge Operations
    // =========================================================================

    /// Merge a source branch into the current branch
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    /// * `source` - Name of the branch to merge from
    /// * `_target` - Name of the target branch (unused, we merge into current HEAD)
    pub fn merge_branch(repo: &Path, source: &str, _target: &str) -> AppResult<MergeResult> {
        debug!("Merging branch '{}' in {:?}", source, repo);

        let output = Command::new("git")
            .args(["merge", source, "--no-edit"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git merge: {}", e)))?;

        if output.status.success() {
            let sha = Self::get_head_sha(repo)?;
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Check if it was a fast-forward
            if stdout.contains("Fast-forward") {
                return Ok(MergeResult::FastForward { commit_sha: sha });
            }

            return Ok(MergeResult::Success { commit_sha: sha });
        }

        // Check if it's a conflict
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("CONFLICT") || stderr.contains("conflict") {
            let conflict_files = Self::get_conflict_files(repo)?;
            return Ok(MergeResult::Conflict {
                files: conflict_files,
            });
        }

        Err(AppError::GitOperation(format!(
            "Merge failed: {}",
            stderr
        )))
    }

    /// Abort an in-progress merge
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub fn abort_merge(repo: &Path) -> AppResult<()> {
        debug!("Aborting merge in {:?}", repo);

        let output = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git merge --abort: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't error if no merge in progress
            if !stderr.contains("There is no merge to abort") {
                return Err(AppError::GitOperation(format!(
                    "Failed to abort merge: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    /// Get list of files with conflicts
    ///
    /// # Arguments
    /// * `repo` - Path to the git repository
    pub fn get_conflict_files(repo: &Path) -> AppResult<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git diff: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<PathBuf> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(files)
    }

    // =========================================================================
    // Merge State Detection (Phase 76 - Auto-completion)
    // =========================================================================

    /// Check if a rebase is currently in progress
    ///
    /// Detects incomplete rebase by checking for `.git/rebase-merge` or `.git/rebase-apply`
    /// directories which exist while a rebase is paused (e.g., due to conflicts).
    ///
    /// # Arguments
    /// * `worktree` - Path to the git worktree or repository
    pub fn is_rebase_in_progress(worktree: &Path) -> bool {
        // For worktrees, .git is a file pointing to the main repo's .git/worktrees/<name>
        // We need to resolve the actual git directory
        let git_path = worktree.join(".git");

        let git_dir = if git_path.is_file() {
            // Read the gitdir from the .git file
            if let Ok(content) = std::fs::read_to_string(&git_path) {
                if let Some(path) = content.strip_prefix("gitdir: ") {
                    PathBuf::from(path.trim())
                } else {
                    git_path
                }
            } else {
                git_path
            }
        } else {
            git_path
        };

        git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists()
    }

    /// Check for conflict markers in tracked files
    ///
    /// Scans all tracked files for the standard git conflict marker `<<<<<<<`.
    /// Returns true if any conflict markers are found.
    ///
    /// # Arguments
    /// * `worktree` - Path to the git worktree or repository
    pub fn has_conflict_markers(worktree: &Path) -> AppResult<bool> {
        // Get list of tracked files
        let output = Command::new("git")
            .args(["ls-files"])
            .current_dir(worktree)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git ls-files: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitOperation(format!(
                "Failed to list tracked files: {}",
                stderr
            )));
        }

        let tracked_files = String::from_utf8_lossy(&output.stdout);

        // Check each tracked file for conflict markers
        for file in tracked_files.lines() {
            if file.is_empty() {
                continue;
            }

            let file_path = worktree.join(file);

            // Skip if file doesn't exist (could be deleted in working tree)
            if !file_path.exists() {
                continue;
            }

            // Skip binary files - only check text files
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if content.contains("<<<<<<<") {
                    debug!("Found conflict marker in file: {}", file);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Attempt to rebase and merge (Phase 1 of merge workflow)
    ///
    /// This is the "fast path" - tries to do a programmatic rebase + merge.
    /// If it succeeds, we skip the agent entirely.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `task_branch` - Name of the task branch to merge
    /// * `base` - Name of the base branch to merge into
    pub fn try_rebase_and_merge(
        repo: &Path,
        task_branch: &str,
        base: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting rebase and merge of '{}' onto '{}' in {:?}",
            task_branch, base, repo
        );

        // Step 1: Fetch latest from origin (non-fatal if fails)
        let _ = Self::fetch_origin(repo);

        // Step 2: Checkout task branch and rebase onto base
        Self::checkout_branch(repo, task_branch)?;

        match Self::rebase_onto(repo, base)? {
            RebaseResult::Success => {
                // Step 3: Checkout base and merge task branch (should be fast-forward)
                Self::checkout_branch(repo, base)?;

                match Self::merge_branch(repo, task_branch, base)? {
                    MergeResult::Success { commit_sha } | MergeResult::FastForward { commit_sha } => {
                        Ok(MergeAttemptResult::Success { commit_sha })
                    }
                    MergeResult::Conflict { files } => {
                        // This shouldn't happen after successful rebase, but handle it
                        Self::abort_merge(repo)?;
                        Ok(MergeAttemptResult::NeedsAgent {
                            conflict_files: files,
                        })
                    }
                }
            }
            RebaseResult::Conflict { files } => {
                // Abort the rebase and let agent handle it
                Self::abort_rebase(repo)?;
                // Checkout back to base to leave repo in clean state
                Self::checkout_branch(repo, base)?;
                Ok(MergeAttemptResult::NeedsAgent {
                    conflict_files: files,
                })
            }
        }
    }

    // =========================================================================
    // Query Operations
    // =========================================================================

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
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git rev-parse: {}", e))
            })?;

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

        let output = Command::new("git")
            .args([
                "log",
                &range,
                "--pretty=format:%H|%h|%s|%an|%aI",
            ])
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
    fn parse_shortstat(output: &str) -> (u32, u32, u32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortstat_full() {
        let output = " 3 files changed, 50 insertions(+), 10 deletions(-)";
        let (files, insertions, deletions) = GitService::parse_shortstat(output);
        assert_eq!(files, 3);
        assert_eq!(insertions, 50);
        assert_eq!(deletions, 10);
    }

    #[test]
    fn test_parse_shortstat_insertions_only() {
        let output = " 1 file changed, 25 insertions(+)";
        let (files, insertions, deletions) = GitService::parse_shortstat(output);
        assert_eq!(files, 1);
        assert_eq!(insertions, 25);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn test_parse_shortstat_deletions_only() {
        let output = " 2 files changed, 15 deletions(-)";
        let (files, insertions, deletions) = GitService::parse_shortstat(output);
        assert_eq!(files, 2);
        assert_eq!(insertions, 0);
        assert_eq!(deletions, 15);
    }

    #[test]
    fn test_parse_shortstat_empty() {
        let output = "";
        let (files, insertions, deletions) = GitService::parse_shortstat(output);
        assert_eq!(files, 0);
        assert_eq!(insertions, 0);
        assert_eq!(deletions, 0);
    }

    // =========================================================================
    // Merge State Detection Tests (Phase 76)
    // =========================================================================

    #[test]
    fn test_is_rebase_in_progress_no_rebase() {
        // Use a temp directory without rebase state
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        assert!(!GitService::is_rebase_in_progress(temp_dir.path()));
    }

    #[test]
    fn test_is_rebase_in_progress_with_rebase_merge() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        // Simulate rebase-merge directory (interactive rebase in progress)
        std::fs::create_dir(git_dir.join("rebase-merge")).unwrap();

        assert!(GitService::is_rebase_in_progress(temp_dir.path()));
    }

    #[test]
    fn test_is_rebase_in_progress_with_rebase_apply() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        // Simulate rebase-apply directory (git am or older rebase in progress)
        std::fs::create_dir(git_dir.join("rebase-apply")).unwrap();

        assert!(GitService::is_rebase_in_progress(temp_dir.path()));
    }

    #[test]
    fn test_is_rebase_in_progress_worktree_style() {
        // Test worktree-style .git file pointing to gitdir
        let temp_dir = tempfile::tempdir().unwrap();
        let git_path = temp_dir.path().join(".git");

        // Create the actual git directory somewhere else
        let actual_git_dir = temp_dir.path().join("actual_git_dir");
        std::fs::create_dir(&actual_git_dir).unwrap();

        // Create .git file pointing to actual git dir
        std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

        // No rebase in progress
        assert!(!GitService::is_rebase_in_progress(temp_dir.path()));

        // Add rebase-merge to actual git dir
        std::fs::create_dir(actual_git_dir.join("rebase-merge")).unwrap();

        assert!(GitService::is_rebase_in_progress(temp_dir.path()));
    }
}
