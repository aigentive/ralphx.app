use super::*;
use std::fs;
use std::process::Command;

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

/// Create a branch with a file change and commit
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

/// Merge a branch into the current branch
fn merge_branch(repo: &Path, branch: &str) {
    Command::new("git")
        .args(["merge", branch, "--no-edit"])
        .current_dir(repo)
        .output()
        .expect("git merge failed");
}

#[tokio::test]
async fn test_verify_merge_happy_path_merged() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Create a task branch with changes
    create_branch_with_change(repo, "task-branch", "feature.txt", "feature content\n");

    // Merge it into main
    merge_branch(repo, "task-branch");

    // Verify the merge
    let result = verify_merge_on_target(repo, "task-branch", "main").await;
    match result {
        MergeVerification::Merged(sha) => {
            assert!(!sha.is_empty(), "Merge commit SHA should not be empty");
        }
        _ => panic!("Expected Merged, got: {:?}", result),
    }
}

#[tokio::test]
async fn test_verify_merge_race_condition_not_merged() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Create a task branch but DON'T merge it
    create_branch_with_change(repo, "task-branch", "feature.txt", "feature content\n");

    // Simulate the race condition: we're checking from main repo
    // Task branch exists but is not merged to main
    let result = verify_merge_on_target(repo, "task-branch", "main").await;
    assert_eq!(
        result,
        MergeVerification::NotMerged,
        "Expected NotMerged for unmerged task branch"
    );
}

#[tokio::test]
async fn test_verify_merge_source_branch_missing() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Try to verify a non-existent source branch
    let result = verify_merge_on_target(repo, "non-existent-branch", "main").await;
    assert_eq!(
        result,
        MergeVerification::SourceBranchMissing,
        "Expected SourceBranchMissing for non-existent source branch"
    );
}

#[tokio::test]
async fn test_verify_merge_target_branch_missing() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Create a task branch
    create_branch_with_change(repo, "task-branch", "feature.txt", "feature content\n");

    // Try to verify against a non-existent target branch
    let result = verify_merge_on_target(repo, "task-branch", "non-existent-target").await;
    assert_eq!(
        result,
        MergeVerification::TargetBranchMissing,
        "Expected TargetBranchMissing for non-existent target branch"
    );
}

#[tokio::test]
async fn test_verify_merge_plan_branch_merged() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Create a plan branch
    create_branch_with_change(repo, "plan-abc123", "plan-feature.txt", "plan content\n");

    // Create a task branch from the plan branch
    Command::new("git")
        .args(["checkout", "plan-abc123"])
        .current_dir(repo)
        .output()
        .expect("git checkout plan-abc123 failed");

    create_branch_with_change(repo, "task-branch", "task-feature.txt", "task content\n");

    // Merge task branch into plan branch
    Command::new("git")
        .args(["checkout", "plan-abc123"])
        .current_dir(repo)
        .output()
        .expect("git checkout plan-abc123 failed");

    merge_branch(repo, "task-branch");

    // Verify the merge to plan branch
    let result = verify_merge_on_target(repo, "task-branch", "plan-abc123").await;
    match result {
        MergeVerification::Merged(sha) => {
            assert!(!sha.is_empty(), "Merge commit SHA should not be empty");
        }
        _ => panic!(
            "Expected Merged for task merged to plan branch, got: {:?}",
            result
        ),
    }
}

#[tokio::test]
async fn test_verify_merge_plan_branch_not_merged() {
    let dir = setup_test_repo();
    let repo = dir.path();

    // Create a plan branch
    create_branch_with_change(repo, "plan-abc123", "plan-feature.txt", "plan content\n");

    // Create a task branch but DON'T merge it to the plan branch
    create_branch_with_change(repo, "task-branch", "task-feature.txt", "task content\n");

    // Verify should return NotMerged
    let result = verify_merge_on_target(repo, "task-branch", "plan-abc123").await;
    assert_eq!(
        result,
        MergeVerification::NotMerged,
        "Expected NotMerged for task not merged to plan branch"
    );
}

// --- Auto-complete dedup guard tests ---

#[test]
fn test_auto_complete_guard_prevents_duplicate() {
    use crate::commands::ExecutionState;

    let exec_state = ExecutionState::new();
    let task_id = "task-abc123";

    // First call succeeds
    assert!(exec_state.try_start_auto_complete(task_id));
    // Second call is blocked (duplicate)
    assert!(!exec_state.try_start_auto_complete(task_id));
}

#[test]
fn test_auto_complete_guard_allows_different_tasks() {
    use crate::commands::ExecutionState;

    let exec_state = ExecutionState::new();

    assert!(exec_state.try_start_auto_complete("task-a"));
    assert!(exec_state.try_start_auto_complete("task-b"));
    // Still blocked for task-a
    assert!(!exec_state.try_start_auto_complete("task-a"));
}

#[test]
fn test_auto_complete_guard_cleanup_allows_retry() {
    use crate::commands::ExecutionState;

    let exec_state = ExecutionState::new();
    let task_id = "task-abc123";

    assert!(exec_state.try_start_auto_complete(task_id));
    assert!(!exec_state.try_start_auto_complete(task_id));

    // After cleanup, a new call is allowed
    exec_state.finish_auto_complete(task_id);
    assert!(exec_state.try_start_auto_complete(task_id));
}

#[test]
fn test_auto_complete_raii_guard_cleans_up_on_drop() {
    use crate::commands::ExecutionState;

    let exec_state = Arc::new(ExecutionState::new());
    let task_id = "task-abc123";

    // Simulate inserting and creating a guard
    assert!(exec_state.try_start_auto_complete(task_id));
    {
        let _guard = super::AutoCompleteGuard {
            execution_state: Arc::clone(&exec_state),
            task_id: task_id.to_string(),
        };
        // While guard is alive, duplicate is blocked
        assert!(!exec_state.try_start_auto_complete(task_id));
        // Re-insert since try_start failed (it wasn't actually added again)
    }
    // After guard drops, the task is removed from the set
    assert!(exec_state.try_start_auto_complete(task_id));
}
