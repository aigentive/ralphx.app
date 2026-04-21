// Production implementation of GithubServiceTrait using the `gh` CLI.
//
// Safety rules (NON-NEGOTIABLE):
//  - All subprocess calls: tokio::process::Command + .spawn() + kill_on_drop(true)
//  - NEVER .output() — kills the tokio runtime by blocking
//  - Pipe buffer safety: piped stdout/stderr consumed via BufReader to prevent deadlocks
//  - All calls wrapped in tokio::time::timeout(30s)
//  - Stderr sanitized: secrets filtered, token-embedded URLs scrubbed

use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

use crate::domain::services::github_service::{GithubServiceTrait, PrStatus};
use crate::error::AppError;
use crate::utils::secret_redactor::redact;
use crate::AppResult;

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(30);

/// Secret keyword fragments to filter from stderr output (case-insensitive match)
const SECRET_KEYWORDS: &[&str] = &[
    "token", "bearer", "auth", "credential", "password", "secret", "ghp_", "gho_",
];

/// Known error message fragments from `gh pr create` when a PR already exists for this branch.
/// Used by `create_draft_pr` to detect duplicates and return `AppError::DuplicatePr`.
pub(crate) const DUPLICATE_PR_FRAGMENTS: [&str; 3] = [
    "already exists",
    "a pull request for",
    "already a pull request",
];

pub(crate) const CREATE_PR_UNSUPPORTED_JSON_FRAGMENTS: [&str; 1] = ["unknown flag: --json"];

#[async_trait]
pub(crate) trait GhCliCommandRunner: Send + Sync {
    async fn run_gh(&self, working_dir: &Path, args: &[String]) -> AppResult<Vec<String>>;
    async fn run_git(&self, working_dir: &Path, args: &[String]) -> AppResult<()>;
}

struct RealGhCliCommandRunner;

#[async_trait]
impl GhCliCommandRunner for RealGhCliCommandRunner {
    async fn run_gh(&self, working_dir: &Path, args: &[String]) -> AppResult<Vec<String>> {
        GhCliGithubService::run_gh_process(working_dir, args).await
    }

    async fn run_git(&self, working_dir: &Path, args: &[String]) -> AppResult<()> {
        GhCliGithubService::run_git_process(working_dir, args).await
    }
}

/// Production GitHub service backed by the `gh` CLI
pub struct GhCliGithubService {
    runner: Arc<dyn GhCliCommandRunner>,
}

impl GhCliGithubService {
    pub fn new() -> Self {
        Self::with_runner(Arc::new(RealGhCliCommandRunner))
    }

    pub(crate) fn with_runner(runner: Arc<dyn GhCliCommandRunner>) -> Self {
        Self { runner }
    }

    /// Consume stdout + stderr from a spawned child in separate tasks.
    /// Returns (stdout_lines, sanitized_stderr_lines).
    async fn collect_output(
        child: &mut tokio::process::Child,
    ) -> AppResult<(Vec<String>, Vec<String>)> {
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::Infrastructure("Failed to capture stdout pipe".to_string())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            AppError::Infrastructure("Failed to capture stderr pipe".to_string())
        })?;

        let stdout_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                lines.push(line);
            }
            lines
        });

        let stderr_task = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let sanitized = sanitize_stderr_line(&line);
                lines.push(sanitized);
            }
            lines
        });

        let stdout_lines = stdout_task.await.map_err(|e| {
            AppError::Infrastructure(format!("stdout task panicked: {e}"))
        })?;
        let stderr_lines = stderr_task.await.map_err(|e| {
            AppError::Infrastructure(format!("stderr task panicked: {e}"))
        })?;

        Ok((stdout_lines, stderr_lines))
    }

    /// Run a `gh` command, collect output, wait for exit, and return stdout lines.
    /// Errors if the process exits non-zero.
    async fn run_gh_process<I, S>(working_dir: &Path, args: I) -> AppResult<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut child = tokio::process::Command::new("gh")
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Infrastructure(format!("Failed to spawn gh: {e}")))?;

        let result = timeout(SUBPROCESS_TIMEOUT, async {
            let (stdout, stderr) = Self::collect_output(&mut child).await?;
            let status = child.wait().await.map_err(|e| {
                AppError::Infrastructure(format!("Failed to wait for gh process: {e}"))
            })?;
            Ok::<_, AppError>((stdout, stderr, status))
        })
        .await
        .map_err(|_| AppError::Infrastructure("gh command timed out after 30s".to_string()))??;

        let (stdout, stderr, status) = result;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            let err_msg = stderr.join("\n");
            debug!(code, %err_msg, "gh command failed");
            return Err(AppError::Infrastructure(format!(
                "gh exited with code {code}: {err_msg}"
            )));
        }

        if !stderr.is_empty() {
            debug!(lines = ?stderr, "gh stderr output");
        }

        Ok(stdout)
    }

    /// Run a git command (for operations not covered by `gh`, e.g. push, fetch).
    async fn run_git_process<I, S>(working_dir: &Path, args: I) -> AppResult<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut child = tokio::process::Command::new("git")
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Infrastructure(format!("Failed to spawn git: {e}")))?;

        let result = timeout(SUBPROCESS_TIMEOUT, async {
            let stderr_handle = child.stderr.take();
            let stderr_task = tokio::spawn(async move {
                if let Some(stderr) = stderr_handle {
                    let mut lines = Vec::new();
                    let mut reader = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        lines.push(sanitize_stderr_line(&line));
                    }
                    lines
                } else {
                    Vec::new()
                }
            });
            let status = child.wait().await.map_err(|e| {
                AppError::Infrastructure(format!("Failed to wait for git process: {e}"))
            })?;
            let stderr = stderr_task.await.unwrap_or_default();
            Ok::<_, AppError>((status, stderr))
        })
        .await
        .map_err(|_| AppError::Infrastructure("git command timed out after 30s".to_string()))??;

        let (status, stderr) = result;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            let err_msg = stderr.join("\n");
            return Err(AppError::Infrastructure(format!(
                "git exited with code {code}: {err_msg}"
            )));
        }

        Ok(())
    }
}

