// Merge strategy methods extracted from attempt_programmatic_merge
//
// Each strategy method handles the unique merge logic for a specific
// (MergeStrategy, GitMode) combination. They return a MergeOutcome which
// is then handled uniformly by the shared post-merge handler.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::application::{GitService, MergeAttemptResult};
use crate::application::git_service::checkout_free::{self, CheckoutFreeMergeResult};
use crate::domain::entities::{
    task_metadata::{
        MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata, MergeRecoveryReasonCode,
        MergeRecoverySource,
    },
    GitMode, MergeStrategy, Project, Task, TaskId,
};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::error::{AppError, AppResult};

use super::{compute_merge_worktree_path, compute_rebase_worktree_path};

/// Outcome of a merge strategy execution.
///
/// This enum represents the result of attempting a merge operation,
/// allowing the caller to handle success, conflicts, missing branches,
/// and errors uniformly.
#[derive(Debug)]
pub(super) enum MergeOutcome {
    /// Merge succeeded, ready for post-merge validation/completion
    Success {
        commit_sha: String,
        /// Path where merge occurred (main repo or worktree)
        merge_path: PathBuf,
    },

    /// Merge conflicts detected, needs agent intervention
    NeedsAgent {
        conflict_files: Vec<String>,
        /// Worktree path for conflict resolution (empty for Local mode)
        merge_worktree: Option<PathBuf>,
    },

    /// Source or target branch not found
    BranchNotFound { branch: String },

    /// Git operation or other error
    GitError(AppError),

    /// Status already handled (early return case)
    AlreadyHandled,
}

