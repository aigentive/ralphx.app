use super::super::*;
use super::init_test_repo;
use std::process::Command;

// =========================================================================
// commit_all Tests
// =========================================================================

#[test]
fn test_commit_all_excludes_environment_artifacts() {
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

    // Create .gitignore with environment artifact patterns
    std::fs::write(repo.join(".gitignore"), "node_modules\nsrc-tauri/target\n").unwrap();

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

    // Create .gitignore to exclude environment artifacts
    std::fs::write(repo.join(".gitignore"), "node_modules\nsrc-tauri/target\n").unwrap();

    std::fs::write(repo.join("tracked.txt"), "updated").unwrap();
    std::fs::write(repo.join("node_modules"), "placeholder").unwrap();
    std::fs::create_dir_all(repo.join("src-tauri")).unwrap();
    std::fs::write(repo.join("src-tauri/target"), "placeholder").unwrap();

    let commit_sha = GitService::commit_all(repo, "test commit")
        .unwrap()
        .expect("commit should be created");
    assert!(!commit_sha.is_empty());

    let tracked_files = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    let tracked_files = String::from_utf8_lossy(&tracked_files.stdout);

    assert!(tracked_files.contains("tracked.txt"));
    assert!(!tracked_files.contains("node_modules"));
    assert!(!tracked_files.contains("src-tauri/target"));
}

// =========================================================================
// Conflict Marker Detection Tests
// =========================================================================

#[test]
fn test_has_conflict_markers_ignores_committed_marker_literals() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();
    init_test_repo(repo);

    // Commit a file that intentionally contains marker-like content.
    std::fs::write(
        repo.join("fixture.txt"),
        "this literal is intentional: <<<<<<< HEAD\n",
    )
    .unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add fixture"])
        .current_dir(repo)
        .output()
        .unwrap();

    let has_markers = GitService::has_conflict_markers(repo).unwrap();
    assert!(
        !has_markers,
        "Committed marker literals in unchanged files should not block merge completion"
    );
}

#[test]
fn test_has_conflict_markers_detects_unstaged_markers() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = temp_dir.path();
    init_test_repo(repo);

    std::fs::write(repo.join("file.txt"), "line one\nline two\n").unwrap();
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

    std::fs::write(
        repo.join("file.txt"),
        "<<<<<<< ours\nline\n=======\nline\n>>>>>>> theirs\n",
    )
    .unwrap();

    let has_markers = GitService::has_conflict_markers(repo).unwrap();
    assert!(
        has_markers,
        "Unstaged conflict markers in changed files should be detected"
    );
}

// =========================================================================
// branches_have_same_content Tests
// =========================================================================

#[test]
fn test_branches_have_same_content_identical() {
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

    // Create a branch pointing at the same commit (identical content)
    Command::new("git")
        .args(["branch", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();

    let result = GitService::branches_have_same_content(repo, "main", "feature").unwrap();
    assert!(result, "Branches at same commit should be identical");
}

#[test]
fn test_branches_have_same_content_diverged() {
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

    // Create feature branch and add a commit
    Command::new("git")
        .args(["checkout", "-b", "feature"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::fs::write(repo.join("file.txt"), "changed\n").unwrap();
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

    let result = GitService::branches_have_same_content(repo, "main", "feature").unwrap();
    assert!(
        !result,
        "Branches with different content should not be identical"
    );
}