impl Default for GhCliGithubService {
    fn default() -> Self {
        Self::new()
    }
}

fn build_create_pr_args(
    base: &str,
    head: &str,
    title: &str,
    body_file: &str,
    include_json: bool,
) -> Vec<String> {
    let mut args = vec![
        "pr".to_string(),
        "create".to_string(),
        "--draft".to_string(),
        "--base".to_string(),
        base.to_string(),
        "--head".to_string(),
        head.to_string(),
        "--title".to_string(),
        title.to_string(),
        "--body-file".to_string(),
        body_file.to_string(),
    ];
    if include_json {
        args.push("--json".to_string());
        args.push("number,url".to_string());
    }
    args
}

fn build_update_pr_args(pr_number: i64, title: &str, body_file: &str) -> Vec<String> {
    vec![
        "pr".to_string(),
        "edit".to_string(),
        pr_number.to_string(),
        "--title".to_string(),
        title.to_string(),
        "--body-file".to_string(),
        body_file.to_string(),
    ]
}

fn is_duplicate_pr_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    DUPLICATE_PR_FRAGMENTS.iter().any(|fragment| lower.contains(fragment))
}

fn is_create_pr_json_unsupported_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    CREATE_PR_UNSUPPORTED_JSON_FRAGMENTS
        .iter()
        .any(|fragment| lower.contains(fragment))
}

/// Sanitize a single stderr line:
/// 1. Filter lines containing secret keywords (case-insensitive) — full-line suppression
/// 2. Scrub token-embedded URLs: `https://<token>@github.com` → `https://***@github.com`
/// 3. Apply `redact()` as a second pass for any remaining regex-pattern secrets
pub(crate) fn sanitize_stderr_line(line: &str) -> String {
    let lower = line.to_lowercase();
    for keyword in SECRET_KEYWORDS {
        if lower.contains(keyword) {
            return "[REDACTED: potential secret in stderr]".to_string();
        }
    }
    let url_scrubbed = scrub_token_urls(line);
    redact(&url_scrubbed)
}

/// Replace `https://<anything>@github.com` with `https://***@github.com`
pub(crate) fn scrub_token_urls(s: &str) -> String {
    // Simple state-machine scan — avoids pulling in the regex crate
    let prefix = "https://";
    let separator = "@github.com";

    let mut result = String::with_capacity(s.len());
    let mut remaining = s;

    while let Some(start) = remaining.find(prefix) {
        result.push_str(&remaining[..start]);
        let after_prefix = &remaining[start + prefix.len()..];

        if let Some(at_pos) = after_prefix.find(separator) {
            // Check there's an actual token (non-empty) before the @
            if at_pos > 0 {
                result.push_str(prefix);
                result.push_str("***");
                result.push_str(separator);
                remaining = &after_prefix[at_pos + separator.len()..];
            } else {
                // No token — keep as-is
                result.push_str(prefix);
                remaining = after_prefix;
            }
        } else {
            // No @github.com after this https:// — keep as-is
            result.push_str(prefix);
            remaining = after_prefix;
        }
    }

    result.push_str(remaining);
    result
}

