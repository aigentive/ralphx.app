//! Merge operations for the two-phase merge workflow
//!
//! Extracted from `mod.rs` — contains merge, squash merge, and
//! worktree-isolated merge operations. Rebase operations are in `rebase.rs`.

use super::*;

impl GitService {
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

        Err(AppError::GitOperation(format!("Merge failed: {}", stderr)))
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

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, task_branch, base) {
            return Ok(not_found);
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
            debug!(
                "Checking out base branch '{}' for direct merge (empty repo path)",
                base
            );
            Self::checkout_branch(repo, base)?;

            let merge_result = Self::merge_branch(repo, task_branch, base)?;
            debug!(
                "Direct merge result for '{}': {:?}",
                task_branch, merge_result
            );
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
        debug!(
            "Rebase result for '{}' onto '{}': {:?}",
            task_branch, base, rebase_result
        );
        match rebase_result {
            RebaseResult::Success => {
                // Step 4: Checkout base and merge task branch (should be fast-forward)
                debug!("Checking out base branch '{}' for fast-forward merge", base);
                Self::checkout_branch(repo, base)?;

                let merge_result = Self::merge_branch(repo, task_branch, base)?;
                debug!(
                    "Post-rebase merge result for '{}': {:?}",
                    task_branch, merge_result
                );
                match merge_result {
                    MergeResult::Success { commit_sha }
                    | MergeResult::FastForward { commit_sha } => {
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
    pub fn try_merge(repo: &Path, task_branch: &str, base: &str) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting direct merge of '{}' into '{}' in {:?}",
            task_branch, base, repo
        );

        // Step 1: Fetch latest from origin (non-fatal if fails)
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, task_branch, base) {
            return Ok(not_found);
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
            debug!(
                "Direct merge succeeded for '{}', SHA: {}",
                task_branch, commit_sha
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
            Self::abort_merge(repo)?;
            debug!(
                "Direct merge conflict for '{}', files: {:?}",
                task_branch, conflict_files
            );
            return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
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

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Step 2: Checkout target branch (no-op if already checked out)
        debug!(
            "Checking out target branch '{}' for in-repo merge",
            target_branch
        );
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

    /// Squash merge: squash all commits from source into a single commit on target (local mode)
    ///
    /// Runs `git merge --squash` followed by `git commit`. Produces a clean single
    /// commit on the target branch with no merge commit.
    ///
    /// - On **success**: returns commit SHA of the squashed commit.
    /// - On **conflict**: aborts the merge and returns NeedsAgent.
    /// - On **error**: returns an error.
    pub fn try_squash_merge(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        commit_message: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting squash merge of '{}' into '{}' in {:?}",
            source_branch, target_branch, repo
        );

        // Step 1: Fetch latest from origin (non-fatal)
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch).unwrap_or(false) {
            debug!(
                "Source '{}' and target '{}' already identical, skipping merge",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch)?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 2: Checkout target branch
        Self::checkout_branch(repo, target_branch)?;

        // Step 3: Squash merge (stages changes but does NOT commit)
        let output = Command::new("git")
            .args(["merge", "--squash", source_branch])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                AppError::GitOperation(format!("Failed to run git merge --squash: {}", e))
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stdout.contains("CONFLICT")
                || stderr.contains("CONFLICT")
                || stdout.contains("conflict")
                || stderr.contains("conflict")
            {
                let conflict_files = Self::get_conflict_files(repo)?;
                Self::abort_merge(repo)?;
                debug!(
                    "Squash merge conflict for '{}', files: {:?}",
                    source_branch, conflict_files
                );
                return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
            }

            return Err(AppError::GitOperation(format!(
                "Squash merge of '{}' into '{}' failed: {}{}",
                source_branch, target_branch, stderr, stdout
            )));
        }

        // Step 4: Commit the squashed changes
        let commit_output = Command::new("git")
            .args(["commit", "-m", commit_message])
            .current_dir(repo)
            .output()
            .map_err(|e| AppError::GitOperation(format!("Failed to commit squash merge: {}", e)))?;

        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout);
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            // "nothing to commit" means branches are identical — treat as success
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                let commit_sha = Self::get_head_sha(repo)?;
                debug!(
                    "Squash merge no-op (branches identical), SHA: {}",
                    commit_sha
                );
                return Ok(MergeAttemptResult::Success { commit_sha });
            }
            return Err(AppError::GitOperation(format!(
                "Failed to commit squash merge: stdout={}, stderr={}",
                stdout, stderr
            )));
        }

        let commit_sha = Self::get_head_sha(repo)?;
        debug!(
            "Squash merge succeeded for '{}', SHA: {}",
            source_branch, commit_sha
        );
        Ok(MergeAttemptResult::Success { commit_sha })
    }

    /// Squash merge in an isolated worktree
    ///
    /// Creates a temporary worktree on the target branch, squash-merges the source
    /// branch, and commits. Avoids disrupting the main repo working directory.
    ///
    /// - On **success**: returns commit SHA. Caller should clean up the worktree.
    /// - On **conflict**: leaves worktree in conflict state for agent resolution.
    /// - On **error**: cleans up worktree and returns error.
    pub fn try_squash_merge_in_worktree(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        merge_worktree_path: &Path,
        commit_message: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting squash merge of '{}' into '{}' in worktree {:?}",
            source_branch, target_branch, merge_worktree_path
        );

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch).unwrap_or(false) {
            debug!(
                "Source '{}' and target '{}' already identical, skipping worktree merge",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch)?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 1: Create worktree on target branch
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch)?;

        // Step 2: Squash merge source into worktree
        let output = Command::new("git")
            .args(["merge", "--squash", source_branch])
            .current_dir(merge_worktree_path)
            .output()
            .map_err(|e| {
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                AppError::GitOperation(format!("Failed to run git merge --squash: {}", e))
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stdout.contains("CONFLICT")
                || stderr.contains("CONFLICT")
                || stdout.contains("conflict")
                || stderr.contains("conflict")
            {
                let conflict_files = Self::get_conflict_files(merge_worktree_path)?;
                debug!(
                    "Squash merge conflict in worktree, files: {:?}",
                    conflict_files
                );
                // Leave worktree in conflict state for agent resolution
                return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
            }

            let _ = Self::delete_worktree(repo, merge_worktree_path);
            return Err(AppError::GitOperation(format!(
                "Squash merge of '{}' into '{}' in worktree failed: {}{}",
                source_branch, target_branch, stderr, stdout
            )));
        }

        // Step 3: Commit the squashed changes in the worktree
        let commit_output = Command::new("git")
            .args(["commit", "-m", commit_message])
            .current_dir(merge_worktree_path)
            .output()
            .map_err(|e| {
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                AppError::GitOperation(format!("Failed to commit squash merge: {}", e))
            })?;

        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout);
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                let commit_sha = Self::get_head_sha(merge_worktree_path)?;
                debug!(
                    "Squash merge no-op in worktree (branches identical), SHA: {}",
                    commit_sha
                );
                return Ok(MergeAttemptResult::Success { commit_sha });
            }
            let _ = Self::delete_worktree(repo, merge_worktree_path);
            return Err(AppError::GitOperation(format!(
                "Failed to commit squash merge in worktree: stdout={}, stderr={}",
                stdout, stderr
            )));
        }

        let commit_sha = Self::get_head_sha(merge_worktree_path)?;
        debug!("Squash merge in worktree succeeded, SHA: {}", commit_sha);
        Ok(MergeAttemptResult::Success { commit_sha })
    }

    /// Rebase source onto target, then squash into a single commit (local mode)
    ///
    /// 1. Rebase source_branch onto target_branch (conflicts caught here)
    /// 2. Checkout target, `git merge --squash source`, commit
    /// 3. Result: single clean commit on target with all changes
    pub fn try_rebase_squash_merge(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        commit_message: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting rebase+squash of '{}' onto '{}' in {:?}",
            source_branch, target_branch, repo
        );

        // Step 1: Fetch
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch).unwrap_or(false) {
            debug!(
                "Source '{}' and target '{}' already identical, skipping rebase+squash",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch)?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 2: Check for empty base (skip rebase for first task)
        let base_commit_count = Self::get_commit_count(repo, target_branch).unwrap_or(0);
        if base_commit_count <= 1 {
            debug!(
                "Base branch '{}' has {} commit(s), using plain squash merge",
                target_branch, base_commit_count
            );
            return Self::try_squash_merge(repo, source_branch, target_branch, commit_message);
        }

        // Step 3: Checkout source branch and rebase onto target
        Self::checkout_branch(repo, source_branch)?;
        let rebase_result = Self::rebase_onto(repo, target_branch)?;

        match rebase_result {
            RebaseResult::Success => {
                // Step 4: Checkout target and squash merge
                Self::checkout_branch(repo, target_branch)?;

                let output = Command::new("git")
                    .args(["merge", "--squash", source_branch])
                    .current_dir(repo)
                    .output()
                    .map_err(|e| {
                        AppError::GitOperation(format!(
                            "Failed to run git merge --squash after rebase: {}",
                            e
                        ))
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // After successful rebase, squash should not conflict
                    return Err(AppError::GitOperation(format!(
                        "Squash merge after rebase unexpectedly failed: {}",
                        stderr
                    )));
                }

                // Step 5: Commit
                let commit_output = Command::new("git")
                    .args(["commit", "-m", commit_message])
                    .current_dir(repo)
                    .output()
                    .map_err(|e| {
                        AppError::GitOperation(format!(
                            "Failed to commit rebase+squash merge: {}",
                            e
                        ))
                    })?;

                if !commit_output.status.success() {
                    let stdout = String::from_utf8_lossy(&commit_output.stdout);
                    let stderr = String::from_utf8_lossy(&commit_output.stderr);
                    if stdout.contains("nothing to commit") || stderr.contains("nothing to commit")
                    {
                        let sha = Self::get_head_sha(repo)?;
                        return Ok(MergeAttemptResult::Success { commit_sha: sha });
                    }
                    return Err(AppError::GitOperation(format!(
                        "Failed to commit rebase+squash: stdout={}, stderr={}",
                        stdout, stderr
                    )));
                }

                let sha = Self::get_head_sha(repo)?;
                debug!("Rebase+squash succeeded, SHA: {}", sha);
                Ok(MergeAttemptResult::Success { commit_sha: sha })
            }
            RebaseResult::Conflict { files } => {
                Self::abort_rebase(repo)?;
                Self::checkout_branch(repo, target_branch)?;
                debug!("Rebase conflict during rebase+squash, files: {:?}", files);
                Ok(MergeAttemptResult::NeedsAgent {
                    conflict_files: files,
                })
            }
        }
    }

    /// Rebase source onto target in a worktree, then squash into a single commit
    ///
    /// 1. Create rebase worktree on source, rebase onto target
    /// 2. On success: delete rebase wt, create merge wt on target, squash merge, commit
    /// 3. On conflict: leave rebase worktree for agent resolution
    pub fn try_rebase_squash_merge_in_worktree(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        rebase_worktree_path: &Path,
        merge_worktree_path: &Path,
        commit_message: &str,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting rebase+squash of '{}' onto '{}' in worktrees",
            source_branch, target_branch
        );

        // Step 1: Fetch
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded"),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch).unwrap_or(false) {
            debug!(
                "Source '{}' and target '{}' already identical, skipping worktree rebase+squash",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch)?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 2: Check for empty base
        let base_commit_count = Self::get_commit_count(repo, target_branch).unwrap_or(0);
        if base_commit_count <= 1 {
            debug!(
                "Base branch '{}' has {} commit(s), using plain squash merge in worktree",
                target_branch, base_commit_count
            );
            return Self::try_squash_merge_in_worktree(
                repo,
                source_branch,
                target_branch,
                merge_worktree_path,
                commit_message,
            );
        }

        // Step 3: Create rebase worktree on source branch
        Self::checkout_existing_branch_worktree(repo, rebase_worktree_path, source_branch)?;

        // Step 4: Rebase onto target in the worktree
        let rebase_output = Command::new("git")
            .args(["rebase", target_branch])
            .current_dir(rebase_worktree_path)
            .output()
            .map_err(|e| {
                let _ = Self::delete_worktree(repo, rebase_worktree_path);
                AppError::GitOperation(format!("Failed to run git rebase: {}", e))
            })?;

        if !rebase_output.status.success() {
            let stderr = String::from_utf8_lossy(&rebase_output.stderr);
            if stderr.contains("CONFLICT") || stderr.contains("conflict") {
                let conflict_files = Self::get_conflict_files(rebase_worktree_path)?;
                debug!(
                    "Rebase conflict in worktree during rebase+squash, files: {:?}",
                    conflict_files
                );
                // Leave rebase worktree for agent — abort rebase first so agent starts clean
                let _ = Self::abort_rebase(rebase_worktree_path);
                return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
            }
            let _ = Self::delete_worktree(repo, rebase_worktree_path);
            return Err(AppError::GitOperation(format!(
                "Rebase in worktree failed: {}",
                stderr
            )));
        }

        // Step 5: Rebase succeeded — delete rebase worktree
        if let Err(e) = Self::delete_worktree(repo, rebase_worktree_path) {
            debug!("Failed to delete rebase worktree (non-fatal): {}", e);
        }

        // Step 6: Create merge worktree on target, squash merge
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch)?;

        let squash_output = Command::new("git")
            .args(["merge", "--squash", source_branch])
            .current_dir(merge_worktree_path)
            .output()
            .map_err(|e| {
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                AppError::GitOperation(format!(
                    "Failed to squash merge after rebase in worktree: {}",
                    e
                ))
            })?;

        if !squash_output.status.success() {
            let stderr = String::from_utf8_lossy(&squash_output.stderr);
            let _ = Self::delete_worktree(repo, merge_worktree_path);
            return Err(AppError::GitOperation(format!(
                "Squash merge after rebase unexpectedly failed in worktree: {}",
                stderr
            )));
        }

        // Step 7: Commit
        let commit_output = Command::new("git")
            .args(["commit", "-m", commit_message])
            .current_dir(merge_worktree_path)
            .output()
            .map_err(|e| {
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                AppError::GitOperation(format!("Failed to commit rebase+squash in worktree: {}", e))
            })?;

        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout);
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                let sha = Self::get_head_sha(merge_worktree_path)?;
                return Ok(MergeAttemptResult::Success { commit_sha: sha });
            }
            let _ = Self::delete_worktree(repo, merge_worktree_path);
            return Err(AppError::GitOperation(format!(
                "Failed to commit rebase+squash in worktree: stdout={}, stderr={}",
                stdout, stderr
            )));
        }

        let sha = Self::get_head_sha(merge_worktree_path)?;
        debug!("Rebase+squash in worktree succeeded, SHA: {}", sha);
        Ok(MergeAttemptResult::Success { commit_sha: sha })
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

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

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
            debug!("Merge succeeded in worktree, SHA: {}", commit_sha);
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
            debug!("Merge conflict in worktree, files: {:?}", conflict_files);
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

    /// Attempt a rebase-then-merge in isolated worktrees (for Worktree+Rebase strategy)
    ///
    /// 1. Create a worktree on the source branch at `rebase_worktree_path`
    /// 2. Rebase onto target branch in that worktree
    /// 3. On success: delete rebase worktree, create merge worktree on target, fast-forward merge
    /// 4. On conflict: leave rebase worktree in place for agent resolution
    ///
    /// # Arguments
    /// * `repo` - Path to the main git repository
    /// * `source_branch` - Branch to rebase and merge from (e.g., task branch)
    /// * `target_branch` - Branch to merge into (e.g., plan feature branch or main)
    /// * `rebase_worktree_path` - Path for the temporary rebase worktree
    /// * `merge_worktree_path` - Path for the temporary merge worktree
    pub fn try_rebase_and_merge_in_worktree(
        repo: &Path,
        source_branch: &str,
        target_branch: &str,
        rebase_worktree_path: &Path,
        merge_worktree_path: &Path,
    ) -> AppResult<MergeAttemptResult> {
        debug!(
            "Attempting rebase-and-merge of '{}' onto '{}' in worktrees (rebase: {:?}, merge: {:?})",
            source_branch, target_branch, rebase_worktree_path, merge_worktree_path
        );

        // Step 1: Fetch latest from origin (non-fatal)
        match Self::fetch_origin(repo) {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) = Self::validate_merge_branches(repo, source_branch, target_branch) {
            return Ok(not_found);
        }

        // Step 2: Check if base branch is empty (0 or 1 commits)
        // For first task on empty repo, rebase fails. Skip rebase and merge directly.
        let base_commit_count = Self::get_commit_count(repo, target_branch).unwrap_or(0);
        if base_commit_count <= 1 {
            debug!(
                "Target branch '{}' has {} commit(s), falling back to direct worktree merge",
                target_branch, base_commit_count
            );
            return Self::try_merge_in_worktree(
                repo,
                source_branch,
                target_branch,
                merge_worktree_path,
            );
        }

        // Step 3: Create rebase worktree on source branch
        Self::checkout_existing_branch_worktree(repo, rebase_worktree_path, source_branch)?;

        // Step 4: Rebase source onto target in the rebase worktree
        let rebase_result = Self::rebase_onto(rebase_worktree_path, target_branch)?;
        debug!(
            "Rebase result for '{}' onto '{}' in worktree: {:?}",
            source_branch, target_branch, rebase_result
        );

        match rebase_result {
            RebaseResult::Success => {
                // Step 5: Rebase succeeded — delete rebase worktree, then merge
                let _ = Self::delete_worktree(repo, rebase_worktree_path);

                // Step 6: Create merge worktree on target branch and fast-forward
                Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch)?;

                let output = Command::new("git")
                    .args(["merge", source_branch, "--no-edit"])
                    .current_dir(merge_worktree_path)
                    .output()
                    .map_err(|e| {
                        let _ = Self::delete_worktree(repo, merge_worktree_path);
                        AppError::GitOperation(format!("Failed to run git merge: {}", e))
                    })?;

                if output.status.success() {
                    let commit_sha = Self::get_head_sha(merge_worktree_path)?;
                    debug!(
                        "Rebase-and-merge in worktree succeeded, SHA: {}",
                        commit_sha
                    );
                    return Ok(MergeAttemptResult::Success { commit_sha });
                }

                // Check for conflict (shouldn't happen after successful rebase, but handle it)
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stdout.contains("CONFLICT")
                    || stderr.contains("CONFLICT")
                    || stdout.contains("conflict")
                    || stderr.contains("conflict")
                {
                    let conflict_files = Self::get_conflict_files(merge_worktree_path)?;
                    debug!(
                        "Post-rebase merge conflict in worktree (unexpected), files: {:?}",
                        conflict_files
                    );
                    // Leave merge worktree in conflict state for agent
                    return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
                }

                // Unexpected error — clean up merge worktree
                let _ = Self::delete_worktree(repo, merge_worktree_path);
                Err(AppError::GitOperation(format!(
                    "Post-rebase merge of '{}' into '{}' in worktree failed: {}{}",
                    source_branch, target_branch, stderr, stdout
                )))
            }
            RebaseResult::Conflict { files } => {
                // Rebase conflict — leave rebase worktree in place for agent resolution
                debug!("Rebase conflict in worktree, files: {:?}", files);
                Ok(MergeAttemptResult::NeedsAgent {
                    conflict_files: files,
                })
            }
        }
    }
}
