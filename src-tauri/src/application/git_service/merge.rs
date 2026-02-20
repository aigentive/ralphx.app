//! Merge operations for the two-phase merge workflow
//!
//! Extracted from `mod.rs` — contains merge, squash merge, and
//! worktree-isolated merge operations. Rebase operations are in `rebase.rs`.

use super::git_cmd;
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
    pub async fn merge_branch(
        repo: &Path,
        source: &str,
        _target: &str,
    ) -> AppResult<MergeResult> {
        debug!("Merging branch '{}' in {:?}", source, repo);

        let output = git_cmd::run(&["merge", source, "--no-edit"], repo).await?;

        if output.status.success() {
            let sha = Self::get_head_sha(repo).await?;
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
            let conflict_files = Self::get_conflict_files(repo).await?;
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
    pub async fn abort_merge(repo: &Path) -> AppResult<()> {
        debug!("Aborting merge in {:?}", repo);

        let output = git_cmd::run(&["merge", "--abort"], repo).await?;

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

    /// Squash merge in an isolated worktree
    ///
    /// Creates a temporary worktree on the target branch, squash-merges the source
    /// branch, and commits. Avoids disrupting the main repo working directory.
    ///
    /// - On **success**: returns commit SHA. Caller should clean up the worktree.
    /// - On **conflict**: leaves worktree in conflict state for agent resolution.
    /// - On **error**: cleans up worktree and returns error.
    pub async fn try_squash_merge_in_worktree(
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
        if let Some(not_found) =
            Self::validate_merge_branches(repo, source_branch, target_branch).await
        {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch)
            .await
            .unwrap_or(false)
        {
            debug!(
                "Source '{}' and target '{}' already identical, skipping worktree merge",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch).await?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 1: Create worktree on target branch
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch).await?;

        // Step 2: Squash merge source into worktree
        let output = match git_cmd::run(
            &["merge", "--squash", source_branch],
            merge_worktree_path,
        )
        .await
        {
            Ok(output) => output,
            Err(e) => {
                let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                return Err(e);
            }
        };

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stdout.contains("CONFLICT")
                || stderr.contains("CONFLICT")
                || stdout.contains("conflict")
                || stderr.contains("conflict")
            {
                let conflict_files = Self::get_conflict_files(merge_worktree_path).await?;
                debug!(
                    "Squash merge conflict in worktree, files: {:?}",
                    conflict_files
                );
                // Leave worktree in conflict state for agent resolution
                return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
            }

            let _ = Self::delete_worktree(repo, merge_worktree_path).await;
            return Err(AppError::GitOperation(format!(
                "Squash merge of '{}' into '{}' in worktree failed: {}{}",
                source_branch, target_branch, stderr, stdout
            )));
        }

        // Step 3: Commit the squashed changes in the worktree
        let commit_output =
            match git_cmd::run(&["commit", "-m", commit_message], merge_worktree_path).await {
                Ok(output) => output,
                Err(e) => {
                    let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                    return Err(e);
                }
            };

        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout);
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                let commit_sha = Self::get_head_sha(merge_worktree_path).await?;
                debug!(
                    "Squash merge no-op in worktree (branches identical), SHA: {}",
                    commit_sha
                );
                return Ok(MergeAttemptResult::Success { commit_sha });
            }
            let _ = Self::delete_worktree(repo, merge_worktree_path).await;
            return Err(AppError::GitOperation(format!(
                "Failed to commit squash merge in worktree: stdout={}, stderr={}",
                stdout, stderr
            )));
        }

        let commit_sha = Self::get_head_sha(merge_worktree_path).await?;
        debug!("Squash merge in worktree succeeded, SHA: {}", commit_sha);
        Ok(MergeAttemptResult::Success { commit_sha })
    }

    /// Rebase source onto target in a worktree, then squash into a single commit
    ///
    /// 1. Create rebase worktree on source, rebase onto target
    /// 2. On success: delete rebase wt, create merge wt on target, squash merge, commit
    /// 3. On conflict: leave rebase worktree for agent resolution
    pub async fn try_rebase_squash_merge_in_worktree(
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
        match Self::fetch_origin(repo).await {
            Ok(_) => debug!("Fetch from origin succeeded"),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) =
            Self::validate_merge_branches(repo, source_branch, target_branch).await
        {
            return Ok(not_found);
        }

        // Early return: if branches are already identical, skip merge entirely
        if Self::branches_have_same_content(repo, source_branch, target_branch)
            .await
            .unwrap_or(false)
        {
            debug!(
                "Source '{}' and target '{}' already identical, skipping worktree rebase+squash",
                source_branch, target_branch
            );
            let commit_sha = Self::get_branch_sha(repo, target_branch).await?;
            return Ok(MergeAttemptResult::Success { commit_sha });
        }

        // Step 2: Check for empty base
        let base_commit_count = Self::get_commit_count(repo, target_branch)
            .await
            .unwrap_or(0);
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
            )
            .await;
        }

        // Step 3: Create rebase worktree on source branch
        Self::checkout_existing_branch_worktree(repo, rebase_worktree_path, source_branch).await?;

        // Step 4: Rebase onto target in the worktree
        let rebase_output =
            match git_cmd::run(&["rebase", target_branch], rebase_worktree_path).await {
                Ok(output) => output,
                Err(e) => {
                    let _ = Self::delete_worktree(repo, rebase_worktree_path).await;
                    return Err(e);
                }
            };

        if !rebase_output.status.success() {
            let stderr = String::from_utf8_lossy(&rebase_output.stderr);
            if stderr.contains("CONFLICT") || stderr.contains("conflict") {
                let conflict_files = Self::get_conflict_files(rebase_worktree_path).await?;
                debug!(
                    "Rebase conflict in worktree during rebase+squash, files: {:?}",
                    conflict_files
                );
                // Leave rebase worktree for agent — abort rebase first so agent starts clean
                let _ = Self::abort_rebase(rebase_worktree_path).await;
                return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
            }
            let _ = Self::delete_worktree(repo, rebase_worktree_path).await;
            return Err(AppError::GitOperation(format!(
                "Rebase in worktree failed: {}",
                stderr
            )));
        }

        // Step 5: Rebase succeeded — delete rebase worktree
        if let Err(e) = Self::delete_worktree(repo, rebase_worktree_path).await {
            debug!("Failed to delete rebase worktree (non-fatal): {}", e);
        }

        // Step 6: Create merge worktree on target, squash merge
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch).await?;

        let squash_output = match git_cmd::run(
            &["merge", "--squash", source_branch],
            merge_worktree_path,
        )
        .await
        {
            Ok(output) => output,
            Err(e) => {
                let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                return Err(e);
            }
        };

        if !squash_output.status.success() {
            let stderr = String::from_utf8_lossy(&squash_output.stderr);
            let _ = Self::delete_worktree(repo, merge_worktree_path).await;
            return Err(AppError::GitOperation(format!(
                "Squash merge after rebase unexpectedly failed in worktree: {}",
                stderr
            )));
        }

        // Step 7: Commit
        let commit_output =
            match git_cmd::run(&["commit", "-m", commit_message], merge_worktree_path).await {
                Ok(output) => output,
                Err(e) => {
                    let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                    return Err(e);
                }
            };

        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout);
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
                let sha = Self::get_head_sha(merge_worktree_path).await?;
                return Ok(MergeAttemptResult::Success { commit_sha: sha });
            }
            let _ = Self::delete_worktree(repo, merge_worktree_path).await;
            return Err(AppError::GitOperation(format!(
                "Failed to commit rebase+squash in worktree: stdout={}, stderr={}",
                stdout, stderr
            )));
        }

        let sha = Self::get_head_sha(merge_worktree_path).await?;
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
    pub async fn try_merge_in_worktree(
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
        if let Some(not_found) =
            Self::validate_merge_branches(repo, source_branch, target_branch).await
        {
            return Ok(not_found);
        }

        // Step 1: Create merge worktree checking out the target branch
        Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch).await?;

        // Step 2: Merge source branch into the merge worktree
        let output = match git_cmd::run(
            &["merge", source_branch, "--no-edit"],
            merge_worktree_path,
        )
        .await
        {
            Ok(output) => output,
            Err(e) => {
                // Clean up worktree on command execution failure
                let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                return Err(e);
            }
        };

        if output.status.success() {
            let commit_sha = Self::get_head_sha(merge_worktree_path).await?;
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
            let conflict_files = Self::get_conflict_files(merge_worktree_path).await?;
            debug!("Merge conflict in worktree, files: {:?}", conflict_files);
            // Do NOT abort — leave worktree in conflict state for agent resolution
            return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
        }

        // Unexpected error — clean up worktree
        let _ = Self::delete_worktree(repo, merge_worktree_path).await;
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
    pub async fn try_rebase_and_merge_in_worktree(
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
        match Self::fetch_origin(repo).await {
            Ok(_) => debug!("Fetch from origin succeeded for {:?}", repo),
            Err(e) => debug!("Fetch from origin failed (non-fatal): {}", e),
        }

        // Validate branches exist before attempting merge
        if let Some(not_found) =
            Self::validate_merge_branches(repo, source_branch, target_branch).await
        {
            return Ok(not_found);
        }

        // Step 2: Check if base branch is empty (0 or 1 commits)
        // For first task on empty repo, rebase fails. Skip rebase and merge directly.
        let base_commit_count = Self::get_commit_count(repo, target_branch)
            .await
            .unwrap_or(0);
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
            )
            .await;
        }

        // Step 3: Create rebase worktree on source branch
        Self::checkout_existing_branch_worktree(repo, rebase_worktree_path, source_branch).await?;

        // Step 4: Rebase source onto target in the rebase worktree
        let rebase_result = Self::rebase_onto(rebase_worktree_path, target_branch).await?;
        debug!(
            "Rebase result for '{}' onto '{}' in worktree: {:?}",
            source_branch, target_branch, rebase_result
        );

        match rebase_result {
            RebaseResult::Success => {
                // Step 5: Rebase succeeded — delete rebase worktree, then merge
                let _ = Self::delete_worktree(repo, rebase_worktree_path).await;

                // Step 6: Create merge worktree on target branch and fast-forward
                Self::checkout_existing_branch_worktree(repo, merge_worktree_path, target_branch)
                    .await?;

                let output = match git_cmd::run(
                    &["merge", source_branch, "--no-edit"],
                    merge_worktree_path,
                )
                .await
                {
                    Ok(output) => output,
                    Err(e) => {
                        let _ = Self::delete_worktree(repo, merge_worktree_path).await;
                        return Err(e);
                    }
                };

                if output.status.success() {
                    let commit_sha = Self::get_head_sha(merge_worktree_path).await?;
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
                    let conflict_files = Self::get_conflict_files(merge_worktree_path).await?;
                    debug!(
                        "Post-rebase merge conflict in worktree (unexpected), files: {:?}",
                        conflict_files
                    );
                    // Leave merge worktree in conflict state for agent
                    return Ok(MergeAttemptResult::NeedsAgent { conflict_files });
                }

                // Unexpected error — clean up merge worktree
                let _ = Self::delete_worktree(repo, merge_worktree_path).await;
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
