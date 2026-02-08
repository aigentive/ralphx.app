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

    /// Create a feature branch without checking it out.
    /// Used for plan group feature branches that isolate plan work.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the git repository
    /// * `branch_name` - Name of the feature branch (e.g., "ralphx/my-app/plan-abc123")
    /// * `source_branch` - Branch to create from (e.g., "main")
    pub fn create_feature_branch(
        repo_path: &Path,
        branch_name: &str,
        source_branch: &str,
    ) -> AppResult<()> {
        debug!(
            "Creating feature branch '{}' from '{}' in {:?}",
            branch_name, source_branch, repo_path
        );

        let output = Command::new("git")
            .args(["branch", branch_name, source_branch])
            .current_dir(repo_path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git branch: {}", e))
            })?;

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
    pub fn delete_feature_branch(repo_path: &Path, branch_name: &str) -> AppResult<()> {
        debug!(
            "Deleting feature branch '{}' in {:?}",
            branch_name, repo_path
        );

        let output = Command::new("git")
            .args(["branch", "-d", branch_name])
            .current_dir(repo_path)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git branch -d: {}", e))
            })?;

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
    pub fn checkout_existing_branch_worktree(
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

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                worktree.to_str().unwrap_or_default(),
                branch,
            ])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git worktree add: {}", e))
            })?;

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

    /// Resolves the actual git directory for a worktree or repository.
    ///
    /// For regular repos, returns `worktree/.git`. For worktrees where `.git`
    /// is a file containing `gitdir: <path>`, follows the indirection.
    fn resolve_git_dir(worktree: &Path) -> PathBuf {
        let git_path = worktree.join(".git");

        if git_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&git_path) {
                if let Some(path) = content.strip_prefix("gitdir: ") {
                    return PathBuf::from(path.trim());
                }
            }
        }

        git_path
    }

    /// Check if a rebase is currently in progress
    ///
    /// Detects incomplete rebase by checking for `.git/rebase-merge` or `.git/rebase-apply`
    /// directories which exist while a rebase is paused (e.g., due to conflicts).
    ///
    /// # Arguments
    /// * `worktree` - Path to the git worktree or repository
    pub fn is_rebase_in_progress(worktree: &Path) -> bool {
        let git_dir = Self::resolve_git_dir(worktree);
        git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists()
    }

    /// Detects an incomplete `git merge` by checking for the MERGE_HEAD file.
    ///
    /// MERGE_HEAD exists when a merge has been started but not yet committed
    /// (e.g., the agent resolved conflicts but forgot `git merge --continue`).
    ///
    /// # Arguments
    /// * `worktree` - Path to the git worktree or repository
    pub fn is_merge_in_progress(worktree: &Path) -> bool {
        let git_dir = Self::resolve_git_dir(worktree);
        git_dir.join("MERGE_HEAD").exists()
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
    /// For first tasks on empty repos (base has <= 1 commit), rebase is skipped
    /// as there's no meaningful history to rebase onto - we directly merge instead.
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
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Step 2: Check if base branch is empty (0 or 1 commits)
        // For first task on empty repo, rebase fails due to unrelated histories.
        // Skip rebase and directly merge - the task branch becomes the base history.
        let base_commit_count = Self::get_commit_count(repo, base).unwrap_or(0);
        if base_commit_count <= 1 {
            debug!(
                "Base branch '{}' has {} commit(s), skipping rebase for first task",
                base, base_commit_count
            );

            // Checkout base and merge task branch directly
            debug!("Checking out base branch '{}' for direct merge (empty repo path)", base);
            Self::checkout_branch(repo, base)?;

            let merge_result = Self::merge_branch(repo, task_branch, base)?;
            debug!("Direct merge result for '{}': {:?}", task_branch, merge_result);
            match merge_result {
                MergeResult::Success { commit_sha } | MergeResult::FastForward { commit_sha } => {
                    return Ok(MergeAttemptResult::Success { commit_sha });
                }
                MergeResult::Conflict { files } => {
                    Self::abort_merge(repo)?;
                    return Ok(MergeAttemptResult::NeedsAgent {
                        conflict_files: files,
                    });
                }
            }
        }

        // Step 3: Checkout task branch and rebase onto base (normal case)
        debug!("Checking out task branch '{}' for rebase", task_branch);
        Self::checkout_branch(repo, task_branch)?;

        let rebase_result = Self::rebase_onto(repo, base)?;
        debug!("Rebase result for '{}' onto '{}': {:?}", task_branch, base, rebase_result);
        match rebase_result {
            RebaseResult::Success => {
                // Step 4: Checkout base and merge task branch (should be fast-forward)
                debug!("Checking out base branch '{}' for fast-forward merge", base);
                Self::checkout_branch(repo, base)?;

                let merge_result = Self::merge_branch(repo, task_branch, base)?;
                debug!("Post-rebase merge result for '{}': {:?}", task_branch, merge_result);
                match merge_result {
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

    /// Attempt a direct merge without rebase (for worktree mode)
    ///
    /// Unlike `try_rebase_and_merge`, this uses `git merge` directly which
    /// doesn't require a clean working tree. This is important for worktree mode
    /// where the main repo may have unrelated unstaged changes that would block
    /// `git rebase`.
    ///
    /// Tradeoff: produces merge commits instead of linear history. Acceptable
    /// for worktree-isolated tasks.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `task_branch` - Name of the task branch to merge
    /// * `base` - Name of the base branch to merge into
    pub fn try_merge(
        repo: &Path,
        task_branch: &str,
        base: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting direct merge of '{}' into '{}' in {:?}",
            task_branch, base, repo
        );

        // Step 1: Fetch latest from origin (non-fatal if fails)
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Step 2: Checkout base branch
        debug!("Checking out base branch '{}' for merge", base);
        Self::checkout_branch(repo, base)?;

        // Step 3: Merge task branch into base
        let output = Command::new("git")
            .args(["merge", task_branch, "--no-edit"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git merge: {}", e)))?;

        if output.status.success() {
            let commit_sha = Self::get_head_sha(repo)?;
            debug!("Direct merge succeeded for '{}', SHA: {}", task_branch, commit_sha);
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Check for conflict in both stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stdout.contains("CONFLICT") || stderr.contains("CONFLICT")
            || stdout.contains("conflict") || stderr.contains("conflict")
        {
            let conflict_files = Self::get_conflict_files(repo)?;
            Self::abort_merge(repo)?;
            debug!("Direct merge conflict for '{}', files: {:?}", task_branch, conflict_files);
            return Ok(MergeAttemptResult::NeedsAgent {
                conflict_files,
            });
        }

        Err(AppError::GitOperation(format!(
            "Merge of '{}' into '{}' failed: {}{}",
            task_branch, base, stderr, stdout
        )))
    }

    /// Attempt a merge directly in the primary repository without aborting on conflict
    ///
    /// Unlike `try_merge()`, this method leaves the conflict state in place on
    /// conflict so that the merger agent can resolve conflicts in-place in the
    /// primary repo. Use this when the target branch is already checked out
    /// (e.g., merging a plan feature branch into main in worktree mode).
    ///
    /// - On **success**: returns commit SHA.
    /// - On **conflict**: leaves the merge in conflict state (does NOT abort).
    ///   The caller (or merger agent) can resolve conflicts in the primary repo.
    /// - On **error**: returns an error.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `source_branch` - Branch to merge from (e.g., plan feature branch)
    /// * `target_branch` - Branch to merge into (e.g., main — already checked out)
    pub fn try_merge_in_repo(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting in-repo merge of '{}' into '{}' in {:?}",
            source_branch, target_branch, repo
        );

        // Step 1: Fetch latest from origin (non-fatal if fails)
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Step 2: Checkout target branch (no-op if already checked out)
        debug!("Checking out target branch '{}' for in-repo merge", target_branch);
        Self::checkout_branch(repo, target_branch)?;

        // Step 3: Merge source branch into target
        let output = Command::new("git")
            .args(["merge", source_branch, "--no-edit"])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to run git merge: {}", e)))?;

        if output.status.success() {
            let commit_sha = Self::get_head_sha(repo)?;
            debug!(
                "In-repo merge succeeded for '{}', SHA: {}",
                source_branch, commit_sha
            );
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Check for conflict in both stdout and stderr
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stdout.contains("CONFLICT")
            || stderr.contains("CONFLICT")
            || stdout.contains("conflict")
            || stderr.contains("conflict")
        {
            let conflict_files = Self::get_conflict_files(repo)?;
            debug!(
                "In-repo merge conflict for '{}', files: {:?}",
                source_branch, conflict_files
            );
            // Do NOT abort — leave conflict state in place for agent resolution
            return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
        }

        Err(AppError::GitOperation(format!(
            "In-repo merge of '{}' into '{}' failed: {}{}",
            source_branch, target_branch, stderr, stdout
        )))
    }

    /// Attempt a merge in an isolated worktree
    ///
    /// Creates a temporary merge worktree that checks out the target branch,
    /// then merges the source branch into it. This avoids disrupting the main
    /// repository's working directory.
    ///
    /// - On **success**: returns commit SHA. Caller should clean up the merge worktree.
    /// - On **conflict**: leaves the merge worktree in conflict state (does NOT abort).
    ///   The caller (or merger agent) can resolve conflicts in the merge worktree.
    /// - On **error**: cleans up the merge worktree and returns an error.
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `source_branch` - Branch to merge from (e.g., task branch)
    /// * `target_branch` - Branch to merge into (e.g., plan feature branch)
    /// * `merge_worktree_path` - Path for the temporary merge worktree
    pub fn try_merge_in_worktree(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        merge_worktree_path: &Path,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting merge of '{}' into '{}' in worktree {:?}",
            source_branch, target_branch, merge_worktree_path
        );

        // Step 1: Create merge worktree checking out the target branch
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch)?;

        // Step 2: Merge source branch into the merge worktree
        let output = Command::new("git")
            .args(["merge", source_branch, "--no-edit"])
            .current_dir(merge_worktree_path)
            .output()
            .map_err(|e| {
                // Clean up worktree on command execution failure
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                AppError::GitOperation(format!("Failed to run git merge: {}", e))
            })?;

        if output.status.success() {
            let commit_sha = Self::get_head_sha(merge_worktree_path)?;
            debug!(
                "Merge succeeded in worktree, SHA: {}",
                commit_sha
            );
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Check for conflict
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stdout.contains("CONFLICT")
            || stderr.contains("CONFLICT")
            || stdout.contains("conflict")
            || stderr.contains("conflict")
        {
            let conflict_files = Self::get_conflict_files(merge_worktree_path)?;
            debug!(
                "Merge conflict in worktree, files: {:?}",
                conflict_files
            );
            // Do NOT abort — leave worktree in conflict state for agent resolution
            return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
        }

        // Unexpected error — clean up worktree
        let _ = Self::delete_worktree(repo, merge_worktree_path);
        Err(AppError::GitOperation(format!(
            "Merge of '{}' into '{}' in worktree failed: {}{}",
            source_branch, target_branch, stderr, stdout
        )))
    }

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
            AppError::GitOperation(format!("Failed to parse commit count '{}': {}", count_str, e))
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
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git merge-base: {}", e))
            })?;

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

        let mut output = Command::new("git")
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

        if output.stdout.is_empty() {
            // Fallback: include the merge commit itself (e.g., manual resolution commit on base)
            let single_range = format!("{}^..{}", merge_commit_sha, merge_commit_sha);
            output = Command::new("git")
                .args([
                    "log",
                    &single_range,
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

    // =========================================================================
    // resolve_git_dir Tests
    // =========================================================================

    #[test]
    fn test_resolve_git_dir_regular_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        assert_eq!(GitService::resolve_git_dir(temp_dir.path()), git_dir);
    }

    #[test]
    fn test_resolve_git_dir_worktree_style() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_path = temp_dir.path().join(".git");

        let actual_git_dir = temp_dir.path().join("actual_git_dir");
        std::fs::create_dir(&actual_git_dir).unwrap();

        std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

        assert_eq!(GitService::resolve_git_dir(temp_dir.path()), actual_git_dir);
    }

    // =========================================================================
    // is_merge_in_progress Tests
    // =========================================================================

    #[test]
    fn test_is_merge_in_progress_no_merge() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        assert!(!GitService::is_merge_in_progress(temp_dir.path()));
    }

    #[test]
    fn test_is_merge_in_progress_with_merge_head() {
        let temp_dir = tempfile::tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        // Simulate MERGE_HEAD file (merge started but not committed)
        std::fs::write(git_dir.join("MERGE_HEAD"), "abc123\n").unwrap();

        assert!(GitService::is_merge_in_progress(temp_dir.path()));
    }

    #[test]
    fn test_is_merge_in_progress_worktree_style() {
        // Test worktree-style .git file pointing to gitdir
        let temp_dir = tempfile::tempdir().unwrap();
        let git_path = temp_dir.path().join(".git");

        // Create the actual git directory somewhere else
        let actual_git_dir = temp_dir.path().join("actual_git_dir");
        std::fs::create_dir(&actual_git_dir).unwrap();

        // Create .git file pointing to actual git dir
        std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

        // No merge in progress
        assert!(!GitService::is_merge_in_progress(temp_dir.path()));

        // Add MERGE_HEAD to actual git dir
        std::fs::write(actual_git_dir.join("MERGE_HEAD"), "abc123\n").unwrap();

        assert!(GitService::is_merge_in_progress(temp_dir.path()));
    }

    // =========================================================================
    // is_commit_on_branch Tests (Phase 78)
    // =========================================================================

    #[test]
    fn test_is_commit_on_branch_with_valid_ancestor() {
        // Create a temp git repo with a commit on main
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Get the commit SHA
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap();
        let commit_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Verify commit is on HEAD (main/master)
        let result = GitService::is_commit_on_branch(repo, &commit_sha, "HEAD");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_is_commit_on_branch_with_non_ancestor() {
        // Create a temp git repo with divergent branches
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial commit on main
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Make commit on feature branch
        std::fs::write(repo.join("feature.txt"), "feature content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Get feature commit SHA
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap();
        let feature_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Go back to main
        Command::new("git")
            .args(["checkout", "master"])
            .current_dir(repo)
            .output()
            .ok(); // May be "main" instead of "master"
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .ok();

        // Get main branch name
        let branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap();
        let main_branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // Feature commit should NOT be on main (not merged yet)
        let result = GitService::is_commit_on_branch(repo, &feature_sha, &main_branch);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // =========================================================================
    // get_commit_count Tests (Phase 78)
    // =========================================================================

    #[test]
    fn test_get_commit_count_empty_repo() {
        // Create a temp git repo with only an initial commit
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Should have exactly 1 commit
        let result = GitService::get_commit_count(repo, "HEAD");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_get_commit_count_multiple_commits() {
        // Create a temp git repo with multiple commits
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create 3 commits
        for i in 1..=3 {
            std::fs::write(repo.join(format!("test{}.txt", i)), format!("content {}", i)).unwrap();
            Command::new("git")
                .args(["add", "."])
                .current_dir(repo)
                .output()
                .unwrap();
            Command::new("git")
                .args(["commit", "-m", &format!("commit {}", i)])
                .current_dir(repo)
                .output()
                .unwrap();
        }

        // Should have exactly 3 commits
        let result = GitService::get_commit_count(repo, "HEAD");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    // =========================================================================
    // try_rebase_and_merge Tests (Phase 78)
    // =========================================================================

    #[test]
    fn test_try_rebase_and_merge_first_task_on_empty_repo() {
        // Test that first task on empty repo (only 1 commit) bypasses rebase
        // and directly merges the task branch
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial empty commit on main
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "initial empty commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Rename default branch to 'main' if needed
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output();

        // Create task branch from main
        Command::new("git")
            .args(["checkout", "-b", "task-branch"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Add content on task branch
        std::fs::write(repo.join("feature.txt"), "feature content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Go back to main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Verify main has only 1 commit
        let count = GitService::get_commit_count(repo, "main").unwrap();
        assert_eq!(count, 1, "Main should have only 1 commit before merge");

        // Try rebase and merge - should skip rebase and merge directly
        let result = GitService::try_rebase_and_merge(repo, "task-branch", "main");
        assert!(result.is_ok(), "try_rebase_and_merge should succeed for first task");

        match result.unwrap() {
            MergeAttemptResult::Success { commit_sha } => {
                // Verify commit is on main
                let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
                assert!(on_main, "Merge commit should be on main branch");

                // Verify feature file exists
                assert!(
                    repo.join("feature.txt").exists(),
                    "Feature file should exist after merge"
                );
            }
            MergeAttemptResult::NeedsAgent { .. } => {
                panic!("First task on empty repo should not need agent");
            }
        }
    }

    #[test]
    fn test_try_rebase_and_merge_normal_case_with_history() {
        // Test that normal case (>1 commit on base) uses rebase workflow
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial commit with content
        std::fs::write(repo.join("initial.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Rename default branch to 'main' if needed
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output();

        // Add second commit on main
        std::fs::write(repo.join("second.txt"), "second").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "second commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create task branch from main
        Command::new("git")
            .args(["checkout", "-b", "task-branch"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Add content on task branch
        std::fs::write(repo.join("feature.txt"), "feature content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Go back to main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Verify main has >1 commits
        let count = GitService::get_commit_count(repo, "main").unwrap();
        assert!(count > 1, "Main should have >1 commits (has {})", count);

        // Try rebase and merge - should use normal rebase workflow
        let result = GitService::try_rebase_and_merge(repo, "task-branch", "main");
        assert!(result.is_ok(), "try_rebase_and_merge should succeed");

        match result.unwrap() {
            MergeAttemptResult::Success { commit_sha } => {
                // Verify commit is on main
                let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
                assert!(on_main, "Merge commit should be on main branch");

                // Verify all files exist
                assert!(repo.join("initial.txt").exists(), "Initial file should exist");
                assert!(repo.join("second.txt").exists(), "Second file should exist");
                assert!(repo.join("feature.txt").exists(), "Feature file should exist");
            }
            MergeAttemptResult::NeedsAgent { .. } => {
                panic!("Clean merge should not need agent");
            }
        }
    }

    // =========================================================================
    // Merge Verification Tests (Phase 78 - Task 5)
    // =========================================================================
    // These tests verify the merge verification logic used by attempt_merge_auto_complete
    // and complete_merge HTTP handler to detect when a commit is NOT on main branch.

    #[test]
    fn test_merge_verification_detects_unmerged_task_branch() {
        // This test verifies the core logic that attempt_merge_auto_complete uses:
        // 1. Get task branch HEAD SHA
        // 2. Check if that SHA is on main branch
        // 3. If not, the merge is not complete
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Create initial commit on main
        std::fs::write(repo.join("initial.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Rename to main
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output();

        // Create task branch
        Command::new("git")
            .args(["checkout", "-b", "task-branch"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Add work on task branch
        std::fs::write(repo.join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Get task branch HEAD SHA (simulating getting SHA from worktree)
        let task_branch_head = GitService::get_head_sha(repo).unwrap();

        // Go back to main WITHOUT merging
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();

        // Verify task branch commit is NOT on main - this is the key check
        // that attempt_merge_auto_complete uses before marking merge complete
        let is_on_main = GitService::is_commit_on_branch(repo, &task_branch_head, "main").unwrap();
        assert!(
            !is_on_main,
            "Task branch HEAD {} should NOT be on main before merge",
            task_branch_head
        );

        // Now merge the task branch
        Command::new("git")
            .args(["merge", "task-branch", "-m", "merge task branch"])
            .current_dir(repo)
            .output()
            .unwrap();

        // After merge, task branch commit SHOULD be on main
        let is_on_main_after = GitService::is_commit_on_branch(repo, &task_branch_head, "main").unwrap();
        assert!(
            is_on_main_after,
            "Task branch HEAD {} should be on main after merge",
            task_branch_head
        );

        // Main HEAD should be at least at task branch HEAD (fast-forward or merge commit)
        let main_head = GitService::get_head_sha(repo).unwrap();
        // In fast-forward case, they'll be equal; in merge commit case, main_head is newer
        // The key verification is that is_commit_on_branch returned true - that's what matters
        assert!(
            !main_head.is_empty(),
            "Main HEAD should have a valid SHA after merge"
        );
    }

    #[test]
    fn test_merge_verification_uses_correct_repo_path() {
        // This test verifies that checking the main repo (not worktree) correctly
        // identifies merge status - simulating the fix for the original bug
        let temp_dir = tempfile::tempdir().unwrap();
        let main_repo = temp_dir.path().join("main-repo");
        let worktree = temp_dir.path().join("worktree");

        std::fs::create_dir(&main_repo).unwrap();
        std::fs::create_dir(&worktree).unwrap();

        // Initialize main repo
        Command::new("git")
            .args(["init"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // Configure git user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&main_repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // Create initial commit on main
        std::fs::write(main_repo.join("initial.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&main_repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(&main_repo)
            .output();

        // Create task branch
        Command::new("git")
            .args(["checkout", "-b", "task-branch"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // Add work on task branch
        std::fs::write(main_repo.join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&main_repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // Get task branch HEAD (this is what worktree would have)
        let task_branch_head = GitService::get_head_sha(&main_repo).unwrap();

        // Simulate creating a worktree (just init a separate repo for simplicity)
        // In real code, worktree would have task branch checked out
        Command::new("git")
            .args(["init"])
            .current_dir(&worktree)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&worktree)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&worktree)
            .output()
            .unwrap();
        std::fs::write(worktree.join("feature.txt"), "feature").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&worktree)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(&worktree)
            .output()
            .unwrap();

        // Go back to main in main repo
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // KEY TEST: Checking worktree HEAD vs main repo's main branch
        // The worktree has its own commits, not related to main_repo's main branch
        let worktree_head = GitService::get_head_sha(&worktree).unwrap();

        // Worktree HEAD is NOT on main_repo's main branch - this is the bug we fixed
        // (Previously, code was using worktree HEAD as merge commit)
        let result = GitService::is_commit_on_branch(&main_repo, &worktree_head, "main");
        // This will error or return false because worktree_head doesn't exist in main_repo
        assert!(
            result.is_err() || !result.unwrap(),
            "Worktree HEAD should NOT be found on main_repo's main branch"
        );

        // The correct check: task branch HEAD from main_repo
        let is_merged = GitService::is_commit_on_branch(&main_repo, &task_branch_head, "main").unwrap();
        assert!(
            !is_merged,
            "Task branch HEAD should NOT be on main until merged"
        );

        // Now merge in main_repo
        Command::new("git")
            .args(["merge", "task-branch", "-m", "merge task"])
            .current_dir(&main_repo)
            .output()
            .unwrap();

        // Now task branch HEAD should be on main
        let is_merged_after = GitService::is_commit_on_branch(&main_repo, &task_branch_head, "main").unwrap();
        assert!(
            is_merged_after,
            "Task branch HEAD should be on main after merge"
        );
    }

    // =========================================================================
    // Feature Branch Operations Tests (Phase 85)
    // =========================================================================

    #[test]
    fn test_create_feature_branch_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo with initial commit
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Create feature branch
        let result = GitService::create_feature_branch(repo, "ralphx/my-app/plan-abc123", "main");
        assert!(result.is_ok(), "create_feature_branch should succeed: {:?}", result.err());

        // Verify branch exists
        let output = Command::new("git")
            .args(["branch", "--list", "ralphx/my-app/plan-abc123"])
            .current_dir(repo)
            .output()
            .unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(branches.contains("ralphx/my-app/plan-abc123"), "Feature branch should exist");

        // Verify we didn't checkout the branch (still on main)
        let current = GitService::get_current_branch(repo).unwrap();
        assert_eq!(current, "main", "Should still be on main after creating feature branch");
    }

    #[test]
    fn test_create_feature_branch_from_specific_source() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo with initial commit on main
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Add another commit on main
        std::fs::write(repo.join("file2.txt"), "second").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "second"]).current_dir(repo).output().unwrap();

        let main_sha = GitService::get_head_sha(repo).unwrap();

        // Create feature branch from main
        let result = GitService::create_feature_branch(repo, "feature/plan-test", "main");
        assert!(result.is_ok());

        // Verify feature branch points to same commit as main
        let output = Command::new("git")
            .args(["rev-parse", "feature/plan-test"])
            .current_dir(repo)
            .output()
            .unwrap();
        let feature_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(feature_sha, main_sha, "Feature branch should point to main HEAD");
    }

    #[test]
    fn test_create_feature_branch_already_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Create branch first time
        GitService::create_feature_branch(repo, "feature/dup", "main").unwrap();

        // Try to create again — should fail
        let result = GitService::create_feature_branch(repo, "feature/dup", "main");
        assert!(result.is_err(), "Creating duplicate feature branch should fail");
    }

    #[test]
    fn test_create_feature_branch_invalid_source() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Create from non-existent source branch
        let result = GitService::create_feature_branch(repo, "feature/bad", "nonexistent-branch");
        assert!(result.is_err(), "Creating from non-existent source should fail");
    }

    #[test]
    fn test_delete_feature_branch_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Create feature branch, then merge it back so -d works
        GitService::create_feature_branch(repo, "feature/to-delete", "main").unwrap();

        // Delete it (safe delete — branch is fully merged since it's at same commit as main)
        let result = GitService::delete_feature_branch(repo, "feature/to-delete");
        assert!(result.is_ok(), "delete_feature_branch should succeed: {:?}", result.err());

        // Verify branch no longer exists
        let output = Command::new("git")
            .args(["branch", "--list", "feature/to-delete"])
            .current_dir(repo)
            .output()
            .unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(!branches.contains("feature/to-delete"), "Feature branch should be deleted");
    }

    #[test]
    fn test_delete_feature_branch_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("file.txt"), "content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        // Delete non-existent branch — should fail
        let result = GitService::delete_feature_branch(repo, "feature/nonexistent");
        assert!(result.is_err(), "Deleting non-existent branch should fail");
    }

    // =========================================================================
    // try_merge Tests (Phase 98 - Worktree mode direct merge)
    // =========================================================================

    #[test]
    fn test_try_merge_clean_fast_forward() {
        // Task branch has commits ahead of base, no divergence → fast-forward
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();

        // Initial commit on main
        std::fs::write(repo.join("initial.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create task branch from main
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("feature.txt"), "feature").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "add feature"]).current_dir(repo).output().unwrap();

        // Back to main (no new commits on main → fast-forward possible)
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();

        let result = GitService::try_merge(repo, "task-branch", "main");
        assert!(result.is_ok(), "try_merge should succeed: {:?}", result.err());

        match result.unwrap() {
            MergeAttemptResult::Success { commit_sha } => {
                let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
                assert!(on_main, "Merge commit should be on main");
                assert!(repo.join("feature.txt").exists(), "Feature file should exist");
            }
            MergeAttemptResult::NeedsAgent { .. } => {
                panic!("Clean fast-forward merge should not need agent");
            }
        }
    }

    #[test]
    fn test_try_merge_with_diverged_branches() {
        // Both base and task branch have new commits → merge commit created
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();

        // Initial commit on main
        std::fs::write(repo.join("initial.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create task branch
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("feature.txt"), "feature").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "add feature"]).current_dir(repo).output().unwrap();

        // Go back to main and add a non-conflicting commit
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("other.txt"), "other work").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "other work on main"]).current_dir(repo).output().unwrap();

        let result = GitService::try_merge(repo, "task-branch", "main");
        assert!(result.is_ok(), "try_merge should succeed: {:?}", result.err());

        match result.unwrap() {
            MergeAttemptResult::Success { commit_sha } => {
                let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
                assert!(on_main, "Merge commit should be on main");
                assert!(repo.join("feature.txt").exists(), "Feature file should exist");
                assert!(repo.join("other.txt").exists(), "Other file should exist");
            }
            MergeAttemptResult::NeedsAgent { .. } => {
                panic!("Non-conflicting diverged merge should not need agent");
            }
        }
    }

    #[test]
    fn test_try_merge_with_conflict() {
        // Both branches modify the same file → conflict → NeedsAgent
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test"]).current_dir(repo).output().unwrap();

        // Initial commit
        std::fs::write(repo.join("shared.txt"), "original content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Task branch modifies shared file
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("shared.txt"), "task branch changes").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "task changes"]).current_dir(repo).output().unwrap();

        // Main also modifies shared file (conflict)
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("shared.txt"), "main branch changes").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "main changes"]).current_dir(repo).output().unwrap();

        let result = GitService::try_merge(repo, "task-branch", "main");
        assert!(result.is_ok(), "try_merge should return Ok even on conflict: {:?}", result.err());

        match result.unwrap() {
            MergeAttemptResult::NeedsAgent { conflict_files } => {
                assert!(!conflict_files.is_empty(), "Should report conflict files");
                // Verify merge was aborted (repo is clean)
                let has_changes = GitService::has_uncommitted_changes(repo).unwrap();
                assert!(!has_changes, "Merge should be aborted, no uncommitted changes");
            }
            MergeAttemptResult::Success { .. } => {
                panic!("Conflicting merge should need agent, not succeed");
            }
        }
    }

    // =========================================================================
    // checkout_existing_branch_worktree Tests
    // =========================================================================

    #[test]
    fn test_checkout_existing_branch_worktree_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Initialize repo with a commit
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create a feature branch
        Command::new("git").args(["branch", "feature-branch"]).current_dir(repo).output().unwrap();

        // Create worktree checking out the existing branch
        let worktree_path = temp_dir.path().join("worktrees").join("merge-wt");
        let result = GitService::checkout_existing_branch_worktree(repo, &worktree_path, "feature-branch");
        assert!(result.is_ok(), "Should succeed: {:?}", result.err());

        // Verify worktree was created and is on the correct branch
        assert!(worktree_path.exists(), "Worktree directory should exist");
        let branch = GitService::get_current_branch(&worktree_path).unwrap();
        assert_eq!(branch, "feature-branch");
    }

    #[test]
    fn test_checkout_existing_branch_worktree_creates_parent_dirs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        Command::new("git").args(["branch", "feature"]).current_dir(repo).output().unwrap();

        // Path with deeply nested non-existent parent dirs
        let worktree_path = temp_dir.path().join("deep").join("nested").join("merge-wt");
        let result = GitService::checkout_existing_branch_worktree(repo, &worktree_path, "feature");
        assert!(result.is_ok(), "Should create parent dirs: {:?}", result.err());
        assert!(worktree_path.exists());
    }

    #[test]
    fn test_checkout_existing_branch_worktree_nonexistent_branch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();

        let worktree_path = temp_dir.path().join("merge-wt");
        let result = GitService::checkout_existing_branch_worktree(repo, &worktree_path, "nonexistent-branch");
        assert!(result.is_err(), "Should fail for nonexistent branch");
    }

    // =========================================================================
    // try_merge_in_worktree Tests
    // =========================================================================

    #[test]
    fn test_try_merge_in_worktree_fast_forward() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Setup: feature-branch as target, task-branch as source (fast-forward case)
        // Main repo stays on main; merge worktree checks out feature-branch
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create feature branch (target) at current commit
        Command::new("git").args(["branch", "feature-branch"]).current_dir(repo).output().unwrap();

        // Create task branch with a new file (fast-forward from feature-branch)
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("new-file.txt"), "task work").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "task work"]).current_dir(repo).output().unwrap();

        // Go back to main (user's working branch)
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();

        let merge_wt = temp_dir.path().join("merge-wt");
        let result = GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);
        assert!(result.is_ok(), "Merge should succeed: {:?}", result.err());

        match result.unwrap() {
            MergeAttemptResult::Success { commit_sha } => {
                assert!(!commit_sha.is_empty(), "Should have commit SHA");
            }
            MergeAttemptResult::NeedsAgent { .. } => {
                panic!("Fast-forward merge should succeed, not need agent");
            }
        }

        // Merge worktree should still exist (caller responsible for cleanup)
        assert!(merge_wt.exists(), "Merge worktree should still exist after success");

        // Clean up worktree
        let _ = GitService::delete_worktree(repo, &merge_wt);
    }

    #[test]
    fn test_try_merge_in_worktree_conflict() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Setup: feature-branch and task-branch modify same file differently
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("shared.txt"), "initial content").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create feature branch (target) and add divergent changes
        Command::new("git").args(["checkout", "-b", "feature-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("shared.txt"), "feature branch changes").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "feature changes"]).current_dir(repo).output().unwrap();

        // Create task branch from main with conflicting changes
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("shared.txt"), "task branch changes").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "task changes"]).current_dir(repo).output().unwrap();

        // Go back to main
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();

        let merge_wt = temp_dir.path().join("merge-wt");
        let result = GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);
        assert!(result.is_ok(), "Should return Ok even on conflict: {:?}", result.err());

        match result.unwrap() {
            MergeAttemptResult::NeedsAgent { conflict_files } => {
                assert!(!conflict_files.is_empty(), "Should report conflict files");
                // Merge worktree should still exist (for agent to resolve in)
                assert!(merge_wt.exists(), "Merge worktree should be kept for conflict resolution");
                // MERGE_HEAD should exist (merge NOT aborted)
                assert!(
                    GitService::is_merge_in_progress(&merge_wt),
                    "Merge should still be in progress in worktree"
                );
            }
            MergeAttemptResult::Success { .. } => {
                panic!("Conflicting merge should need agent, not succeed");
            }
        }

        // Clean up
        let _ = GitService::delete_worktree(repo, &merge_wt);
    }

    #[test]
    fn test_try_merge_in_worktree_does_not_touch_main_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = temp_dir.path();

        // Setup repo with feature-branch as merge target
        Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.email", "test@example.com"]).current_dir(repo).output().unwrap();
        Command::new("git").args(["config", "user.name", "Test User"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("test.txt"), "initial").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "initial"]).current_dir(repo).output().unwrap();
        let _ = Command::new("git").args(["branch", "-M", "main"]).current_dir(repo).output();

        // Create feature branch (target)
        Command::new("git").args(["branch", "feature-branch"]).current_dir(repo).output().unwrap();

        // Create task branch
        Command::new("git").args(["checkout", "-b", "task-branch"]).current_dir(repo).output().unwrap();
        std::fs::write(repo.join("new.txt"), "task").unwrap();
        Command::new("git").args(["add", "."]).current_dir(repo).output().unwrap();
        Command::new("git").args(["commit", "-m", "task"]).current_dir(repo).output().unwrap();

        // Go back to main — this is the branch the user is working on
        Command::new("git").args(["checkout", "main"]).current_dir(repo).output().unwrap();

        // Record main repo state before merge
        let branch_before = GitService::get_current_branch(repo).unwrap();

        let merge_wt = temp_dir.path().join("merge-wt");
        let _ = GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);

        // Main repo should still be on the same branch
        let branch_after = GitService::get_current_branch(repo).unwrap();
        assert_eq!(branch_before, branch_after, "Main repo branch should not change");

        // Clean up
        let _ = GitService::delete_worktree(repo, &merge_wt);
    }
}
