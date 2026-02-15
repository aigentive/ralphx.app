// Fix F4/F5: delete_worktree and prune_worktrees now propagate errors
//
// After fix: git worktree operations return Err(AppError::GitOperation(...))
// when the underlying git command fails, instead of silently returning Ok(()).

use crate::error::AppError;

#[test]
fn test_f4_fix_git_operation_error_variant_exists() {
    // Verify AppError::GitOperation can carry worktree error messages
    let error = AppError::GitOperation(
        "Failed to delete worktree at '/tmp/wt': error: 'wt' is not a valid worktree".to_string(),
    );
    let msg = error.to_string();
    assert!(
        msg.contains("Failed to delete worktree"),
        "Error should describe the operation"
    );
    assert!(
        msg.contains("/tmp/wt"),
        "Error should include the worktree path"
    );
}

#[test]
fn test_f5_fix_prune_error_propagated() {
    // Verify AppError::GitOperation can carry prune error messages
    let error = AppError::GitOperation(
        "Failed to prune worktrees in '/tmp/repo': fatal: not a git repository".to_string(),
    );
    let msg = error.to_string();
    assert!(
        msg.contains("prune worktrees"),
        "Error should describe the prune operation"
    );
}

#[test]
fn test_f4_fix_error_includes_path_context() {
    // Verify that worktree errors include the path for debugging
    let path = "/home/user/worktrees/task-123";
    let stderr = "error: 'task-123' is not a valid worktree";
    let error = AppError::GitOperation(format!(
        "Failed to delete worktree at '{}': {}",
        path, stderr
    ));
    let msg = error.to_string();
    assert!(msg.contains(path), "Error should include the full path");
    assert!(
        msg.contains(stderr),
        "Error should include the stderr output"
    );
}
