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
/// Returns `Ok(tree_sha)` on clean merge, or `Err` with conflict file list.
pub fn merge_tree_write(
    repo: &Path,
    target_ref: &str,
    source_ref: &str,
) -> Result<String, Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["merge-tree", "--write-tree", target_ref, source_ref])
        .current_dir(repo)
        .output()
        .expect("Failed to execute git merge-tree");

    if output.status.success() {
        let tree_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("merge-tree --write-tree succeeded: {}", tree_sha);
        Ok(tree_sha)
    } else {
        // Exit code 1 with --write-tree means conflicts.
        // Conflict info (CONFLICT lines) appears on stdout after the tree SHA
        // and file listing. Parse from stdout.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let files = parse_merge_tree_conflicts(&stdout);
        debug!(
            "merge-tree --write-tree found {} conflicts",
            files.len()
        );
        Err(files)
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
    debug!(
        "Checkout-free merge: {} → {}",
        source_branch, target_branch
    );

    let target_sha = super::GitService::get_branch_sha(repo, target_branch)?;
    let source_sha = super::GitService::get_branch_sha(repo, source_branch)?;

    match merge_tree_write(repo, target_branch, source_branch) {
        Ok(tree_sha) => {
            let message = format!("Merge branch '{}' into {}", source_branch, target_branch);
            let commit_sha =
                commit_tree(repo, &tree_sha, &[&target_sha, &source_sha], &message)?;
            update_branch_ref(repo, target_branch, &commit_sha)?;
            debug!(
                "Checkout-free merge succeeded: {} → {} = {}",
                source_branch, target_branch, commit_sha
            );
            Ok(CheckoutFreeMergeResult::Success { commit_sha })
        }
        Err(files) => {
            debug!(
                "Checkout-free merge has {} conflicts",
                files.len()
            );
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

    match merge_tree_write(repo, target_branch, source_branch) {
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
            debug!(
                "Checkout-free squash merge has {} conflicts",
                files.len()
            );
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temp git repo with an initial commit, returns the repo path
    fn setup_test_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let repo = dir.path();

        // Init repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .expect("git init failed");

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(repo)
            .output()
            .expect("git config email failed");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()
            .expect("git config name failed");

        // Create initial commit on main
        fs::write(repo.join("README.md"), "# Test Repo\n").expect("write failed");
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .expect("git add failed");
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo)
            .output()
            .expect("git commit failed");

        // Ensure we're on 'main'
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo)
            .output()
            .expect("git branch -M main failed");

        dir
    }

    /// Create a branch with a file change
    fn create_branch_with_change(repo: &Path, branch: &str, filename: &str, content: &str) {
        Command::new("git")
            .args(["checkout", "-b", branch])
            .current_dir(repo)
            .output()
            .expect("git checkout -b failed");

        fs::write(repo.join(filename), content).expect("write failed");
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .expect("git add failed");
        Command::new("git")
            .args(["commit", "-m", &format!("Add {}", filename)])
            .current_dir(repo)
            .output()
            .expect("git commit failed");

        // Go back to main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .expect("git checkout main failed");
    }

    #[test]
    fn test_merge_tree_write_clean_merge() {
        let dir = setup_test_repo();
        let repo = dir.path();

        create_branch_with_change(repo, "feature", "feature.txt", "feature content\n");

        let result = merge_tree_write(repo, "main", "feature");
        assert!(result.is_ok(), "Expected clean merge, got: {:?}", result);
        let tree_sha = result.unwrap();
        assert!(!tree_sha.is_empty());
    }

    #[test]
    fn test_merge_tree_write_conflict() {
        let dir = setup_test_repo();
        let repo = dir.path();

        // Create conflicting changes on two branches
        create_branch_with_change(repo, "branch-a", "shared.txt", "content from branch-a\n");
        create_branch_with_change(repo, "branch-b", "shared.txt", "content from branch-b\n");

        // Merge branch-a into main first
        Command::new("git")
            .args(["merge", "branch-a", "--no-edit"])
            .current_dir(repo)
            .output()
            .expect("git merge failed");

        // Now try merge-tree with branch-b → should conflict
        let result = merge_tree_write(repo, "main", "branch-b");
        assert!(result.is_err(), "Expected conflict, got: {:?}", result);
        let files = result.unwrap_err();
        assert!(!files.is_empty());
    }

    #[test]
    fn test_commit_tree_creates_commit() {
        let dir = setup_test_repo();
        let repo = dir.path();

        // Get the tree SHA of HEAD
        let tree_output = Command::new("git")
            .args(["rev-parse", "HEAD^{tree}"])
            .current_dir(repo)
            .output()
            .expect("git rev-parse failed");
        let tree_sha = String::from_utf8_lossy(&tree_output.stdout)
            .trim()
            .to_string();

        let head_sha = super::super::GitService::get_head_sha(repo).unwrap();

        let result = commit_tree(repo, &tree_sha, &[&head_sha], "Test commit");
        assert!(result.is_ok());
        let commit_sha = result.unwrap();
        assert!(!commit_sha.is_empty());
        assert_ne!(commit_sha, head_sha);
    }

    #[test]
    fn test_update_branch_ref() {
        let dir = setup_test_repo();
        let repo = dir.path();

        // Create a branch at current HEAD
        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(repo)
            .output()
            .expect("git branch failed");

        let head_sha = super::super::GitService::get_head_sha(repo).unwrap();

        // Create a new commit via commit-tree
        let tree_output = Command::new("git")
            .args(["rev-parse", "HEAD^{tree}"])
            .current_dir(repo)
            .output()
            .expect("git rev-parse failed");
        let tree_sha = String::from_utf8_lossy(&tree_output.stdout)
            .trim()
            .to_string();

        let new_sha = commit_tree(repo, &tree_sha, &[&head_sha], "Advance ref").unwrap();

        // Update the branch ref
        let result = update_branch_ref(repo, "test-branch", &new_sha);
        assert!(result.is_ok());

        // Verify branch now points to new SHA
        let branch_sha = super::super::GitService::get_branch_sha(repo, "test-branch").unwrap();
        assert_eq!(branch_sha, new_sha);
    }

    #[test]
    fn test_try_merge_checkout_free_clean() {
        let dir = setup_test_repo();
        let repo = dir.path();

        create_branch_with_change(repo, "feature", "feature.txt", "feature content\n");

        // Verify main doesn't have feature.txt before merge
        assert!(!repo.join("feature.txt").exists());

        let result = try_merge_checkout_free(repo, "feature", "main");
        assert!(result.is_ok());

        match result.unwrap() {
            CheckoutFreeMergeResult::Success { commit_sha } => {
                assert!(!commit_sha.is_empty());
                // Working tree should NOT have feature.txt yet (checkout-free!)
                assert!(
                    !repo.join("feature.txt").exists(),
                    "Working tree should not be modified by checkout-free merge"
                );
                // But branch ref should be advanced
                let main_sha =
                    super::super::GitService::get_branch_sha(repo, "main").unwrap();
                assert_eq!(main_sha, commit_sha);
            }
            CheckoutFreeMergeResult::Conflict { .. } => {
                panic!("Expected success, got conflict");
            }
        }
    }

    #[test]
    fn test_try_merge_checkout_free_conflict() {
        let dir = setup_test_repo();
        let repo = dir.path();

        create_branch_with_change(repo, "branch-a", "shared.txt", "content from branch-a\n");
        create_branch_with_change(repo, "branch-b", "shared.txt", "content from branch-b\n");

        // Merge branch-a into main
        Command::new("git")
            .args(["merge", "branch-a", "--no-edit"])
            .current_dir(repo)
            .output()
            .expect("git merge failed");

        let result = try_merge_checkout_free(repo, "branch-b", "main");
        assert!(result.is_ok());

        match result.unwrap() {
            CheckoutFreeMergeResult::Conflict { files } => {
                assert!(!files.is_empty());
            }
            CheckoutFreeMergeResult::Success { .. } => {
                panic!("Expected conflict, got success");
            }
        }
    }

    #[test]
    fn test_try_squash_merge_checkout_free() {
        let dir = setup_test_repo();
        let repo = dir.path();

        create_branch_with_change(repo, "feature", "feature.txt", "feature content\n");

        let result =
            try_squash_merge_checkout_free(repo, "feature", "main", "squash: add feature");
        assert!(result.is_ok());

        match result.unwrap() {
            CheckoutFreeMergeResult::Success { commit_sha } => {
                assert!(!commit_sha.is_empty());

                // Verify single parent (squash = no merge commit)
                let parent_output = Command::new("git")
                    .args(["rev-parse", &format!("{}^@", commit_sha)])
                    .current_dir(repo)
                    .output()
                    .expect("git rev-parse parents failed");
                let parents = String::from_utf8_lossy(&parent_output.stdout);
                let parent_count = parents.trim().lines().count();
                assert_eq!(parent_count, 1, "Squash merge should have exactly 1 parent");

                // Working tree untouched
                assert!(
                    !repo.join("feature.txt").exists(),
                    "Working tree should not be modified by checkout-free squash merge"
                );
            }
            CheckoutFreeMergeResult::Conflict { .. } => {
                panic!("Expected success, got conflict");
            }
        }
    }

    #[test]
    fn test_try_fast_forward_checkout_free() {
        let dir = setup_test_repo();
        let repo = dir.path();

        // Create feature branch with a change (main is behind feature = FF possible)
        create_branch_with_change(repo, "feature", "feature.txt", "feature content\n");

        let feature_sha = super::super::GitService::get_branch_sha(repo, "feature").unwrap();

        let result = try_fast_forward_checkout_free(repo, "feature", "main");
        assert!(result.is_ok());

        match result.unwrap() {
            CheckoutFreeMergeResult::Success { commit_sha } => {
                assert_eq!(commit_sha, feature_sha);
                // Main ref should now equal feature
                let main_sha =
                    super::super::GitService::get_branch_sha(repo, "main").unwrap();
                assert_eq!(main_sha, feature_sha);
            }
            CheckoutFreeMergeResult::Conflict { .. } => {
                panic!("Expected FF success, got conflict");
            }
        }
    }

    #[test]
    fn test_try_fast_forward_falls_back_to_merge() {
        let dir = setup_test_repo();
        let repo = dir.path();

        // Create divergent branches (FF not possible)
        create_branch_with_change(repo, "feature", "feature.txt", "feature\n");
        // Add another commit on main so it diverges
        fs::write(repo.join("main-only.txt"), "main change\n").expect("write failed");
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .expect("git add failed");
        Command::new("git")
            .args(["commit", "-m", "Main diverges"])
            .current_dir(repo)
            .output()
            .expect("git commit failed");

        let result = try_fast_forward_checkout_free(repo, "feature", "main");
        assert!(result.is_ok());

        // Should fall back to regular merge (not FF)
        match result.unwrap() {
            CheckoutFreeMergeResult::Success { commit_sha } => {
                assert!(!commit_sha.is_empty());
            }
            CheckoutFreeMergeResult::Conflict { .. } => {
                panic!("Expected merge success after FF fallback");
            }
        }
    }

    #[test]
    fn test_parse_merge_tree_conflicts_content() {
        let stderr = "CONFLICT (content): Merge conflict in src/main.rs\nAuto-merging README.md\n";
        let files = parse_merge_tree_conflicts(stderr);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_parse_merge_tree_conflicts_multiple() {
        let stderr = "\
CONFLICT (content): Merge conflict in file1.rs
CONFLICT (add/add): Merge conflict in file2.rs
Auto-merging file3.rs
";
        let files = parse_merge_tree_conflicts(stderr);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], PathBuf::from("file1.rs"));
        assert_eq!(files[1], PathBuf::from("file2.rs"));
    }

    #[test]
    fn test_parse_merge_tree_conflicts_none() {
        let stderr = "Auto-merging README.md\n";
        let files = parse_merge_tree_conflicts(stderr);
        assert!(files.is_empty());
    }
}
