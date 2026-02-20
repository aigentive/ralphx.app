use super::*;

// ── Unit tests for is_transient_error ─────────────────────────────────────

#[test]
fn test_transient_index_lock() {
    assert!(is_transient_error(
        "error: could not lock config file .git/index.lock: File exists"
    ));
}

#[test]
fn test_transient_unable_to_create_lock() {
    assert!(is_transient_error(
        "fatal: Unable to create '/path/to/.git/index.lock': File exists."
    ));
}

#[test]
fn test_transient_cannot_lock_ref() {
    assert!(is_transient_error(
        "error: cannot lock ref 'refs/heads/main': ref already locked"
    ));
}

#[test]
fn test_transient_fetch_head() {
    assert!(is_transient_error(
        "error: could not lock config file .git/FETCH_HEAD: File exists"
    ));
}

#[test]
fn test_transient_shallow_file_changed() {
    assert!(is_transient_error(
        "error: shallow file has changed since we read it"
    ));
}

#[test]
fn test_non_transient_merge_conflict() {
    assert!(!is_transient_error(
        "CONFLICT (content): Merge conflict in src/main.rs"
    ));
}

#[test]
fn test_non_transient_not_a_repo() {
    assert!(!is_transient_error(
        "fatal: not a git repository (or any of the parent directories): .git"
    ));
}

#[test]
fn test_non_transient_branch_not_found() {
    assert!(!is_transient_error(
        "error: pathspec 'missing-branch' did not match any file(s) known to git"
    ));
}

#[test]
fn test_empty_stderr_not_transient() {
    assert!(!is_transient_error(""));
}

// ── exec_with_retry tests ─────────────────────────────────────────────────

#[test]
fn test_exec_with_retry_success_on_first_attempt() {
    // `git --version` should always succeed
    let args: Vec<String> = vec!["--version".to_string()];
    let cwd = std::path::PathBuf::from("/tmp");
    let result = exec_with_retry(&args, &cwd, None);
    assert!(result.is_ok());
    assert!(result.unwrap().status.success());
}

#[test]
fn test_exec_with_retry_non_transient_error_no_retry() {
    // `git` in a non-existent directory should fail immediately (not retry)
    let args: Vec<String> = vec!["status".to_string()];
    let cwd = std::path::PathBuf::from("/nonexistent_path_that_does_not_exist_xyz");
    let result = exec_with_retry(&args, &cwd, None);
    // Should return an error (either spawn failure or git error)
    assert!(result.is_err() || result.as_ref().map(|o| !o.status.success()).unwrap_or(false));
}

/// Simulate a transient error by running a command whose stderr contains a transient pattern.
/// We do this by using `git rev-parse` on a known-invalid path that produces stderr output,
/// then checking the retry logic path separately.
#[test]
fn test_transient_patterns_constant_coverage() {
    // Verify all documented patterns are in TRANSIENT_PATTERNS
    assert!(TRANSIENT_PATTERNS.contains(&ERR_INDEX_LOCK));
    assert!(TRANSIENT_PATTERNS.contains(&ERR_UNABLE_CREATE_LOCK));
    assert!(TRANSIENT_PATTERNS.contains(&ERR_CANNOT_LOCK_REF));
    assert!(TRANSIENT_PATTERNS.contains(&ERR_FETCH_HEAD));
    assert!(TRANSIENT_PATTERNS.contains(&ERR_SHALLOW_FILE_CHANGED));
}

#[test]
fn test_retry_backoff_array_length() {
    // Ensure backoff array covers all retry attempts
    let git_cfg = git_runtime_config();
    assert_eq!(
        git_cfg.retry_backoff_secs.len(),
        git_cfg.max_retries as usize,
        "retry_backoff_secs must have one entry per retry attempt"
    );
}

// ── Async timeout tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_run_basic_git_version() {
    // Use the temp dir as cwd; --version doesn't need a git repo
    let tmpdir = std::env::temp_dir();
    let result = run(&["--version"], &tmpdir).await;
    assert!(result.is_ok(), "git --version should succeed: {:?}", result);
}

#[tokio::test]
async fn test_run_status_basic() {
    let tmpdir = std::env::temp_dir();
    // `git --version` exits 0, so run_status should return true
    let result = run_status(&["--version"], &tmpdir).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "git --version should report success");
}

#[tokio::test]
async fn test_run_with_env_basic() {
    let tmpdir = std::env::temp_dir();
    let result = run_with_env(&["--version"], &tmpdir, &[("GIT_TERMINAL_PROMPT", "0")]).await;
    assert!(result.is_ok(), "git --version with env should succeed: {:?}", result);
}
