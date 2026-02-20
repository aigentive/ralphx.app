// Tests for try_handle_symlink_idempotent() in merge_validation.rs
//
// Covers: symlink skip, wrong symlink removal, real dir removal,
// non-symlink passthrough, various ln flag formats.

use super::super::merge_validation::try_handle_symlink_idempotent;

#[test]
fn non_symlink_command_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = try_handle_symlink_idempotent("npm install", dir.path(), "Test", ".");
    assert!(result.is_none(), "non-symlink commands should pass through");
}

#[test]
fn non_ln_command_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = try_handle_symlink_idempotent("mkdir -p foo/bar", dir.path(), "Test", ".");
    assert!(result.is_none(), "non-ln commands should pass through");
}

#[test]
fn ln_without_s_flag_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = try_handle_symlink_idempotent("ln source target", dir.path(), "Test", ".");
    assert!(result.is_none(), "ln without -s should pass through");
}

#[test]
fn correct_symlink_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source_dir");
    let target = dir.path().join("target_link");
    std::fs::create_dir(&source).unwrap();

    // Create correct symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &target).unwrap();

    let cmd = format!(
        "ln -s {} {}",
        source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_some(), "correct symlink should be skipped");
    let entry = result.unwrap();
    assert_eq!(entry.status, "skipped");
    assert_eq!(entry.phase, "setup");
    assert!(entry.stderr.contains("already exists"));
}

#[test]
fn wrong_symlink_is_removed_and_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let correct_source = dir.path().join("correct_source");
    let wrong_source = dir.path().join("wrong_source");
    let target = dir.path().join("target_link");
    std::fs::create_dir(&correct_source).unwrap();
    std::fs::create_dir(&wrong_source).unwrap();

    // Create symlink pointing to wrong source
    #[cfg(unix)]
    std::os::unix::fs::symlink(&wrong_source, &target).unwrap();

    let cmd = format!(
        "ln -s {} {}",
        correct_source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_none(), "wrong symlink should be removed and command re-run");
    // Target should have been removed
    assert!(!target.exists(), "wrong symlink should be removed");
    assert!(!target.is_symlink(), "wrong symlink should be removed");
}

#[test]
fn real_dir_at_target_is_removed_and_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source_dir");
    let target = dir.path().join("node_modules");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    // Put a file in the target dir to confirm removal
    std::fs::write(target.join("file.txt"), "content").unwrap();

    let cmd = format!(
        "ln -s {} {}",
        source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_none(), "real dir should be removed and command re-run");
    assert!(!target.exists(), "real dir at target should be removed");
}

#[test]
fn target_does_not_exist_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source_dir");
    let target = dir.path().join("target_link");
    std::fs::create_dir(&source).unwrap();

    let cmd = format!(
        "ln -s {} {}",
        source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_none(), "no target should run normally");
}

#[test]
fn ln_sf_flag_is_recognized() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source_dir");
    let target = dir.path().join("target_link");
    std::fs::create_dir(&source).unwrap();

    // Create correct symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &target).unwrap();

    let cmd = format!(
        "ln -sf {} {}",
        source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_some(), "ln -sf with correct symlink should skip");
    assert_eq!(result.unwrap().status, "skipped");
}

#[test]
fn relative_target_resolved_against_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source_dir");
    std::fs::create_dir(&source).unwrap();

    // Create correct symlink at cwd/node_modules
    let target = dir.path().join("node_modules");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &target).unwrap();

    let cmd = format!("ln -s {} node_modules", source.display());

    let result = try_handle_symlink_idempotent(&cmd, dir.path(), "Test", ".");
    assert!(result.is_some(), "relative target should be resolved against cwd");
    assert_eq!(result.unwrap().status, "skipped");
}

#[test]
fn template_resolved_command_works() {
    // Simulates what happens after template resolution:
    // ln -s /Users/dev/project/node_modules /Users/dev/worktrees/task-123/node_modules
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("project").join("node_modules");
    let worktree = dir.path().join("worktree");
    let target = worktree.join("node_modules");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&worktree).unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, &target).unwrap();

    let cmd = format!(
        "ln -s {} {}",
        source.display(),
        target.display()
    );

    let result = try_handle_symlink_idempotent(&cmd, &worktree, "Node.js root", ".");
    assert!(result.is_some(), "template-resolved symlink should be skipped");
    let entry = result.unwrap();
    assert_eq!(entry.status, "skipped");
    assert_eq!(entry.label, "Node.js root");
}
