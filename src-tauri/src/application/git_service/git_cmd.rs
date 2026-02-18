//! Async git command runner — single point of spawn_blocking for all git operations.
use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Run a git command on the blocking threadpool, returning full Output.
pub(crate) async fn run(args: &[&str], cwd: &Path) -> AppResult<Output> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Command::new("git")
            .args(&args)
            .current_dir(&cwd)
            .output()
            .map_err(|e| AppError::GitOperation(format!("git {}: {}", args.join(" "), e)))
    })
    .await
    .map_err(|e| AppError::GitOperation(format!("git task join error: {}", e)))?
}

/// Run a git command with additional environment variables on the blocking threadpool.
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
    tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new("git");
        cmd.args(&args).current_dir(&cwd);
        for (key, val) in &env {
            cmd.env(key, val);
        }
        cmd.output()
            .map_err(|e| AppError::GitOperation(format!("git {}: {}", args.join(" "), e)))
    })
    .await
    .map_err(|e| AppError::GitOperation(format!("git task join error: {}", e)))?
}

/// Run a git command returning just success/failure (for existence checks).
pub(crate) async fn run_status(args: &[&str], cwd: &Path) -> AppResult<bool> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let cwd = cwd.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Command::new("git")
            .args(&args)
            .current_dir(&cwd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
    .await
    .map_err(|e| AppError::GitOperation(format!("git task join error: {}", e)))
}
