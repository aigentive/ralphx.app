use super::super::*;
use std::process::Command;

// =========================================================================
// clean_working_tree Tests
// =========================================================================

#[test]
fn test_clean_working_tree_removes_untracked_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
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

    // Create initial commit
    std::fs::write(repo.join("tracked.txt"), "initial").unwrap();
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

    // Create untracked files and directories
    std::fs::write(repo.join("untracked.txt"), "untracked").unwrap();
    std::fs::create_dir(repo.join("untracked_dir")).unwrap();
    std::fs::write(repo.join("untracked_dir/file.txt"), "content").unwrap();

    // Verify untracked files exist
    assert!(repo.join("untracked.txt").exists());
    assert!(repo.join("untracked_dir").exists());

    // Clean working tree
    GitService::clean_working_tree(repo).unwrap();

    // Verify untracked files are removed
    assert!(!repo.join("untracked.txt").exists());
    assert!(!repo.join("untracked_dir").exists());

    // Verify tracked files are preserved
    assert!(repo.join("tracked.txt").exists());
}

#[test]
fn test_clean_working_tree_resets_modified_tracked_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
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

    // Create initial commit
    std::fs::write(repo.join("tracked.txt"), "initial").unwrap();
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

    // Modify tracked file
    std::fs::write(repo.join("tracked.txt"), "modified").unwrap();

    // Verify file is modified
    let content = std::fs::read_to_string(repo.join("tracked.txt")).unwrap();
    assert_eq!(content, "modified");

    // Clean working tree
    GitService::clean_working_tree(repo).unwrap();

    // Verify file is reset to HEAD
    let content = std::fs::read_to_string(repo.join("tracked.txt")).unwrap();
    assert_eq!(content, "initial");
}

#[test]
fn test_clean_working_tree_noop_when_clean() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
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

    // Create initial commit
    std::fs::write(repo.join("tracked.txt"), "content").unwrap();
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

    // Working tree is clean, should be no-op
    let result = GitService::clean_working_tree(repo);

    // Should succeed without error
    assert!(result.is_ok());

    // Verify file still exists and is unchanged
    let content = std::fs::read_to_string(repo.join("tracked.txt")).unwrap();
    assert_eq!(content, "content");
}

#[test]
#[cfg(unix)]
fn test_clean_working_tree_handles_symlinks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Create a separate temp directory for the target file
    // so it doesn't get deleted when the repo temp_dir is dropped
    let target_temp = tempfile::tempdir().unwrap();
    let target_file = target_temp.path().join("target_file.txt");
    std::fs::write(&target_file, "target content").unwrap();

    // Initialize repo
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

    // Create initial commit
    std::fs::write(repo.join("tracked.txt"), "initial").unwrap();
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

    // Create a symlink in the repo pointing to the target file
    let symlink_path = repo.join("symlink.txt");
    std::os::unix::fs::symlink(&target_file, &symlink_path).unwrap();

    // Verify symlink exists
    assert!(symlink_path.exists());

    // Clean working tree
    GitService::clean_working_tree(repo).unwrap();

    // Verify symlink is removed
    assert!(!symlink_path.exists());

    // Verify target file is NOT removed (symlink was the link, not the target)
    assert!(target_file.exists());
    let content = std::fs::read_to_string(&target_file).unwrap();
    assert_eq!(content, "target content");

    // Keep target_temp alive until the end of the test
    drop(target_temp);
}

// =========================================================================
// Feature Branch Operations Tests (Phase 85)
// =========================================================================

#[test]
fn test_create_feature_branch_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo with initial commit
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "content").unwrap();
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

    // Create feature branch
    let result = GitService::create_feature_branch(repo, "ralphx/my-app/plan-abc123", "main");
    assert!(
        result.is_ok(),
        "create_feature_branch should succeed: {:?}",
        result.err()
    );

    // Verify branch exists
    let output = Command::new("git")
        .args(["branch", "--list", "ralphx/my-app/plan-abc123"])
        .current_dir(repo)
        .output()
        .unwrap();
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        branches.contains("ralphx/my-app/plan-abc123"),
        "Feature branch should exist"
    );

    // Verify we didn't checkout the branch (still on main)
    let current = GitService::get_current_branch(repo).unwrap();
    assert_eq!(
        current, "main",
        "Should still be on main after creating feature branch"
    );
}

