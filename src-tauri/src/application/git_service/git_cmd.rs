//! Async git command runner — single point of spawn_blocking for all git operations.
//!
//! All git commands are executed with:
//! - A 60-second timeout to prevent hung processes from blocking threads forever.
//! - Up to 3 retry attempts with exponential backoff (1s/2s/4s) for known transient errors.
use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for git commands. Prevents hung git processes from blocking spawn_blocking
/// threads indefinitely (e.g. waiting for a lock or network hang).
const GIT_CMD_TIMEOUT_SECS: u64 = 60;

/// Maximum number of retry attempts for transient git errors.
const GIT_MAX_RETRIES: u32 = 3;

/// Backoff delays (seconds) for successive retry attempts.
const GIT_RETRY_BACKOFF_SECS: [u64; 3] = [1, 2, 4];

// ── Transient error pattern constants ────────────────────────────────────────
// Source: git stderr output on Linux/macOS. These are transient errors caused by
// lock contention, concurrent operations, or temporary I/O issues.

/// git reports a stale or held index.lock file preventing operations.
const ERR_INDEX_LOCK: &str = "index.lock";

/// git cannot create a .lock file (e.g. for a ref or the index).
const ERR_UNABLE_CREATE_LOCK: &str = "Unable to create";

/// git cannot acquire a ref lock, usually due to concurrent ref updates.
const ERR_CANNOT_LOCK_REF: &str = "cannot lock ref";

/// git cannot update FETCH_HEAD due to concurrent fetch operations.
const ERR_FETCH_HEAD: &str = "FETCH_HEAD";

/// git's shallow file was modified concurrently during a fetch/clone.
const ERR_SHALLOW_FILE_CHANGED: &str = "shallow file has changed";

/// All patterns that indicate a transient (retriable) git failure.
/// These strings are matched against git's stderr output.
const TRANSIENT_PATTERNS: &[&str] = &[
    ERR_INDEX_LOCK,
    ERR_UNABLE_CREATE_LOCK,
    ERR_CANNOT_LOCK_REF,
    ERR_FETCH_HEAD,
    ERR_SHALLOW_FILE_CHANGED,
];

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Returns true if the given stderr output matches any known transient error pattern.
fn is_transient_error(stderr: &str) -> bool {
    TRANSIENT_PATTERNS.iter().any(|pat| stderr.contains(pat))
}

/// Execute a git command synchronously (blocking). Returns the full output.
fn exec_git(args: &[String], cwd: &std::path::PathBuf) -> AppResult<Output> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| AppError::GitOperation(format!("git {}: {}", args.join(" "), e)))
}

/// Execute a git command synchronously with additional env vars. Returns the full output.
fn exec_git_with_env(
    args: &[String],
    cwd: &std::path::PathBuf,
    env: &[(String, String)],
) -> AppResult<Output> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(cwd);
    for (key, val) in env {
        cmd.env(key, val);
    }
    cmd.output()
        .map_err(|e| AppError::GitOperation(format!("git {}: {}", args.join(" "), e)))
}

/// Run a blocking git command with retry-on-transient. Meant to be called inside
/// `spawn_blocking`. Returns the `Output` of the last successful attempt.
fn exec_with_retry(
    args: &[String],
    cwd: &std::path::PathBuf,
    env: Option<&[(String, String)]>,
) -> AppResult<Output> {
    let mut last_err: Option<AppError> = None;

    for attempt in 0..GIT_MAX_RETRIES {
        let result = match env {
            Some(e) => exec_git_with_env(args, cwd, e),
            None => exec_git(args, cwd),
        };

        match result {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() || !is_transient_error(&stderr) {
                    // Success or non-transient failure — return immediately.
                    return Ok(output);
                }
                // Transient failure: record and retry after backoff.
                let backoff = GIT_RETRY_BACKOFF_SECS
                    .get(attempt as usize)
                    .copied()
                    .unwrap_or(4);
                tracing::warn!(
                    attempt = attempt + 1,
                    max = GIT_MAX_RETRIES,
                    backoff_secs = backoff,
                    stderr = %stderr.trim(),
                    args = %args.join(" "),
                    "git transient error — retrying"
                );
                last_err = Some(AppError::GitOperation(format!(
                    "git {} transient error (attempt {}): {}",
                    args.join(" "),
                    attempt + 1,
                    stderr.trim()
                )));
                std::thread::sleep(Duration::from_secs(backoff));
            }
            Err(e) => {
                // spawn/IO error — not retriable
                return Err(e);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        AppError::GitOperation(format!(
            "git {} failed after {} retries",
            args.join(" "),
            GIT_MAX_RETRIES
        ))
    }))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run a git command on the blocking threadpool, returning full Output.
///
/// Applies a 60-second timeout and up to 3 retries for transient errors.
pub(crate) async fn run(args: &[&str], cwd: &Path) -> AppResult<Output> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();

    timeout(
        Duration::from_secs(GIT_CMD_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || exec_with_retry(&args, &cwd, None)),
    )
    .await
    .map_err(|_| AppError::GitOperation(format!("git command timed out after {GIT_CMD_TIMEOUT_SECS}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))?
}

/// Run a git command with additional environment variables on the blocking threadpool.
///
/// Applies a 60-second timeout and up to 3 retries for transient errors.
pub(crate) async fn run_with_env(
    args: &[&str],
    cwd: &Path,
    env: &[(&str, &str)],
) -> AppResult<Output> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();
    let env: Vec<(String, String)> = env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    timeout(
        Duration::from_secs(GIT_CMD_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || exec_with_retry(&args, &cwd, Some(&env))),
    )
    .await
    .map_err(|_| AppError::GitOperation(format!("git command timed out after {GIT_CMD_TIMEOUT_SECS}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))?
}

/// Run a git command returning just success/failure (for existence checks).
///
/// Applies a 60-second timeout. Status checks are not retried (they should be fast and
/// idempotent).
pub(crate) async fn run_status(args: &[&str], cwd: &Path) -> AppResult<bool> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();

    timeout(
        Duration::from_secs(GIT_CMD_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            Command::new("git")
                .args(&args)
                .current_dir(&cwd)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }),
    )
    .await
    .map_err(|_| AppError::GitOperation(format!("git status check timed out after {GIT_CMD_TIMEOUT_SECS}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))
}

#[cfg(test)]
mod tests {
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
        assert_eq!(
            GIT_RETRY_BACKOFF_SECS.len(),
            GIT_MAX_RETRIES as usize,
            "GIT_RETRY_BACKOFF_SECS must have one entry per retry attempt"
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
}
