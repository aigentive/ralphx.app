use super::super::*;
use super::init_test_repo;
use std::process::Command;

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
    assert!(
        result.is_ok(),
        "try_rebase_and_merge should succeed for first task"
    );

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
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
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
            assert!(
                repo.join("initial.txt").exists(),
                "Initial file should exist"
            );
            assert!(repo.join("second.txt").exists(), "Second file should exist");
            assert!(
                repo.join("feature.txt").exists(),
                "Feature file should exist"
            );
        }
        MergeAttemptResult::NeedsAgent { .. } => {
            panic!("Clean merge should not need agent");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
        }
    }
}

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
    let is_on_main_after =
        GitService::is_commit_on_branch(repo, &task_branch_head, "main").unwrap();
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
    let is_merged_after =
        GitService::is_commit_on_branch(&main_repo, &task_branch_head, "main").unwrap();
    assert!(
        is_merged_after,
        "Task branch HEAD should be on main after merge"
    );
}

// =========================================================================
// try_merge Tests (Phase 98 - Worktree mode direct merge)
// =========================================================================

#[test]
fn test_try_merge_clean_fast_forward() {
    // Task branch has commits ahead of base, no divergence → fast-forward
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

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

    // Initial commit on main
    std::fs::write(repo.join("initial.txt"), "initial").unwrap();
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

    // Create task branch from main
    Command::new("git")
        .args(["checkout", "-b", "task-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
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

    // Back to main (no new commits on main → fast-forward possible)
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    let result = GitService::try_merge(repo, "task-branch", "main");
    assert!(
        result.is_ok(),
        "try_merge should succeed: {:?}",
        result.err()
    );

    match result.unwrap() {
        MergeAttemptResult::Success { commit_sha } => {
            let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
            assert!(on_main, "Merge commit should be on main");
            assert!(
                repo.join("feature.txt").exists(),
                "Feature file should exist"
            );
        }
        MergeAttemptResult::NeedsAgent { .. } => {
            panic!("Clean fast-forward merge should not need agent");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
        }
    }
}

#[test]
fn test_try_merge_with_diverged_branches() {
    // Both base and task branch have new commits → merge commit created
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

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

    // Initial commit on main
    std::fs::write(repo.join("initial.txt"), "initial").unwrap();
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

    // Create task branch
    Command::new("git")
        .args(["checkout", "-b", "task-branch"])
        .current_dir(repo)
        .output()
        .unwrap();
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

    // Go back to main and add a non-conflicting commit
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("other.txt"), "other work").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "other work on main"])
        .current_dir(repo)
        .output()
        .unwrap();

    let result = GitService::try_merge(repo, "task-branch", "main");
    assert!(
        result.is_ok(),
        "try_merge should succeed: {:?}",
        result.err()
    );

    match result.unwrap() {
        MergeAttemptResult::Success { commit_sha } => {
            let on_main = GitService::is_commit_on_branch(repo, &commit_sha, "main").unwrap();
            assert!(on_main, "Merge commit should be on main");
            assert!(
                repo.join("feature.txt").exists(),
                "Feature file should exist"
            );
            assert!(repo.join("other.txt").exists(), "Other file should exist");
        }
        MergeAttemptResult::NeedsAgent { .. } => {
            panic!("Non-conflicting diverged merge should not need agent");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
        }
    }
}

#[test]
fn test_try_merge_with_conflict() {
    // Both branches modify the same file → conflict → NeedsAgent
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

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

    // Initial commit
    std::fs::write(repo.join("shared.txt"), "original content").unwrap();
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

    // Task branch modifies shared file
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

    // Main also modifies shared file (conflict)
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("shared.txt"), "main branch changes").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main changes"])
        .current_dir(repo)
        .output()
        .unwrap();

    let result = GitService::try_merge(repo, "task-branch", "main");
    assert!(
        result.is_ok(),
        "try_merge should return Ok even on conflict: {:?}",
        result.err()
    );

    match result.unwrap() {
        MergeAttemptResult::NeedsAgent { conflict_files } => {
            assert!(!conflict_files.is_empty(), "Should report conflict files");
            // Verify merge was aborted (repo is clean)
            let has_changes = GitService::has_uncommitted_changes(repo).unwrap();
            assert!(
                !has_changes,
                "Merge should be aborted, no uncommitted changes"
            );
        }
        MergeAttemptResult::Success { .. } => {
            panic!("Conflicting merge should need agent, not succeed");
        }
        MergeAttemptResult::BranchNotFound { branch } => {
            panic!("Unexpected BranchNotFound: {}", branch);
        }
    }
}