#[test]
fn test_create_feature_branch_from_specific_source() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo with initial commit on main
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "initial").unwrap();
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

    // Add another commit on main
    std::fs::write(repo.join("file2.txt"), "second").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "second"])
        .current_dir(repo)
        .output()
        .unwrap();

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
    assert_eq!(
        feature_sha, main_sha,
        "Feature branch should point to main HEAD"
    );
}

#[test]
fn test_create_feature_branch_already_exists() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "content").unwrap();
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

    // Create branch first time
    GitService::create_feature_branch(repo, "feature/dup", "main").unwrap();

    // Try to create again — should fail
    let result = GitService::create_feature_branch(repo, "feature/dup", "main");
    assert!(
        result.is_err(),
        "Creating duplicate feature branch should fail"
    );
}

#[test]
fn test_create_feature_branch_invalid_source() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "content").unwrap();
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

    // Create from non-existent source branch
    let result = GitService::create_feature_branch(repo, "feature/bad", "nonexistent-branch");
    assert!(
        result.is_err(),
        "Creating from non-existent source should fail"
    );
}

#[test]
fn test_delete_feature_branch_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "content").unwrap();
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

    // Create feature branch, then merge it back so -d works
    GitService::create_feature_branch(repo, "feature/to-delete", "main").unwrap();

    // Delete it (safe delete — branch is fully merged since it's at same commit as main)
    let result = GitService::delete_feature_branch(repo, "feature/to-delete");
    assert!(
        result.is_ok(),
        "delete_feature_branch should succeed: {:?}",
        result.err()
    );

    // Verify branch no longer exists
    let output = Command::new("git")
        .args(["branch", "--list", "feature/to-delete"])
        .current_dir(repo)
        .output()
        .unwrap();
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        !branches.contains("feature/to-delete"),
        "Feature branch should be deleted"
    );
}

#[test]
fn test_delete_feature_branch_nonexistent() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "content").unwrap();
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

    // Delete non-existent branch — should fail
    let result = GitService::delete_feature_branch(repo, "feature/nonexistent");
    assert!(result.is_err(), "Deleting non-existent branch should fail");
}

#[test]
fn test_is_branch_lock_error_already_used_by_worktree() {
    let error = AppError::GitOperation(
        "Failed to create worktree at '/tmp/merge-wt' for branch 'main': \
        fatal: 'main' is already used by worktree at '/home/user/project'"
            .to_string(),
    );
    assert!(GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_already_checked_out() {
    let error = AppError::GitOperation(
        "fatal: branch 'feature/foo' already checked out at '/tmp/worktree'".to_string(),
    );
    assert!(GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_is_already_checked_out_at() {
    let error = AppError::GitOperation(
        "fatal: 'main' is already checked out at '/home/user/ralphx'".to_string(),
    );
    assert!(GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_fatal_branch_checked_out() {
    let error =
        AppError::GitOperation("fatal: branch is checked out in another worktree".to_string());
    assert!(GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_case_insensitive() {
    let error = AppError::GitOperation(
        "FATAL: 'main' IS ALREADY USED BY WORKTREE at '/path'".to_string(),
    );
    assert!(GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_merge_conflict_not_deferrable() {
    let error =
        AppError::GitOperation("CONFLICT (content): Merge conflict in src/main.rs".to_string());
    assert!(!GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_generic_git_error_not_deferrable() {
    let error = AppError::GitOperation("fatal: not a git repository".to_string());
    assert!(!GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_non_git_error_not_deferrable() {
    let error = AppError::Database("connection failed".to_string());
    assert!(!GitService::is_branch_lock_error(&error));
}

#[test]
fn test_is_branch_lock_error_false_positive_avoided() {
    // This error contains "fatal", "branch", and "checked out" but is NOT a branch lock error.
    // The old pattern would have incorrectly classified this as deferrable.
    let error = AppError::GitOperation(
        "fatal: could not read branch configuration, checked out files may be corrupt"
            .to_string(),
    );
    assert!(!GitService::is_branch_lock_error(&error));
}

#[test]
fn test_branch_exists_returns_true_for_existing_branch() {
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

    // Create an initial commit so main/master exists
    std::fs::write(repo.join("file.txt"), "hello").unwrap();
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

    // Get the actual default branch name
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    let branch_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

    assert!(GitService::branch_exists(repo, &branch_name));
}

#[test]
fn test_branch_exists_returns_false_for_nonexistent_branch() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();

    assert!(!GitService::branch_exists(repo, "nonexistent-branch"));
}