#[async_trait]
impl GithubServiceTrait for GhCliGithubService {
    async fn create_draft_pr(
        &self,
        working_dir: &Path,
        base: &str,
        head: &str,
        title: &str,
        body_file: &Path,
    ) -> AppResult<(i64, String)> {
        // gh pr create --draft --base <base> --head <head> --title <title> --body-file <file>
        //              --json number,url
        let body_file_str = body_file
            .to_str()
            .ok_or_else(|| {
                AppError::Infrastructure("body_file path is not valid UTF-8".to_string())
            })?
            .to_string();

        let json_args = build_create_pr_args(base, head, title, &body_file_str, true);
        let result = self.runner.run_gh(working_dir, &json_args).await;

        match result {
            Ok(stdout) => {
                let json_str = stdout.join("\n");
                parse_pr_create_output(&json_str)
            }
            Err(AppError::Infrastructure(msg)) if is_duplicate_pr_error(&msg) => {
                Err(AppError::DuplicatePr)
            }
            Err(AppError::Infrastructure(msg)) if is_create_pr_json_unsupported_error(&msg) => {
                warn!(
                    head,
                    "gh pr create does not support --json; retrying without JSON output"
                );
                let plain_args = build_create_pr_args(base, head, title, &body_file_str, false);
                let stdout = match self.runner.run_gh(working_dir, &plain_args).await {
                    Ok(stdout) => stdout,
                    Err(AppError::Infrastructure(msg)) if is_duplicate_pr_error(&msg) => {
                        return Err(AppError::DuplicatePr);
                    }
                    Err(other) => return Err(other),
                };
                let plain_output = stdout.join("\n");
                parse_pr_create_plain_output(&plain_output)
            }
            Err(other) => Err(other),
        }
    }

    async fn mark_pr_ready(&self, working_dir: &Path, pr_number: i64) -> AppResult<()> {
        // gh pr ready <number>
        let args = vec!["pr".to_string(), "ready".to_string(), pr_number.to_string()];
        self.runner.run_gh(working_dir, &args).await?;
        Ok(())
    }

    async fn update_pr_details(
        &self,
        working_dir: &Path,
        pr_number: i64,
        title: &str,
        body_file: &Path,
    ) -> AppResult<()> {
        let body_file_str = body_file
            .to_str()
            .ok_or_else(|| AppError::Infrastructure("body_file path is not valid UTF-8".to_string()))?
            .to_string();
        let args = build_update_pr_args(pr_number, title, &body_file_str);
        self.runner.run_gh(working_dir, &args).await?;
        Ok(())
    }

    async fn check_pr_status(&self, working_dir: &Path, pr_number: i64) -> AppResult<PrStatus> {
        // gh pr view <number> --json state,mergedAt,mergeCommit
        let args = vec![
            "pr".to_string(),
            "view".to_string(),
            pr_number.to_string(),
            "--json".to_string(),
            "state,mergedAt,mergeCommit".to_string(),
        ];
        let stdout = self.runner.run_gh(working_dir, &args).await?;

        let json_str = stdout.join("\n");
        parse_pr_status_output(&json_str)
    }

    async fn push_branch(&self, working_dir: &Path, branch: &str) -> AppResult<()> {
        // git push origin <branch> — fire-and-forget style (stdout null, stderr piped for safety)
        let args = vec![
            "push".to_string(),
            "origin".to_string(),
            branch.to_string(),
        ];
        self.runner.run_git(working_dir, &args).await
    }

    async fn close_pr(&self, working_dir: &Path, pr_number: i64) -> AppResult<()> {
        // gh pr close <number>
        let args = vec!["pr".to_string(), "close".to_string(), pr_number.to_string()];
        self.runner.run_gh(working_dir, &args).await?;
        Ok(())
    }

    async fn delete_remote_branch(&self, working_dir: &Path, branch: &str) -> AppResult<()> {
        // git push origin --delete <branch>
        // Already-deleted → "remote ref does not exist" → treat as no-op
        let mut child = tokio::process::Command::new("git")
            .args(["push", "origin", "--delete", branch])
            .current_dir(working_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Infrastructure(format!("Failed to spawn git: {e}")))?;

        let result = timeout(SUBPROCESS_TIMEOUT, async {
            let stderr_handle = child.stderr.take();
            let stderr_task = tokio::spawn(async move {
                if let Some(stderr) = stderr_handle {
                    let mut lines = Vec::new();
                    let mut reader = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        lines.push(sanitize_stderr_line(&line));
                    }
                    lines
                } else {
                    Vec::new()
                }
            });
            let status = child
                .wait()
                .await
                .map_err(|e| AppError::Infrastructure(format!("git wait failed: {e}")))?;
            let stderr = stderr_task.await.unwrap_or_default();
            Ok::<_, AppError>((status, stderr))
        })
        .await
        .map_err(|_| AppError::Infrastructure("git push --delete timed out".to_string()))??;

        let (status, stderr) = result;

        if status.success() {
            return Ok(());
        }

