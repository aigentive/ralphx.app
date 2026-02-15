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

    let result = merge_tree_write(repo, "main", "feature").expect("git command failed");
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
    let result = merge_tree_write(repo, "main", "branch-b").expect("git command failed");
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
            let main_sha = super::super::GitService::get_branch_sha(repo, "main").unwrap();
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

    let result = try_squash_merge_checkout_free(repo, "feature", "main", "squash: add feature");
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
            let main_sha = super::super::GitService::get_branch_sha(repo, "main").unwrap();
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
