use super::super::*;
use std::process::Command;

// =========================================================================
// checkout_existing_branch_worktree Tests
// =========================================================================

#[test]
fn test_checkout_existing_branch_worktree_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo with a commit
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo)
        .output();

    // Create a feature branch
    Command::new("git")
        .args(["branch", "feature-branch"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Create worktree checking out the existing branch
    let worktree_path = temp_dir.path().join("worktrees").join("merge-wt");
    let result =
        GitService::checkout_existing_branch_worktree(repo, &worktree_path, "feature-branch");
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

    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo)
        .output();

    Command::new("git")
        .args(["branch", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Path with deeply nested non-existent parent dirs
    let worktree_path = temp_dir.path().join("deep").join("nested").join("merge-wt");
    let result = GitService::checkout_existing_branch_worktree(repo, &worktree_path, "feature");
    assert!(
        result.is_ok(),
        "Should create parent dirs: {:?}",
        result.err()
    );
    assert!(worktree_path.exists());
}

#[test]
fn test_checkout_existing_branch_worktree_nonexistent_branch() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();

    let worktree_path = temp_dir.path().join("merge-wt");
    let result = GitService::checkout_existing_branch_worktree(
        repo,
        &worktree_path,
        "nonexistent-branch",
    );
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
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo)
        .output();

    // Create feature branch (target) at current commit
    Command::new("git")
        .args(["branch", "feature-branch"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Create task branch with a new file (fast-forward from feature-branch)
    Command::new("git")
        .args(["checkout", "-b", "task-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("new-file.txt"), "task work").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "task work"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Go back to main (user's working branch)
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    let merge_wt = temp_dir.path().join("merge-wt");
    let result =
        GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);
    assert!(result.is_ok(), "Merge should succeed: {:?}", result.err());

    match result.unwrap() {
        MergeAttemptResult::Success { commit_sha } => {
            assert!(!commit_sha.is_empty(), "Should have commit SHA");
        }
        MergeAttemptResult::NeedsAgent { .. } => {
            panic!("Fast-forward merge should succeed, not need agent");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
        }
    }

    // Merge worktree should still exist (caller responsible for cleanup)
    assert!(
        merge_wt.exists(),
        "Merge worktree should still exist after success"
    );

    // Clean up worktree
    let _ = GitService::delete_worktree(repo, &merge_wt);
}

#[test]
fn test_try_merge_in_worktree_conflict() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Setup: feature-branch and task-branch modify same file differently
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("shared.txt"), "initial content").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo)
        .output();

    // Create feature branch (target) and add divergent changes
    Command::new("git")
        .args(["checkout", "-b", "feature-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("shared.txt"), "feature branch changes").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature changes"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Create task branch from main with conflicting changes
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["checkout", "-b", "task-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("shared.txt"), "task branch changes").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "task changes"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Go back to main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    let merge_wt = temp_dir.path().join("merge-wt");
    let result =
        GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);
    assert!(
        result.is_ok(),
        "Should return Ok even on conflict: {:?}",
        result.err()
    );

    match result.unwrap() {
        MergeAttemptResult::NeedsAgent { conflict_files } => {
            assert!(!conflict_files.is_empty(), "Should report conflict files");
            // Merge worktree should still exist (for agent to resolve in)
            assert!(
                merge_wt.exists(),
                "Merge worktree should be kept for conflict resolution"
            );
            // MERGE_HEAD should exist (merge NOT aborted)
            assert!(
                GitService::is_merge_in_progress(&merge_wt),
                "Merge should still be in progress in worktree"
            );
        }
        MergeAttemptResult::Success { .. } => {
            panic!("Conflicting merge should need agent, not succeed");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
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
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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
    std::fs::write(repo.join("test.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(repo)
        .output();

    // Create feature branch (target)
    Command::new("git")
        .args(["branch", "feature-branch"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Create task branch
    Command::new("git")
        .args(["checkout", "-b", "task-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("new.txt"), "task").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "task"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Go back to main — this is the branch the user is working on
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Record main repo state before merge
    let branch_before = GitService::get_current_branch(repo).unwrap();

    let merge_wt = temp_dir.path().join("merge-wt");
    let _ = GitService::try_merge_in_worktree(repo, "task-branch", "feature-branch", &merge_wt);

    // Main repo should still be on the same branch
    let branch_after = GitService::get_current_branch(repo).unwrap();
    assert_eq!(
        branch_before, branch_after,
        "Main repo branch should not change"
    );

    // Clean up
    let _ = GitService::delete_worktree(repo, &merge_wt);
}

// =========================================================================
// Worktree porcelain parsing tests
// =========================================================================

#[test]
fn test_parse_worktree_porcelain_normal() {
    let output = "\
worktree /home/user/project
HEAD abc1234567890abcdef1234567890abcdef123456
branch refs/heads/main

worktree /home/user/worktrees/feature
HEAD def4567890abcdef1234567890abcdef12345678
branch refs/heads/feature-branch

";
    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 2);

    assert_eq!(result[0].path, "/home/user/project");
    assert_eq!(
        result[0].head.as_deref(),
        Some("abc1234567890abcdef1234567890abcdef123456")
    );
    assert_eq!(result[0].branch.as_deref(), Some("main"));

    assert_eq!(result[1].path, "/home/user/worktrees/feature");
    assert_eq!(
        result[1].head.as_deref(),
        Some("def4567890abcdef1234567890abcdef12345678")
    );
    assert_eq!(result[1].branch.as_deref(), Some("feature-branch"));
}

#[test]
fn test_parse_worktree_porcelain_bare_repo() {
    let output = "\
worktree /home/user/project.git
bare

";
    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 1);

    assert_eq!(result[0].path, "/home/user/project.git");
    assert!(result[0].head.is_none());
    assert!(result[0].branch.is_none());
}

#[test]
fn test_parse_worktree_porcelain_detached_head() {
    let output = "\
worktree /home/user/project
HEAD abc1234567890abcdef1234567890abcdef123456
branch refs/heads/main

worktree /home/user/worktrees/detached
HEAD 9876543210abcdef1234567890abcdef12345678
detached

";
    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 2);

    assert_eq!(result[1].path, "/home/user/worktrees/detached");
    assert_eq!(
        result[1].head.as_deref(),
        Some("9876543210abcdef1234567890abcdef12345678")
    );
    assert!(result[1].branch.is_none());
}

#[test]
fn test_parse_worktree_porcelain_no_trailing_newline() {
    // Some git versions may not emit a trailing blank line
    let output = "\
worktree /home/user/project
HEAD abc1234567890abcdef1234567890abcdef123456
branch refs/heads/main";

    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 1);

    assert_eq!(result[0].path, "/home/user/project");
    assert_eq!(result[0].branch.as_deref(), Some("main"));
}

#[test]
fn test_parse_worktree_porcelain_empty_output() {
    let result = GitService::parse_worktree_porcelain("");
    assert!(result.is_empty());
}

#[test]
fn test_parse_worktree_porcelain_nested_branch_name() {
    let output = "\
worktree /home/user/worktrees/task
HEAD abc1234567890abcdef1234567890abcdef123456
branch refs/heads/ralphx/my-app/task-abc123

";
    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 1);

    // Nested branch names should be preserved after stripping refs/heads/
    assert_eq!(
        result[0].branch.as_deref(),
        Some("ralphx/my-app/task-abc123")
    );
}

#[test]
fn test_parse_worktree_porcelain_prunable() {
    let output = "\
worktree /home/user/project
HEAD abc1234567890abcdef1234567890abcdef123456
branch refs/heads/main

worktree /tmp/stale-wt
HEAD def4567890abcdef1234567890abcdef12345678
branch refs/heads/old-branch
prunable gitdir file points to non-existent location

";
    let result = GitService::parse_worktree_porcelain(output);
    assert_eq!(result.len(), 2);

    // Prunable flag is ignored (we just parse path/head/branch)
    assert_eq!(result[1].path, "/tmp/stale-wt");
    assert_eq!(result[1].branch.as_deref(), Some("old-branch"));
}
