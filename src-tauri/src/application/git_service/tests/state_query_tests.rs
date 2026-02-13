use super::super::*;

// =========================================================================
// Merge State Detection Tests (Phase 76)
// =========================================================================

#[test]
fn test_is_rebase_in_progress_no_rebase() {
    // Use a temp directory without rebase state
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    assert!(!GitService::is_rebase_in_progress(temp_dir.path()));
}

#[test]
fn test_is_rebase_in_progress_with_rebase_merge() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    // Simulate rebase-merge directory (interactive rebase in progress)
    std::fs::create_dir(git_dir.join("rebase-merge")).unwrap();

    assert!(GitService::is_rebase_in_progress(temp_dir.path()));
}

#[test]
fn test_is_rebase_in_progress_with_rebase_apply() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    // Simulate rebase-apply directory (git am or older rebase in progress)
    std::fs::create_dir(git_dir.join("rebase-apply")).unwrap();

    assert!(GitService::is_rebase_in_progress(temp_dir.path()));
}

#[test]
fn test_is_rebase_in_progress_worktree_style() {
    // Test worktree-style .git file pointing to gitdir
    let temp_dir = tempfile::tempdir().unwrap();
    let git_path = temp_dir.path().join(".git");

    // Create the actual git directory somewhere else
    let actual_git_dir = temp_dir.path().join("actual_git_dir");
    std::fs::create_dir(&actual_git_dir).unwrap();

    // Create .git file pointing to actual git dir
    std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

    // No rebase in progress
    assert!(!GitService::is_rebase_in_progress(temp_dir.path()));

    // Add rebase-merge to actual git dir
    std::fs::create_dir(actual_git_dir.join("rebase-merge")).unwrap();

    assert!(GitService::is_rebase_in_progress(temp_dir.path()));
}

// =========================================================================
// resolve_git_dir Tests
// =========================================================================

#[test]
fn test_resolve_git_dir_regular_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    assert_eq!(GitService::resolve_git_dir(temp_dir.path()), git_dir);
}

#[test]
fn test_resolve_git_dir_worktree_style() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_path = temp_dir.path().join(".git");

    let actual_git_dir = temp_dir.path().join("actual_git_dir");
    std::fs::create_dir(&actual_git_dir).unwrap();

    std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

    assert_eq!(GitService::resolve_git_dir(temp_dir.path()), actual_git_dir);
}

// =========================================================================
// is_merge_in_progress Tests
// =========================================================================

#[test]
fn test_is_merge_in_progress_no_merge() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    assert!(!GitService::is_merge_in_progress(temp_dir.path()));
}

#[test]
fn test_is_merge_in_progress_with_merge_head() {
    let temp_dir = tempfile::tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir(&git_dir).unwrap();

    // Simulate MERGE_HEAD file (merge started but not committed)
    std::fs::write(git_dir.join("MERGE_HEAD"), "abc123\n").unwrap();

    assert!(GitService::is_merge_in_progress(temp_dir.path()));
}

#[test]
fn test_is_merge_in_progress_worktree_style() {
    // Test worktree-style .git file pointing to gitdir
    let temp_dir = tempfile::tempdir().unwrap();
    let git_path = temp_dir.path().join(".git");

    // Create the actual git directory somewhere else
    let actual_git_dir = temp_dir.path().join("actual_git_dir");
    std::fs::create_dir(&actual_git_dir).unwrap();

    // Create .git file pointing to actual git dir
    std::fs::write(&git_path, format!("gitdir: {}", actual_git_dir.display())).unwrap();

    // No merge in progress
    assert!(!GitService::is_merge_in_progress(temp_dir.path()));

    // Add MERGE_HEAD to actual git dir
    std::fs::write(actual_git_dir.join("MERGE_HEAD"), "abc123\n").unwrap();

    assert!(GitService::is_merge_in_progress(temp_dir.path()));
}