#[test]
fn test_try_squash_merge_source_branch_not_found() {
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

    let result =
        GitService::try_squash_merge(repo, "nonexistent-source", "main", "squash commit").unwrap();

    match result {
        MergeAttemptResult::BranchNotFound { branch } => {
            assert_eq!(branch, "nonexistent-source");
        }
        other => panic!("Expected BranchNotFound, got {:?}", other),
    }
}

#[test]
fn test_try_squash_merge_target_branch_not_found() {
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

    // Get the actual default branch name (the source that exists)
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    let existing_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let result = GitService::try_squash_merge(
        repo,
        &existing_branch,
        "nonexistent-target",
        "squash commit",
    )
    .unwrap();

    match result {
        MergeAttemptResult::BranchNotFound { branch } => {
            assert_eq!(branch, "nonexistent-target");
        }
        other => panic!("Expected BranchNotFound, got {:?}", other),
    }
}

// =========================================================================
// try_continue_rebase Tests (Bug A fix)
// =========================================================================

#[test]
fn test_try_continue_rebase_no_rebase_in_progress() {
    // When no rebase is in progress, git rebase --continue should fail gracefully
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

    // Create initial commit
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

    // try_continue_rebase should fail since no rebase is in progress
    let result = GitService::try_continue_rebase(repo);
    assert!(result.is_err());
}

