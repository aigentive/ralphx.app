//! Async git command runner — single point of spawn_blocking for all git operations.
//!
//! All git commands are executed with:
//! - A configurable timeout (from `git_runtime_config()`) to prevent hung processes.
//! - Configurable retry attempts with backoff for known transient errors.
use crate::error::{AppError, AppResult};
use crate::infrastructure::agents::claude::git_runtime_config;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use tokio::time::timeout;

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
    let git_cfg = git_runtime_config();
    let max_retries = git_cfg.max_retries as u32;
    let retry_backoff = &git_cfg.retry_backoff_secs;
    let mut last_err: Option<AppError> = None;

    for attempt in 0..max_retries {
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
                let backoff = retry_backoff
                    .get(attempt as usize)
                    .copied()
                    .unwrap_or(4);
                tracing::warn!(
                    attempt = attempt + 1,
                    max = max_retries,
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
            max_retries
        ))
    }))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run a git command on the blocking threadpool, returning full Output.
///
/// Applies a configurable timeout and retries for transient errors.
pub(crate) async fn run(args: &[&str], cwd: &Path) -> AppResult<Output> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();
    let timeout_secs = git_runtime_config().cmd_timeout_secs;

    timeout(
        Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking(move || exec_with_retry(&args, &cwd, None)),
    )
    .await
    .map_err(|_| AppError::GitOperation(format!("git command timed out after {timeout_secs}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))?
}

/// Run a git command with additional environment variables on the blocking threadpool.
///
/// Applies a configurable timeout and retries for transient errors.
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
    let timeout_secs = git_runtime_config().cmd_timeout_secs;

    timeout(
        Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking(move || exec_with_retry(&args, &cwd, Some(&env))),
    )
    .await
    .map_err(|_| AppError::GitOperation(format!("git command timed out after {timeout_secs}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))?
}

/// Run a git command returning just success/failure (for existence checks).
///
/// Applies a configurable timeout. Status checks are not retried (they should be fast and
/// idempotent).
pub(crate) async fn run_status(args: &[&str], cwd: &Path) -> AppResult<bool> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();
    let timeout_secs = git_runtime_config().cmd_timeout_secs;

    timeout(
        Duration::from_secs(timeout_secs),
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
    .map_err(|_| AppError::GitOperation(format!("git status check timed out after {timeout_secs}s")))?
    .map_err(|e| AppError::GitOperation(format!("git task join error: {e}")))
}

#[cfg(test)]
#[path = "git_cmd_tests.rs"]
mod tests;
