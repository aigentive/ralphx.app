use super::super::*;
use std::process::Command;

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
        std::fs::write(
            repo.join(format!("test{}.txt", i)),
            format!("content {}", i),
        )
        .unwrap();
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
// find_commit_by_message_grep Tests
// =========================================================================

#[test]
fn test_find_commit_by_message_grep_finds_matching_commit() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    super::init_test_repo(repo);

    // Create a commit with a task ID in the message
    std::fs::write(repo.join("file.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "task abc-123: initial work"])
        .current_dir(repo)
        .output()
        .unwrap();

    // Get the SHA of that commit for verification
    let expected_sha = {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    // Get the branch name
    let branch = {
        let out = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let result = GitService::find_commit_by_message_grep(repo, "abc-123", &branch).unwrap();
    assert_eq!(result, Some(expected_sha));
}

#[test]
fn test_find_commit_by_message_grep_returns_none_when_not_found() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();

    super::init_test_repo(repo);

    std::fs::write(repo.join("file.txt"), "initial").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "unrelated commit message"])
        .current_dir(repo)
        .output()
        .unwrap();

    let branch = {
        let out = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    let result = GitService::find_commit_by_message_grep(repo, "nonexistent-id", &branch).unwrap();
    assert_eq!(result, None);
}