        // Treat "remote ref does not exist" as success (already deleted)
        let stderr_combined = stderr.join("\n").to_lowercase();
        if stderr_combined.contains("remote ref does not exist")
            || (stderr_combined.contains("error: unable to delete")
                && stderr_combined.contains("does not exist"))
            || stderr_combined.contains("no such ref")
        {
            warn!(branch, "Remote branch already deleted — treating as no-op");
            return Ok(());
        }

        Err(AppError::Infrastructure(format!(
            "git push --delete failed: {}",
            stderr.join("\n")
        )))
    }

    async fn fetch_remote(&self, working_dir: &Path, branch: &str) -> AppResult<()> {
        // git fetch origin <branch>
        let args = vec![
            "fetch".to_string(),
            "origin".to_string(),
            branch.to_string(),
        ];
        self.runner.run_git(working_dir, &args).await
    }

    async fn find_pr_by_head_branch(
        &self,
        working_dir: &Path,
        head: &str,
    ) -> AppResult<Option<(i64, String)>> {
        // gh pr list --head <head> --json number,url --state open
        let args = vec![
            "pr".to_string(),
            "list".to_string(),
            "--head".to_string(),
            head.to_string(),
            "--json".to_string(),
            "number,url".to_string(),
            "--state".to_string(),
            "open".to_string(),
        ];
        let stdout = self.runner.run_gh(working_dir, &args).await?;

        let json_str = stdout.join("\n");
        parse_pr_list_output(&json_str)
    }
}

// ── Output parsers ────────────────────────────────────────────────────────────

pub(crate) fn parse_pr_create_output(json_str: &str) -> AppResult<(i64, String)> {
    let v: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        AppError::Infrastructure(format!("Failed to parse gh pr create JSON: {e}\nRaw: {json_str}"))
    })?;

    let number = v["number"]
        .as_i64()
        .ok_or_else(|| AppError::Infrastructure("gh pr create: missing 'number' field".to_string()))?;
    let url = v["url"]
        .as_str()
        .ok_or_else(|| AppError::Infrastructure("gh pr create: missing 'url' field".to_string()))?
        .to_string();

    Ok((number, url))
}

pub(crate) fn parse_pr_create_plain_output(stdout_str: &str) -> AppResult<(i64, String)> {
    let url = stdout_str
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| "()[]<>{},'\"".contains(c)))
        .find(|token| token.starts_with("https://") && token.contains("github.com/") && token.contains("/pull/"))
        .ok_or_else(|| {
            AppError::Infrastructure(format!(
                "gh pr create fallback: could not find PR URL in output: {stdout_str}"
            ))
        })?
        .to_string();

    let pr_number = url
        .split("/pull/")
        .nth(1)
        .and_then(|tail| tail.split(['/', '?', '#']).next())
        .ok_or_else(|| {
            AppError::Infrastructure(format!(
                "gh pr create fallback: could not extract PR number from URL: {url}"
            ))
        })?
        .parse::<i64>()
        .map_err(|e| {
            AppError::Infrastructure(format!(
                "gh pr create fallback: invalid PR number in URL {url}: {e}"
            ))
        })?;

    Ok((pr_number, url))
}

pub(crate) fn parse_pr_list_output(json_str: &str) -> AppResult<Option<(i64, String)>> {
    let arr: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        AppError::Infrastructure(format!("Failed to parse gh pr list JSON: {e}\nRaw: {json_str}"))
    })?;

    let items = arr.as_array().ok_or_else(|| {
        AppError::Infrastructure(format!("gh pr list: expected JSON array, got: {json_str}"))
    })?;

    if items.is_empty() {
        return Ok(None);
    }

    let first = &items[0];
    let number = first["number"]
        .as_i64()
        .ok_or_else(|| AppError::Infrastructure("gh pr list: missing 'number' field".to_string()))?;
    let url = first["url"]
        .as_str()
        .ok_or_else(|| AppError::Infrastructure("gh pr list: missing 'url' field".to_string()))?
        .to_string();

    Ok(Some((number, url)))
}

pub(crate) fn parse_pr_status_output(json_str: &str) -> AppResult<PrStatus> {
    let v: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        AppError::Infrastructure(format!("Failed to parse gh pr view JSON: {e}\nRaw: {json_str}"))
    })?;

    let state = v["state"]
        .as_str()
        .ok_or_else(|| AppError::Infrastructure("gh pr view: missing 'state' field".to_string()))?;

    match state {
        "OPEN" => Ok(PrStatus::Open),
        "CLOSED" => Ok(PrStatus::Closed),
        "MERGED" => {
            // mergeCommit is an object with "oid" when merged, null otherwise
            let sha = v["mergeCommit"]["oid"].as_str().map(str::to_string);
            Ok(PrStatus::Merged { merge_commit_sha: sha })
        }
        other => Err(AppError::Infrastructure(format!(
            "gh pr view: unknown state '{other}'"
        ))),
    }
}