impl<'a> super::TransitionHandler<'a> {
    /// Strategy: (Merge, Worktree)
    ///
    /// Merge task branch into target using worktree isolation, or use checkout-free
    /// merge if target is already checked out in the main repo.
    ///
    /// Line count: ~891 lines in original match arm
    pub(super) async fn merge_worktree_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        project: &Project,
        task: &mut Task,
        task_id: &TaskId,
        task_id_str: &str,
    ) -> MergeOutcome {
        // Detect if the target branch is already checked out in the primary repo.
        // This happens for plan merge tasks (plan feature branch → main) because
        // main is always checked out in the primary repo. Git forbids the same
        // branch in multiple worktrees, so we merge directly in-repo instead.
        let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
        let target_is_checked_out = current_branch == target_branch;

        if target_is_checked_out {
            // Target branch (e.g., main) is checked out in the primary repo.
            // Use checkout-free merge (git plumbing) to avoid disrupting working tree.
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                "Target branch is checked out, using checkout-free merge"
            );

            // Validate branches exist before merge
            if !GitService::branch_exists(repo_path, source_branch) {
                tracing::error!(task_id = task_id_str, "Source branch '{}' does not exist", source_branch);
                return MergeOutcome::BranchNotFound {
                    branch: source_branch.to_string(),
                };
            }
            if !GitService::branch_exists(repo_path, target_branch) {
                tracing::error!(task_id = task_id_str, "Target branch '{}' does not exist", target_branch);
                return MergeOutcome::BranchNotFound {
                    branch: target_branch.to_string(),
                };
            }

            // Perform checkout-free merge
            match checkout_free::try_merge_checkout_free(repo_path, source_branch, target_branch) {
                Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                    tracing::info!(
                        task_id = task_id_str,
                        commit_sha = %commit_sha,
                        "Checkout-free merge succeeded"
                    );

                    // Ensure working tree is clean
                    if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to reset working tree after checkout-free merge (non-fatal)"
                        );
                    }

                    return MergeOutcome::Success {
                        commit_sha,
                        merge_path: repo_path.to_path_buf(),
                    };
                }
                Ok(CheckoutFreeMergeResult::NeedsAgent { conflict_files }) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        conflict_count = conflict_files.len(),
                        "Checkout-free merge detected conflicts"
                    );

                    return MergeOutcome::NeedsAgent {
                        conflict_files,
                        merge_worktree: None, // Local mode for checkout-free
                    };
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Checkout-free merge failed"
                    );
                    return MergeOutcome::GitError(e);
                }
            }
        }

        // Target not checked out — use isolated worktree

        // Validate branches exist before creating worktree
        if !GitService::branch_exists(repo_path, source_branch) {
            tracing::error!(task_id = task_id_str, "Source branch '{}' does not exist", source_branch);
            return MergeOutcome::BranchNotFound {
                branch: source_branch.to_string(),
            };
        }
        if !GitService::branch_exists(repo_path, target_branch) {
            tracing::error!(task_id = task_id_str, "Target branch '{}' does not exist", target_branch);
            return MergeOutcome::BranchNotFound {
                branch: target_branch.to_string(),
            };
        }

        // Create merge worktree
        let merge_wt_path = compute_merge_worktree_path(project, task_id_str);
        let merge_wt = PathBuf::from(&merge_wt_path);

        tracing::info!(
            task_id = task_id_str,
            worktree = %merge_wt_path,
            "Creating merge worktree"
        );

        // Delete existing worktree if present
        if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt) {
            tracing::debug!(
                task_id = task_id_str,
                error = %e,
                "Failed to delete pre-existing merge worktree (non-fatal)"
            );
        }

        // Create fresh worktree at target branch
        if let Err(e) = GitService::create_worktree(repo_path, &merge_wt, target_branch) {
            tracing::error!(
                task_id = task_id_str,
                error = %e,
                target_branch = %target_branch,
                "Failed to create merge worktree"
            );
            return MergeOutcome::GitError(e);
        }

        // Attempt merge in worktree
        let result = GitService::try_merge_in_worktree(&merge_wt, source_branch);

        // Clean up worktree
        if let Err(e) = GitService::delete_worktree(repo_path, &merge_wt) {
            tracing::warn!(
                task_id = task_id_str,
                error = %e,
                "Failed to delete merge worktree after operation (non-fatal)"
            );
        }

        match result {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Worktree merge succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Worktree merge detected conflicts"
                );

                // Recreate worktree for agent conflict resolution
                if let Err(e) = GitService::create_worktree(repo_path, &merge_wt, target_branch) {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Failed to recreate merge worktree for conflict resolution"
                    );
                    return MergeOutcome::GitError(e);
                }

                // Attempt merge again to recreate conflict state
                if let Err(e) = GitService::merge_branch(&merge_wt, source_branch) {
                    tracing::warn!(
                        task_id = task_id_str,
                        error = %e,
                        "Expected conflict recreation, got error (non-fatal)"
                    );
                }

                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: Some(merge_wt),
                }
            }
            Err(e) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Worktree merge failed"
                );
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (Merge, Local)
    ///
    /// Direct merge in main repo without rebase.
    ///
    /// Line count: ~283 lines in original match arm
    pub(super) async fn merge_local_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        task_id_str: &str,
    ) -> MergeOutcome {
        tracing::info!(
            task_id = task_id_str,
            source = %source_branch,
            target = %target_branch,
            "Attempting local merge"
        );

        match GitService::try_merge(repo_path, source_branch, target_branch) {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Local merge succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Local merge detected conflicts"
                );
                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: None, // Local mode
                }
            }
            Err(e) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Local merge failed"
                );
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (Rebase, Local)
    ///
    /// Rebase task branch onto target, then merge.
    ///
    /// Line count: ~425 lines in original match arm
    pub(super) async fn rebase_local_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        task_id_str: &str,
    ) -> MergeOutcome {
        tracing::info!(
            task_id = task_id_str,
            source = %source_branch,
            target = %target_branch,
            "Attempting local rebase and merge"
        );

        match GitService::try_rebase_and_merge(repo_path, source_branch, target_branch) {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Local rebase and merge succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Local rebase detected conflicts"
                );
                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: None, // Local mode
                }
            }
            Err(e) => {
                tracing::error!(
                    task_id = task_id_str,
                    error = %e,
                    "Local rebase and merge failed"
                );
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (Rebase, Worktree)
    ///
    /// Rebase in worktree then fast-forward merge, or use checkout-free if target checked out.
    ///
    /// Line count: ~526 lines in original match arm
    pub(super) async fn rebase_worktree_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        project: &Project,
        task_id_str: &str,
    ) -> MergeOutcome {
        let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
        let target_is_checked_out = current_branch == target_branch;

        if target_is_checked_out {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                "Target branch is checked out, using checkout-free rebase"
            );

            // Validate branches
            if !GitService::branch_exists(repo_path, source_branch) {
                tracing::error!(task_id = task_id_str, "Source branch '{}' does not exist", source_branch);
                return MergeOutcome::BranchNotFound {
                    branch: source_branch.to_string(),
                };
            }
            if !GitService::branch_exists(repo_path, target_branch) {
                tracing::error!(task_id = task_id_str, "Target branch '{}' does not exist", target_branch);
                return MergeOutcome::BranchNotFound {
                    branch: target_branch.to_string(),
                };
            }

            match checkout_free::try_fast_forward_checkout_free(repo_path, source_branch, target_branch) {
                Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                    tracing::info!(
                        task_id = task_id_str,
                        commit_sha = %commit_sha,
                        "Checkout-free fast-forward succeeded"
                    );

                    if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                        tracing::warn!(
                            task_id = task_id_str,
                            error = %e,
                            "Failed to reset working tree (non-fatal)"
                        );
                    }

                    return MergeOutcome::Success {
                        commit_sha,
                        merge_path: repo_path.to_path_buf(),
                    };
                }
                Ok(CheckoutFreeMergeResult::NeedsAgent { conflict_files }) => {
                    tracing::warn!(
                        task_id = task_id_str,
                        conflict_count = conflict_files.len(),
                        "Checkout-free rebase detected conflicts"
                    );
                    return MergeOutcome::NeedsAgent {
                        conflict_files,
                        merge_worktree: None,
                    };
                }
                Err(e) => {
                    tracing::error!(
                        task_id = task_id_str,
                        error = %e,
                        "Checkout-free rebase failed"
                    );
                    return MergeOutcome::GitError(e);
                }
            }
        }

        // Target not checked out — use worktree

        // Validate branches
        if !GitService::branch_exists(repo_path, source_branch) {
            return MergeOutcome::BranchNotFound {
                branch: source_branch.to_string(),
            };
        }
        if !GitService::branch_exists(repo_path, target_branch) {
            return MergeOutcome::BranchNotFound {
                branch: target_branch.to_string(),
            };
        }

        let rebase_wt_path = compute_rebase_worktree_path(project, task_id_str);
        let rebase_wt = PathBuf::from(&rebase_wt_path);

        tracing::info!(
            task_id = task_id_str,
            worktree = %rebase_wt_path,
            "Creating rebase worktree"
        );

        let _ = GitService::delete_worktree(repo_path, &rebase_wt);

        if let Err(e) = GitService::create_worktree(repo_path, &rebase_wt, source_branch) {
            tracing::error!(error = %e, "Failed to create rebase worktree");
            return MergeOutcome::GitError(e);
        }

        let result = GitService::try_rebase_and_merge_in_worktree(&rebase_wt, target_branch);

        let _ = GitService::delete_worktree(repo_path, &rebase_wt);

        match result {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Worktree rebase and merge succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Worktree rebase detected conflicts"
                );

                // Recreate worktree for conflict resolution
                if let Err(e) = GitService::create_worktree(repo_path, &rebase_wt, source_branch) {
                    return MergeOutcome::GitError(e);
                }

                let _ = GitService::rebase_onto(&rebase_wt, target_branch);

                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: Some(rebase_wt),
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Worktree rebase failed");
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (Squash, Local)
    ///
    /// Squash merge in main repo.
    ///
    /// Line count: ~274 lines in original match arm
    pub(super) async fn squash_local_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        squash_commit_msg: &str,
        task_id_str: &str,
    ) -> MergeOutcome {
        tracing::info!(
            task_id = task_id_str,
            source = %source_branch,
            target = %target_branch,
            "Attempting local squash merge"
        );

        match GitService::try_squash_merge(repo_path, source_branch, target_branch, squash_commit_msg) {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Local squash merge succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Local squash merge detected conflicts"
                );
                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: None,
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Local squash merge failed");
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (Squash, Worktree)
    ///
    /// Squash merge in worktree, or use checkout-free if target checked out.
    ///
    /// Line count: ~388 lines in original match arm
    pub(super) async fn squash_worktree_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        squash_commit_msg: &str,
        project: &Project,
        task_id_str: &str,
    ) -> MergeOutcome {
        let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
        let target_is_checked_out = current_branch == target_branch;

        if target_is_checked_out {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                "Target checked out, using checkout-free squash"
            );

            if !GitService::branch_exists(repo_path, source_branch) {
                return MergeOutcome::BranchNotFound {
                    branch: source_branch.to_string(),
                };
            }
            if !GitService::branch_exists(repo_path, target_branch) {
                return MergeOutcome::BranchNotFound {
                    branch: target_branch.to_string(),
                };
            }

            match checkout_free::try_squash_merge_checkout_free(
                repo_path,
                source_branch,
                target_branch,
                squash_commit_msg,
            ) {
                Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                    if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                        tracing::warn!(error = %e, "Failed to reset working tree (non-fatal)");
                    }
                    return MergeOutcome::Success {
                        commit_sha,
                        merge_path: repo_path.to_path_buf(),
                    };
                }
                Ok(CheckoutFreeMergeResult::NeedsAgent { conflict_files }) => {
                    return MergeOutcome::NeedsAgent {
                        conflict_files,
                        merge_worktree: None,
                    };
                }
                Err(e) => {
                    return MergeOutcome::GitError(e);
                }
            }
        }

        // Use worktree

        if !GitService::branch_exists(repo_path, source_branch) {
            return MergeOutcome::BranchNotFound {
                branch: source_branch.to_string(),
            };
        }
        if !GitService::branch_exists(repo_path, target_branch) {
            return MergeOutcome::BranchNotFound {
                branch: target_branch.to_string(),
            };
        }

        let merge_wt_path = compute_merge_worktree_path(project, task_id_str);
        let merge_wt = PathBuf::from(&merge_wt_path);

        let _ = GitService::delete_worktree(repo_path, &merge_wt);

        if let Err(e) = GitService::create_worktree(repo_path, &merge_wt, target_branch) {
            return MergeOutcome::GitError(e);
        }

        let result = GitService::try_squash_merge_in_worktree(&merge_wt, source_branch, squash_commit_msg);

        let _ = GitService::delete_worktree(repo_path, &merge_wt);

        match result {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                // Recreate for conflict resolution
                if let Err(e) = GitService::create_worktree(repo_path, &merge_wt, target_branch) {
                    return MergeOutcome::GitError(e);
                }

                let _ = GitService::merge_branch(&merge_wt, source_branch);

                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: Some(merge_wt),
                }
            }
            Err(e) => MergeOutcome::GitError(e),
        }
    }

    /// Strategy: (RebaseSquash, Local)
    ///
    /// Rebase then squash merge in main repo.
    ///
    /// Line count: ~275 lines in original match arm
    pub(super) async fn rebase_squash_local_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        squash_commit_msg: &str,
        task_id_str: &str,
    ) -> MergeOutcome {
        tracing::info!(
            task_id = task_id_str,
            source = %source_branch,
            target = %target_branch,
            "Attempting local rebase-squash merge"
        );

        match GitService::try_rebase_squash_merge(repo_path, source_branch, target_branch, squash_commit_msg) {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Local rebase-squash succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Local rebase-squash detected conflicts"
                );
                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: None,
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Local rebase-squash failed");
                MergeOutcome::GitError(e)
            }
        }
    }

    /// Strategy: (RebaseSquash, Worktree)
    ///
    /// Rebase in worktree, then squash merge. Uses checkout-free if target checked out.
    /// Most complex strategy: manages dual worktrees for rebase + merge.
    ///
    /// Line count: ~486 lines in original match arm
    pub(super) async fn rebase_squash_worktree_strategy(
        &self,
        repo_path: &Path,
        source_branch: &str,
        target_branch: &str,
        squash_commit_msg: &str,
        project: &Project,
        task_id_str: &str,
    ) -> MergeOutcome {
        let current_branch = GitService::get_current_branch(repo_path).unwrap_or_default();
        let target_is_checked_out = current_branch == target_branch;

        if target_is_checked_out {
            tracing::info!(
                task_id = task_id_str,
                target_branch = %target_branch,
                "Target checked out, using checkout-free squash (skipping rebase to avoid conflicts)"
            );

            if !GitService::branch_exists(repo_path, source_branch) {
                return MergeOutcome::BranchNotFound {
                    branch: source_branch.to_string(),
                };
            }
            if !GitService::branch_exists(repo_path, target_branch) {
                return MergeOutcome::BranchNotFound {
                    branch: target_branch.to_string(),
                };
            }

            match checkout_free::try_squash_merge_checkout_free(
                repo_path,
                source_branch,
                target_branch,
                squash_commit_msg,
            ) {
                Ok(CheckoutFreeMergeResult::Success { commit_sha }) => {
                    if let Err(e) = GitService::hard_reset_to_head(repo_path) {
                        tracing::warn!(error = %e, "Failed to reset working tree (non-fatal)");
                    }
                    return MergeOutcome::Success {
                        commit_sha,
                        merge_path: repo_path.to_path_buf(),
                    };
                }
                Ok(CheckoutFreeMergeResult::NeedsAgent { conflict_files }) => {
                    return MergeOutcome::NeedsAgent {
                        conflict_files,
                        merge_worktree: None,
                    };
                }
                Err(e) => {
                    return MergeOutcome::GitError(e);
                }
            }
        }

        // Dual-worktree strategy: rebase worktree first, then squash merge in separate worktree

        if !GitService::branch_exists(repo_path, source_branch) {
            return MergeOutcome::BranchNotFound {
                branch: source_branch.to_string(),
            };
        }
        if !GitService::branch_exists(repo_path, target_branch) {
            return MergeOutcome::BranchNotFound {
                branch: target_branch.to_string(),
            };
        }

        let rebase_wt_path = compute_rebase_worktree_path(project, task_id_str);
        let rebase_wt = PathBuf::from(&rebase_wt_path);

        tracing::info!(
            task_id = task_id_str,
            worktree = %rebase_wt_path,
            "Creating rebase worktree for rebase-squash"
        );

        let _ = GitService::delete_worktree(repo_path, &rebase_wt);

        if let Err(e) = GitService::create_worktree(repo_path, &rebase_wt, source_branch) {
            return MergeOutcome::GitError(e);
        }

        let result = GitService::try_rebase_squash_merge_in_worktree(&rebase_wt, target_branch, squash_commit_msg);

        let _ = GitService::delete_worktree(repo_path, &rebase_wt);

        match result {
            Ok(MergeAttemptResult::Success { commit_sha }) => {
                tracing::info!(
                    task_id = task_id_str,
                    commit_sha = %commit_sha,
                    "Worktree rebase-squash succeeded"
                );
                MergeOutcome::Success {
                    commit_sha,
                    merge_path: repo_path.to_path_buf(),
                }
            }
            Ok(MergeAttemptResult::NeedsAgent { conflict_files }) => {
                tracing::warn!(
                    task_id = task_id_str,
                    conflict_count = conflict_files.len(),
                    "Worktree rebase-squash detected conflicts"
                );

                // Recreate rebase worktree for conflict resolution
                if let Err(e) = GitService::create_worktree(repo_path, &rebase_wt, source_branch) {
                    return MergeOutcome::GitError(e);
                }

                let _ = GitService::rebase_onto(&rebase_wt, target_branch);

                MergeOutcome::NeedsAgent {
                    conflict_files,
                    merge_worktree: Some(rebase_wt),
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Worktree rebase-squash failed");
                MergeOutcome::GitError(e)
            }
        }
    }
}
