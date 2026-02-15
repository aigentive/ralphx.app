//! Checkout-free merge operations using git plumbing commands
//!
//! Performs merges without touching the working tree by using `git merge-tree`,
//! `git commit-tree`, and `git update-ref`. The working tree is only updated
//! once at the end via `git reset --hard HEAD` (a single atomic file update).
//!
//! Requires Git 2.38+ for `git merge-tree --write-tree`.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Result of a checkout-free merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckoutFreeMergeResult {
    /// Merge succeeded — branch ref updated, working tree NOT yet synced
    Success { commit_sha: String },
    /// Merge has conflicts — caller must handle (e.g. create temp worktree)
    Conflict { files: Vec<PathBuf> },
}

// =============================================================================
// Low-level plumbing primitives (never touch working tree)
// =============================================================================

/// Run `git merge-tree --write-tree <target> <source>` to compute a merged tree.
///
/// Returns `Ok(Ok(tree_sha))` on clean merge, `Ok(Err(files))` on conflicts,
/// or `Err(AppError)` if the git command fails to spawn.
pub fn merge_tree_write(
    repo: &Path,
    target_ref: &str,
    source_ref: &str,
) -> AppResult<Result<String, Vec<PathBuf>>> {
    let output = Command::new("git")
        .args(["merge-tree", "--write-tree", target_ref, source_ref])
        .current_dir(repo)
        .output()
        .map_err(|e| AppError::GitOperation(format!("Failed to run git merge-tree: {}", e)))?;

    if output.status.success() {
        let tree_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("merge-tree --write-tree succeeded: {}", tree_sha);
        Ok(Ok(tree_sha))
    } else {
        // Exit code 1 with --write-tree means conflicts.
        // Conflict info (CONFLICT lines) appears on stdout after the tree SHA
        // and file listing. Parse from stdout.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let files = parse_merge_tree_conflicts(&stdout);
        debug!("merge-tree --write-tree found {} conflicts", files.len());
        Ok(Err(files))
    }
}

/// Parse conflict file paths from `git merge-tree --write-tree` stdout output.
///
/// Stdout contains the tree SHA, file listings, then CONFLICT lines:
///   CONFLICT (content): Merge conflict in path/to/file.rs
///   CONFLICT (add/add): Merge conflict in another/file.ts
fn parse_merge_tree_conflicts(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter(|line| line.starts_with("CONFLICT"))
        .filter_map(|line| {
            // Extract path after "Merge conflict in " or after the last space for other formats
            if let Some(path) = line.strip_suffix(|_: char| false).or(Some(line)) {
                if let Some(idx) = path.find("Merge conflict in ") {
                    let file_path = &path[idx + "Merge conflict in ".len()..];
                    return Some(PathBuf::from(file_path.trim()));
                }
                // Handle "CONFLICT (rename/delete): path deleted in ..." format
                // and other CONFLICT formats - extract the first file path mentioned
            }
            None
        })
        .collect()
}

/// Run `git commit-tree <tree_sha> -p <parent1> [-p <parent2>] -m <message>`
///
/// Creates a commit object without touching HEAD or the working tree.
/// Returns the new commit SHA.
pub fn commit_tree(
    repo: &Path,
    tree_sha: &str,
    parents: &[&str],
    message: &str,
) -> AppResult<String> {
    let mut args = vec!["commit-tree".to_string(), tree_sha.to_string()];
    for parent in parents {
        args.push("-p".to_string());
        args.push(parent.to_string());
    }
    args.push("-m".to_string());
    args.push(message.to_string());

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo)
        .output()
        .map_err(|e| AppError::GitOperation(format!("Failed to run git commit-tree: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::GitOperation(format!(
            "git commit-tree failed: {}",
            stderr
        )));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    debug!("commit-tree created commit: {}", sha);
    Ok(sha)
}

/// Run `git update-ref refs/heads/<branch> <new_sha>`
///
/// Advances (or sets) the branch pointer without touching HEAD or the working tree.
pub fn update_branch_ref(repo: &Path, branch: &str, new_sha: &str) -> AppResult<()> {
    let refname = format!("refs/heads/{}", branch);
    let output = Command::new("git")
        .args(["update-ref", &refname, new_sha])
        .current_dir(repo)
        .output()
        .map_err(|e| AppError::GitOperation(format!("Failed to run git update-ref: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::GitOperation(format!(
            "git update-ref {} {} failed: {}",
            refname, new_sha, stderr
        )));
    }

    debug!("update-ref {} → {}", refname, new_sha);
    Ok(())
}

// =============================================================================
// High-level checkout-free merge operations
// =============================================================================