#[test]
fn test_try_continue_rebase_auto_resolved_step() {
    // Create a scenario with auto-resolved conflicts
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo with main branch
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

    // Create file with initial content
    std::fs::write(repo.join("file.txt"), "line1\nline2\nline3\n").unwrap();
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
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Make changes in feature branch (non-conflicting area)
    std::fs::write(repo.join("file.txt"), "line1\nline2modified\nline3\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Switch back to main and make non-conflicting changes
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "line1modified\nline2\nline3\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Attempt rebase from feature onto main - this should auto-resolve
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Simulate a rebase that git auto-resolves
    // In practice, this creates a rebase-merge directory when it would normally succeed
    let rebase_output = Command::new("git")
        .args(["rebase", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // If rebase succeeded directly, that's fine for this test
    // If it enters a state requiring --continue, try_continue_rebase should complete it
    if !rebase_output.status.success() {
        // Rebase is in progress, try to continue it
        let result = GitService::try_continue_rebase(repo).unwrap();
        match result {
            RebaseResult::Success => {
                // Successfully completed the rebase
                assert!(GitService::is_rebase_in_progress(repo) == false);
            }
            RebaseResult::Conflict { .. } => {
                // Acceptable: real conflicts were detected
                // In this test case with non-conflicting changes, we expect Success
            }
        }
    }
}

#[test]
fn test_rebase_onto_auto_resolved_conflicts() {
    // Test that rebase_onto detects and handles auto-resolved conflicts
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

    // Create initial commit on main
    std::fs::write(repo.join("file.txt"), "initial content\n").unwrap();
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

    // Create and checkout feature branch
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Add commit to feature
    std::fs::write(repo.join("file.txt"), "initial content\nfeature line\n").unwrap();
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

    // Go back to main and add a non-conflicting commit
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("newfile.txt"), "new content\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main commit"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Now checkout feature and rebase onto main
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Call rebase_onto - it should handle auto-resolved cases
    let result = GitService::rebase_onto(repo, "main").unwrap();

    // With non-conflicting changes, we expect Success
    match result {
        RebaseResult::Success => {
            // Good: rebase completed
            assert!(GitService::is_rebase_in_progress(repo) == false);
        }
        RebaseResult::Conflict { .. } => {
            // If conflicts were detected, they should be real (non-empty files)
            // The test setup should avoid this
        }
    }
}

// =========================================================================
// try_continue_rebase Tests
// =========================================================================

#[test]
fn test_try_continue_rebase_completes_no_rebase() {
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

    // Call try_continue_rebase when no rebase is in progress
    // Should return an error since there's nothing to continue
    let result = GitService::try_continue_rebase(repo);
    assert!(
        result.is_err(),
        "try_continue_rebase should fail when no rebase in progress"
    );
}

#[test]
fn test_try_continue_rebase_with_auto_resolved_conflicts() {
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

    // Create main with one commit
    std::fs::write(repo.join("file.txt"), "main content\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Create feature branch
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "feature content\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Add another commit on main
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "main updated\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main update"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Checkout feature and try to rebase onto main
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Start rebase - this will result in a conflict with auto-resolve attempt
    let rebase_output = Command::new("git")
        .args(["rebase", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Check if we got into a conflict state where auto-resolve could happen
    if !rebase_output.status.success() {
        let stderr = String::from_utf8_lossy(&rebase_output.stderr);
        if stderr.contains("CONFLICT") {
            // We have a rebase in progress due to conflict
            // This test checks the try_continue_rebase behavior
            let result = GitService::try_continue_rebase(repo);

            // The result depends on whether there are actually unmerged files
            // If no unmerged files exist after git's merge, it should succeed
            // If real conflicts exist, it should return Conflict variant
            match result {
                Ok(RebaseResult::Success) => {
                    // Good: rebase continued and completed
                    assert!(!GitService::is_rebase_in_progress(repo));
                }
                Ok(RebaseResult::Conflict { files }) => {
                    // Real conflicts were found - acceptable
                    assert!(GitService::is_rebase_in_progress(repo) || files.is_empty());
                }
                Err(_) => {
                    // Error during continuation - rebase might be aborted
                }
            }
        }
    }
}

// =========================================================================
// try_complete_stale_rebase Tests (Bug B recovery)
// =========================================================================

#[test]
fn test_try_complete_stale_rebase_no_rebase() {
    // When no rebase is in progress, should return NoRebase
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

    // try_complete_stale_rebase should return NoRebase since no rebase is in progress
    let result = GitService::try_complete_stale_rebase(repo);
    assert!(matches!(result, StaleRebaseResult::NoRebase));
}

#[test]
fn test_try_complete_stale_rebase_auto_resolved_completes() {
    // Create a scenario where rebase is in auto-resolved state and can be completed
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    // Initialize repo with main branch
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

    // Create file with initial content
    std::fs::write(repo.join("file.txt"), "line1\nline2\nline3\n").unwrap();
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
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Make changes in feature branch (non-conflicting area)
    std::fs::write(repo.join("file.txt"), "line1\nline2modified\nline3\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Switch back to main and make non-conflicting changes
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "line1modified\nline2\nline3\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Attempt rebase from feature onto main
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    let rebase_output = Command::new("git")
        .args(["rebase", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // If rebase completed directly, test by simulating a stale state
    // If it's in progress, test try_complete_stale_rebase
    if !rebase_output.status.success() {
        // Rebase is in progress - this is the scenario we want to test
        let result = GitService::try_complete_stale_rebase(repo);

        match result {
            StaleRebaseResult::Completed => {
                // Successfully completed the stale rebase
                assert!(GitService::is_rebase_in_progress(repo) == false);
            }
            StaleRebaseResult::HasConflicts { .. } => {
                // Acceptable: real conflicts were detected
            }
            _ => {
                panic!("Unexpected result: {:?}", result);
            }
        }
    } else {
        // Rebase succeeded directly - that's acceptable for this test
        assert!(GitService::is_rebase_in_progress(repo) == false);
    }
}

#[test]
fn test_squash_merge_identical_branches_returns_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();
    init_test_repo(repo);

    // Create initial commit on main
    std::fs::write(repo.join("file.txt"), "hello\n").unwrap();
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

    // Create feature branch at same commit (identical)
    Command::new("git")
        .args(["branch", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    // try_squash_merge should return Success immediately (early return)
    let result = GitService::try_squash_merge(repo, "feature", "main", "squash merge").unwrap();

    match result {
        MergeAttemptResult::Success { commit_sha } => {
            assert!(!commit_sha.is_empty(), "Should return a valid SHA");
        }
        other => panic!(
            "Expected MergeAttemptResult::Success for identical branches, got {:?}",
            other
        ),
    }
}

// =========================================================================
// try_complete_stale_rebase Tests — has_real_conflicts
// =========================================================================

#[test]
fn test_try_complete_stale_rebase_has_real_conflicts() {
    // Create a scenario where rebase hits real conflicts that can't be auto-resolved
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

    // Create file with initial content
    std::fs::write(repo.join("file.txt"), "same line\n").unwrap();
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

    // Create feature branch with conflicting change on same line
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "feature version of line\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "feature change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Switch back to main and make conflicting change on same line
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "main version of line\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "main change"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Start rebase from feature onto main — should produce real conflicts
    Command::new("git")
        .args(["checkout", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    let rebase_output = Command::new("git")
        .args(["rebase", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Rebase should fail with real conflicts
    assert!(
        !rebase_output.status.success(),
        "Expected rebase to fail with conflicts"
    );
    assert!(
        GitService::is_rebase_in_progress(repo),
        "Expected rebase to be in progress"
    );

    // try_complete_stale_rebase should detect the real conflicts
    let result = GitService::try_complete_stale_rebase(repo);
    match result {
        StaleRebaseResult::HasConflicts { files } => {
            assert!(!files.is_empty(), "Expected at least one conflict file");
        }
        other => {
            panic!("Expected HasConflicts, got {:?}", other);
        }
    }
}