/// Checkout-free regular merge (two-parent merge commit).
///
/// 1. `merge-tree --write-tree` to compute the merged tree
/// 2. `commit-tree` with two parents (target, source) to create merge commit
/// 3. `update-ref` to advance the target branch
///
/// Does NOT touch the working tree. Caller must `git reset --hard HEAD` to sync.
pub fn try_merge_checkout_free(
    repo: &Path,
    source_branch: &str,
    target_branch: &str,
) -> AppResult<CheckoutFreeMergeResult> {
    debug!("Checkout-free merge: {} → {}", source_branch, target_branch);

    let target_sha = super::GitService::get_branch_sha(repo, target_branch)?;
    let source_sha = super::GitService::get_branch_sha(repo, source_branch)?;

    match merge_tree_write(repo, target_branch, source_branch)? {
        Ok(tree_sha) => {
            let message = format!("Merge branch '{}' into {}", source_branch, target_branch);
            let commit_sha = commit_tree(repo, &tree_sha, &[&target_sha, &source_sha], &message)?;
            update_branch_ref(repo, target_branch, &commit_sha)?;
            debug!(
                "Checkout-free merge succeeded: {} → {} = {}",
                source_branch, target_branch, commit_sha
            );
            Ok(CheckoutFreeMergeResult::Success { commit_sha })
        }
        Err(files) => {
            debug!("Checkout-free merge has {} conflicts", files.len());
            Ok(CheckoutFreeMergeResult::Conflict { files })
        }
    }
}

/// Checkout-free squash merge (single-parent commit).
///
/// 1. `merge-tree --write-tree` to compute the merged tree
/// 2. `commit-tree` with one parent (target only) to create squash commit
/// 3. `update-ref` to advance the target branch
///
/// Does NOT touch the working tree. Caller must `git reset --hard HEAD` to sync.
pub fn try_squash_merge_checkout_free(
    repo: &Path,
    source_branch: &str,
    target_branch: &str,
    commit_message: &str,
) -> AppResult<CheckoutFreeMergeResult> {
    debug!(
        "Checkout-free squash merge: {} → {}",
        source_branch, target_branch
    );

    let target_sha = super::GitService::get_branch_sha(repo, target_branch)?;

    match merge_tree_write(repo, target_branch, source_branch)? {
        Ok(tree_sha) => {
            let commit_sha = commit_tree(repo, &tree_sha, &[&target_sha], commit_message)?;
            update_branch_ref(repo, target_branch, &commit_sha)?;
            debug!(
                "Checkout-free squash merge succeeded: {} → {} = {}",
                source_branch, target_branch, commit_sha
            );
            Ok(CheckoutFreeMergeResult::Success { commit_sha })
        }
        Err(files) => {
            debug!("Checkout-free squash merge has {} conflicts", files.len());
            Ok(CheckoutFreeMergeResult::Conflict { files })
        }
    }
}

/// Checkout-free fast-forward "merge" (just advance the ref).
///
/// Verifies that target is an ancestor of source (i.e. FF is possible),
/// then advances target ref to source's commit.
///
/// Does NOT touch the working tree. Caller must `git reset --hard HEAD` to sync.
pub fn try_fast_forward_checkout_free(
    repo: &Path,
    source_branch: &str,
    target_branch: &str,
) -> AppResult<CheckoutFreeMergeResult> {
    debug!(
        "Checkout-free fast-forward: {} → {}",
        source_branch, target_branch
    );

    let target_sha = super::GitService::get_branch_sha(repo, target_branch)?;
    let source_sha = super::GitService::get_branch_sha(repo, source_branch)?;

    // Check if target is ancestor of source (FF possible)
    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", &target_sha, &source_sha])
        .current_dir(repo)
        .output()
        .map_err(|e| {
            AppError::GitOperation(format!("Failed to run git merge-base --is-ancestor: {}", e))
        })?;

    if !output.status.success() {
        // Not a fast-forward — fall back to regular merge
        debug!(
            "Cannot fast-forward {} → {}, target is not ancestor of source",
            source_branch, target_branch
        );
        return try_merge_checkout_free(repo, source_branch, target_branch);
    }

    // Fast-forward: just advance the ref
    update_branch_ref(repo, target_branch, &source_sha)?;
    debug!(
        "Checkout-free fast-forward succeeded: {} → {} = {}",
        source_branch, target_branch, source_sha
    );
    Ok(CheckoutFreeMergeResult::Success {
        commit_sha: source_sha,
    })
}

#[cfg(test)]
#[path = "checkout_free_tests.rs"]
mod tests;
