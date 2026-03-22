// Merge validation: post-merge validation gate
//
// Extracted from side_effects.rs — runs project analysis commands to verify merge correctness.
// Decomposed into setup phase, validate phase, and orchestrator.

/// Delay before retrying a failed install command (ms).
/// Covers macOS filesystem lock recovery window (Spotlight indexing, npm `ENOTEMPTY` errors).
/// 500ms is sufficient for the realistic lock window while keeping latency low.
pub(super) const INSTALL_RETRY_DELAY_MS: u64 = 500;

/// Delay before retrying a failed validation command (ms).
/// Covers transient compilation timeouts and file-system lock windows.
/// 2 seconds allows caches to settle while keeping merge latency acceptable.
pub(super) const VALIDATE_RETRY_DELAY_MS: u64 = 2000;

/// Status string for failed validation/install log entries.
/// Used in ValidationLogEntry.status and compared in retry logic.
const STATUS_FAILED: &str = "failed";

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Emitter};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

use crate::domain::entities::{
    merge_progress_event::{
        derive_phases_from_analysis, map_command_to_phase, MergePhase, MergePhaseStatus,
        MergeProgressEvent, PhaseAnalysisEntry,
    },
    Project, Task,
};

use crate::utils::truncate_str;

/// Outcome of a cancellable shell command execution.
///
/// Replaces the nested `Result<Result<Output, io::Error>, JoinError>` from
/// `spawn_blocking` + `Command::output()` with a flat enum that adds explicit
/// cancellation support via `CancellationToken`.
pub(crate) enum CancellableCommandResult {
    /// Process completed normally (may have succeeded or failed — check `output.status`).
    Completed(std::process::Output),
    /// Failed to spawn the process or read its output.
    SpawnError(std::io::Error),
    /// Process was cancelled via the cancellation token.
    /// Child process tree has been killed.
    Cancelled,
}

/// Spawn a shell command with cancellation support.
///
/// Uses `tokio::process::Command` (async, non-blocking) instead of
/// `std::process::Command` inside `spawn_blocking` (blocking, uncancellable).
/// The process can be killed cooperatively via the `CancellationToken`, or
/// externally via PID-based kill (e.g., `kill_worktree_processes`).
///
/// `kill_on_drop(true)` provides a safety net: if the `Child` is dropped
/// without explicit cleanup, the OS sends SIGKILL to the process.
pub(crate) async fn spawn_cancellable_command(
    cmd: &str,
    cwd: &Path,
    cancel: &CancellationToken,
) -> CancellableCommandResult {
    let mut child = match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return CancellableCommandResult::SpawnError(e),
    };

    // Capture PID before taking stdout/stderr handles.
    let pid = child.id();

    // Take stdout/stderr handles so we can read them concurrently with wait().
    // This avoids the deadlock that occurs when a child writes more data than
    // the pipe buffer can hold while we're waiting for exit (child blocks on
    // write, we block on wait — neither makes progress).
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Read stdout/stderr concurrently with wait to avoid pipe-buffer deadlock.
    // If the child writes >64KB and nobody drains the pipe, the child blocks on
    // write and wait() never returns. By draining in parallel, both make progress.
    let stdout_fut = async {
        let mut buf = Vec::new();
        if let Some(mut out) = stdout_handle {
            let _ = out.read_to_end(&mut buf).await;
        }
        buf
    };
    let stderr_fut = async {
        let mut buf = Vec::new();
        if let Some(mut err) = stderr_handle {
            let _ = err.read_to_end(&mut buf).await;
        }
        buf
    };

    tokio::select! {
        biased; // Check cancellation first on each poll

        _ = cancel.cancelled() => {
            // Kill the process tree: children first, then the shell itself.
            if let Some(id) = pid {
                crate::domain::services::kill_process(id);
            }
            // Explicit kill + reap to avoid zombie processes.
            let _ = child.kill().await;
            let _ = child.wait().await;
            CancellableCommandResult::Cancelled
        }

        (status, stdout_bytes, stderr_bytes) = async {
            tokio::join!(child.wait(), stdout_fut, stderr_fut)
        } => {
            match status {
                Ok(exit_status) => {
                    CancellableCommandResult::Completed(std::process::Output {
                        status: exit_status,
                        stdout: stdout_bytes,
                        stderr: stderr_bytes,
                    })
                }
                Err(e) => CancellableCommandResult::SpawnError(e),
            }
        }
    }
}

/// Analysis entry for path-scoped build/validation commands.
/// Mirrors the HTTP handler's AnalysisEntry but kept local to avoid cross-module coupling.
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct MergeAnalysisEntry {
    pub(super) path: String,
    #[allow(dead_code)]
    pub(super) label: String,
    #[serde(default)]
    pub(super) validate: Vec<String>,
    #[serde(default)]
    pub(super) worktree_setup: Vec<String>,
}

/// Analysis entry for pre-execution setup commands.
/// Includes the `install` field (unlike MergeAnalysisEntry which omits it).
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct PreExecAnalysisEntry {
    pub(super) path: String,
    pub(super) label: String,
    #[serde(default)]
    pub(super) install: Option<String>,
    #[serde(default)]
    pub(super) worktree_setup: Vec<String>,
}

/// A single validation command execution record for streaming + storage.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ValidationLogEntry {
    pub phase: String,
    pub command: String,
    pub path: String,
    pub label: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// Whether this command was retried after initial failure.
    /// True = passed/failed on automatic retry (attempt 2/2).
    #[serde(default)]
    pub retried: bool,
    /// Path to full stdout log file (only set for failed commands)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_log_path: Option<String>,
    /// Path to full stderr log file (only set for failed commands)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_log_path: Option<String>,
}

impl ValidationLogEntry {
    /// Create a new ValidationLogEntry with all required fields.
    /// Sets retried=false, stdout_log_path=None, stderr_log_path=None by default.
    pub(super) fn new(
        phase: impl Into<String>,
        command: impl Into<String>,
        path: impl Into<String>,
        label: impl Into<String>,
        status: impl Into<String>,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        duration_ms: u64,
    ) -> Self {
        Self {
            phase: phase.into(),
            command: command.into(),
            path: path.into(),
            label: label.into(),
            status: status.into(),
            exit_code,
            stdout,
            stderr,
            duration_ms,
            retried: false,
            stdout_log_path: None,
            stderr_log_path: None,
        }
    }

    /// Attach full log file paths for a failed command.
    ///
    /// Writes full stdout/stderr to ~/.ralphx/logs/{task_id}/ and stores
    /// the paths on the entry so the fixer agent can read untruncated output.
    pub(crate) fn attach_failure_logs(&mut self, task_id: &str) {
        let (stdout_path, stderr_path) =
            write_failure_logs(task_id, &self.command, &self.stdout, &self.stderr);
        self.stdout_log_path = stdout_path;
        self.stderr_log_path = stderr_path;
    }
}

/// Result of running post-merge validation commands.
#[derive(Debug)]
pub(crate) struct ValidationResult {
    pub(crate) all_passed: bool,
    pub(crate) failures: Vec<ValidationFailure>,
    pub(crate) log: Vec<ValidationLogEntry>,
}

#[derive(Debug)]
pub(crate) struct ValidationFailure {
    pub(super) command: String,
    pub(super) path: String,
    pub(super) exit_code: Option<i32>,
    pub(super) stderr: String,
}

/// Result of running pre-execution setup commands.
#[derive(Debug)]
pub struct PreExecSetupResult {
    pub success: bool,
    pub log: Vec<ValidationLogEntry>,
}

/// Truncate a string to `max_len` bytes, appending "... (truncated)" if needed.
fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... (truncated)", truncate_str(s, max_len))
    }
}

/// Directory for validation log files: ~/.ralphx/logs/{task_id}/
pub(crate) fn validation_log_dir(task_id: &str) -> std::path::PathBuf {
    let home = validation_log_home_dir();
    home.join(".ralphx").join("logs").join(task_id)
}

#[cfg(test)]
fn validation_log_home_dir() -> std::path::PathBuf {
    // Lib tests run under a workspace sandbox where ambient HOME may not be writable.
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

#[cfg(not(test))]
fn validation_log_home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

/// Write full stdout/stderr to disk for a failed validation command.
///
/// Returns (stdout_log_path, stderr_log_path). Only writes non-empty outputs.
/// Failures are logged but don't block validation — this is best-effort.
fn write_failure_logs(
    task_id: &str,
    command: &str,
    stdout: &str,
    stderr: &str,
) -> (Option<String>, Option<String>) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let dir = validation_log_dir(task_id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!(task_id, error = %e, "Failed to create validation log dir");
        return (None, None);
    }

    // Hash command to create a unique, filesystem-safe filename
    let mut hasher = DefaultHasher::new();
    command.hash(&mut hasher);
    let cmd_hash = format!("{:016x}", hasher.finish());

    let stdout_path = if !stdout.is_empty() {
        let path = dir.join(format!("{cmd_hash}_stdout.log"));
        match std::fs::write(&path, stdout) {
            Ok(()) => Some(path.to_string_lossy().to_string()),
            Err(e) => {
                tracing::warn!(task_id, error = %e, "Failed to write stdout log");
                None
            }
        }
    } else {
        None
    };

    let stderr_path = if !stderr.is_empty() {
        let path = dir.join(format!("{cmd_hash}_stderr.log"));
        match std::fs::write(&path, stderr) {
            Ok(()) => Some(path.to_string_lossy().to_string()),
            Err(e) => {
                tracing::warn!(task_id, error = %e, "Failed to write stderr log");
                None
            }
        }
    } else {
        None
    };

    (stdout_path, stderr_path)
}

/// Clean up validation log directory for a task.
pub(crate) fn cleanup_validation_logs(task_id: &str) {
    let dir = validation_log_dir(task_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::warn!(task_id, error = %e, "Failed to clean up validation logs");
        } else {
            tracing::debug!(task_id, "Cleaned up validation log directory");
        }
    }
}

/// Emit a high-level merge progress event for UI display.
///
/// Note: All `let _ = handle.emit(...)` in this module are intentional —
/// no frontend listeners is OK for progress/validation events.
///
/// Also stores the event in the global hydration store so the frontend
/// can fetch it on mount (events fire before frontend subscribes).
pub(super) fn emit_merge_progress<R: tauri::Runtime>(
    app_handle: Option<&AppHandle<R>>,
    task_id: &str,
    phase: MergePhase,
    status: MergePhaseStatus,
    message: String,
) {
    let event = MergeProgressEvent::new(task_id.to_string(), phase, status, message);

    // Always store in hydration map (even without app_handle — backend-only merges still need hydration)
    crate::domain::entities::merge_progress_event::store_merge_progress(&event);

    if let Some(handle) = app_handle {
        let _ = handle.emit("task:merge_progress", event);
    }
}

/// Parse an `ln -s` or `ln -sfn` command to extract `(source, target)` as absolute PathBufs.
///
/// Resolves relative paths against `cwd`. Returns `None` for non-symlink commands
/// or commands that can't be parsed (wrong arg count, missing -s flag, etc.).
///
/// This is the canonical parser used by both `try_handle_symlink_idempotent` and the
/// collision detection pre-scan — a single source of truth for symlink argument extraction.
pub(super) fn parse_symlink_command(cmd: &str, cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "ln" {
        return None;
    }

    // Must have -s flag (could be -s, -sf, -sn, -sfn, etc.)
    let has_symlink_flag = parts.iter().any(|p| p.starts_with('-') && p.contains('s'));
    if !has_symlink_flag {
        return None;
    }

    // Extract non-flag arguments (source and target)
    let args: Vec<&str> = parts
        .iter()
        .filter(|p| !p.starts_with('-') && **p != "ln")
        .copied()
        .collect();
    if args.len() != 2 {
        return None;
    }

    let source = if Path::new(args[0]).is_absolute() {
        PathBuf::from(args[0])
    } else {
        cwd.join(args[0])
    };
    let target = if Path::new(args[1]).is_absolute() {
        PathBuf::from(args[1])
    } else {
        cwd.join(args[1])
    };

    Some((source, target))
}

/// Idempotent symlink handling for worktree setup commands.
///
/// Parses `ln -s[f] <source> <target>` commands and handles existing targets:
/// - Source == target (circular) → returns `Some(log_entry)` (skip, prevents damage)
/// - Correct symlink already exists → returns `Some(log_entry)` (skip)
/// - Circular self-symlink exists → removes it, returns `None` (re-run command)
/// - Wrong symlink or real file/dir → removes target, returns `None` (re-run command)
/// - Target doesn't exist → returns `None` (run normally)
/// - Non-symlink commands → returns `None` (pass through)
pub(super) fn try_handle_symlink_idempotent(
    cmd: &str,
    cwd: &Path,
    label: &str,
    resolved_path: &str,
) -> Option<ValidationLogEntry> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "ln" {
        return None;
    }

    // Must have -s flag (could be -s, -sf, -sn, -sfn, etc.)
    let has_symlink_flag = parts.iter().any(|p| p.starts_with('-') && p.contains('s'));
    if !has_symlink_flag {
        return None;
    }

    // Extract non-flag arguments (source and target)
    let args: Vec<&str> = parts
        .iter()
        .filter(|p| !p.starts_with('-') && **p != "ln")
        .copied()
        .collect();
    if args.len() != 2 {
        return None;
    }

    let source = PathBuf::from(args[0]);
    let target_path = if Path::new(args[1]).is_absolute() {
        PathBuf::from(args[1])
    } else {
        cwd.join(args[1])
    };

    // Layer 2: Prevent circular symlinks where source == target.
    // This happens when {worktree_path} resolves to the main repo path
    // (e.g., merge succeeded without creating a worktree).
    let resolved_source = if source.is_absolute() {
        source.clone()
    } else {
        cwd.join(&source)
    };
    if resolved_source == target_path {
        tracing::warn!(
            command = %cmd,
            source = %resolved_source.display(),
            target = %target_path.display(),
            "Worktree setup: skipping circular symlink (source == target)"
        );
        return Some(ValidationLogEntry::new(
            "setup",
            cmd,
            resolved_path,
            label,
            "skipped",
            Some(0),
            String::new(),
            "Skipped: circular symlink (source == target)".to_string(),
            0,
        ));
    }

    if target_path.is_symlink() {
        if let Ok(existing_target) = std::fs::read_link(&target_path) {
            // Layer 3: Detect and remove circular self-symlinks left by previous runs.
            if existing_target == target_path || existing_target == Path::new(args[1]) {
                tracing::warn!(
                    command = %cmd,
                    target = %target_path.display(),
                    "Worktree setup: removing circular self-symlink from previous run"
                );
                let _ = std::fs::remove_file(&target_path);
                return None; // Proceed with normal command execution
            }

            if existing_target == source {
                // Correct symlink already exists — report as cached (no action needed)
                tracing::info!(
                    command = %cmd,
                    target = %target_path.display(),
                    "Worktree setup: symlink already correct, cached"
                );
                return Some(ValidationLogEntry::new(
                    "setup",
                    cmd,
                    resolved_path,
                    label,
                    "cached",
                    Some(0),
                    String::new(),
                    "Symlink already exists and is correct".to_string(),
                    0,
                ));
            }
        }
        // Wrong symlink — remove so command can recreate
        tracing::info!(
            command = %cmd,
            target = %target_path.display(),
            "Worktree setup: removing incorrect symlink before re-creation"
        );
        let _ = std::fs::remove_file(&target_path);
    } else if target_path.exists() {
        // Real file/dir exists at target — preserve it. This typically means a fixer
        // agent built locally (creating a real target/ dir) and re-validation should
        // use those artifacts instead of destroying them and symlinking to main repo.
        tracing::info!(
            command = %cmd,
            target = %target_path.display(),
            "Worktree setup: preserving existing real path (skipping symlink)"
        );
        return Some(ValidationLogEntry::new(
            "setup",
            cmd,
            resolved_path,
            label,
            "cached",
            Some(0),
            String::new(),
            "Skipped: real directory exists (preserved)".to_string(),
            0,
        ));
    }

    None // Proceed with normal command execution
}

/// Run worktree setup commands (symlinks, etc.) — non-fatal.
///
/// Returns the log entries and whether any setup command failed.
async fn run_setup_phase(
    entries: &[MergeAnalysisEntry],
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    context: Option<&str>,
    cancel: &CancellationToken,
) -> (Vec<ValidationLogEntry>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut setup_had_failures = false;

    let has_setup_commands = entries.iter().any(|e| !e.worktree_setup.is_empty());
    if has_setup_commands {
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::worktree_setup(),
            MergePhaseStatus::Started,
            "Setting up worktree environment".to_string(),
        );
    }

    let setup_phase_start = std::time::Instant::now();
    let mut setup_count = 0;
    for entry in entries {
        setup_count += entry.worktree_setup.len();
    }
    if setup_count > 0 {
        tracing::info!(
            task_id = task_id_str,
            command_count = setup_count,
            "run_validation_commands: starting setup phase"
        );
    }

    // --- Collision detection pre-scan ---
    // Build a map from resolved symlink target → [(entry_path, full_resolved_cmd)]
    // to detect when multiple entries map to the same target path.
    use std::collections::{HashMap, HashSet};
    let mut target_to_entries: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    for entry in entries {
        for cmd_str in &entry.worktree_setup {
            let resolved_cmd = resolve(cmd_str);
            let resolved_path = resolve(&entry.path);
            let cmd_cwd = if resolved_path == "." {
                merge_cwd.to_path_buf()
            } else {
                merge_cwd.join(&resolved_path)
            };
            if let Some((_src, target)) = parse_symlink_command(&resolved_cmd, &cmd_cwd) {
                target_to_entries
                    .entry(target)
                    .or_default()
                    .push((resolved_path.clone(), resolved_cmd.clone()));
            }
        }
    }

    // Find targets claimed by more than one entry (collisions).
    // Determine the winner for each colliding target:
    //   - If the root entry (entry.path == ".") is a collider → root wins
    //   - Otherwise → first collider by JSON order wins
    let mut collision_targets: HashSet<PathBuf> = HashSet::new();
    let mut collision_winners: HashMap<PathBuf, String> = HashMap::new(); // target → winning entry_path
    for (target, claimants) in &target_to_entries {
        if claimants.len() > 1 {
            collision_targets.insert(target.clone());
            // Winner: root entry ("." after resolve) if present, else first by JSON order
            let winner_path = claimants
                .iter()
                .find(|(ep, _)| ep == ".")
                .map(|(ep, _)| ep.clone())
                .unwrap_or_else(|| claimants[0].0.clone());
            collision_winners.insert(target.clone(), winner_path);
        }
    }

    // Track which collision targets have already been claimed (to handle the case
    // where the same winner has multiple commands mapping to the same target).
    let mut claimed_targets: HashSet<PathBuf> = HashSet::new();

    for entry in entries {
        for cmd_str in &entry.worktree_setup {
            let resolved_cmd = resolve(cmd_str);
            let resolved_path = resolve(&entry.path);
            let cmd_cwd = if resolved_path == "." {
                merge_cwd.to_path_buf()
            } else {
                merge_cwd.join(&resolved_path)
            };

            // --- Collision check ---
            // If this command's target collides with another entry, apply winner-based skipping.
            if let Some((_src, target)) = parse_symlink_command(&resolved_cmd, &cmd_cwd) {
                if collision_targets.contains(&target) {
                    let winner_path = collision_winners.get(&target).cloned().unwrap_or_default();
                    let is_winner = resolved_path == winner_path;
                    let already_claimed = claimed_targets.contains(&target);

                    if is_winner && !already_claimed {
                        // This entry wins the collision — let it proceed, mark target claimed
                        claimed_targets.insert(target.clone());
                        // (fall through to normal processing below)
                    } else {
                        // Loser or duplicate winner claim — skip
                        tracing::warn!(
                            entry_path = %resolved_path,
                            target = %target.display(),
                            winner_path = %winner_path,
                            "Skipping colliding worktree_setup for entry '{}' — target '{}' collides with entry '{}'",
                            resolved_path,
                            target.display(),
                            winner_path,
                        );
                        let skip_entry = ValidationLogEntry::new(
                            "setup",
                            &resolved_cmd,
                            &resolved_path,
                            &entry.label,
                            "skipped",
                            Some(0),
                            String::new(),
                            format!(
                                "Skipped: target '{}' collides with entry '{}'",
                                target.display(),
                                winner_path,
                            ),
                            0,
                        );
                        if let Some(handle) = app_handle {
                            let mut event_data = serde_json::json!({
                                "task_id": task_id_str,
                                "phase": skip_entry.phase,
                                "command": skip_entry.command,
                                "path": skip_entry.path,
                                "label": skip_entry.label,
                                "status": skip_entry.status,
                                "exit_code": skip_entry.exit_code,
                                "stderr": skip_entry.stderr,
                                "duration_ms": skip_entry.duration_ms,
                            });
                            if let Some(ctx) = context {
                                event_data["context"] = serde_json::json!(ctx);
                            }
                            let _ = handle.emit("merge:validation_step", event_data);
                        }
                        log.push(skip_entry);
                        continue;
                    }
                }
            }

            // --- Parent directory creation ---
            // Ensure the symlink target's parent directory exists before running the command.
            if let Some((_src, target)) = parse_symlink_command(&resolved_cmd, &cmd_cwd) {
                if let Some(parent) = target.parent() {
                    if !parent.exists() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            tracing::warn!(
                                command = %resolved_cmd,
                                parent = %parent.display(),
                                error = %e,
                                "Worktree setup: failed to create parent dir for symlink target (continuing)"
                            );
                        }
                    }
                }
            }

            // Idempotent symlink handling: skip if correct symlink exists,
            // remove stale target if wrong, pass through for non-symlink commands
            if let Some(skip_entry) =
                try_handle_symlink_idempotent(&resolved_cmd, &cmd_cwd, &entry.label, &resolved_path)
            {
                if let Some(handle) = app_handle {
                    let mut event_data = serde_json::json!({
                        "task_id": task_id_str,
                        "phase": skip_entry.phase,
                        "command": skip_entry.command,
                        "path": skip_entry.path,
                        "label": skip_entry.label,
                        "status": skip_entry.status,
                        "exit_code": skip_entry.exit_code,
                        "stderr": skip_entry.stderr,
                        "duration_ms": skip_entry.duration_ms,
                    });
                    if let Some(ctx) = context {
                        event_data["context"] = serde_json::json!(ctx);
                    }
                    let _ = handle.emit("merge:validation_step", event_data);
                }
                log.push(skip_entry);
                continue;
            }

            // Harden symlink commands: use -sfn flags to prevent nesting bugs when
            // the target already exists as a symlink (handles complex/chained commands
            // that try_handle_symlink_idempotent can't parse)
            let resolved_cmd = if resolved_cmd.contains("ln -s ") && !resolved_cmd.contains("-sfn")
            {
                resolved_cmd
                    .replace("ln -s ", "ln -sfn ")
                    .replace("ln -sf ", "ln -sfn ")
            } else {
                resolved_cmd
            };

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let mut event_data = serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "setup",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                });
                if let Some(ctx) = context {
                    event_data["context"] = serde_json::json!(ctx);
                }
                let _ = handle.emit("merge:validation_step", event_data);
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running worktree setup command"
            );

            let start = std::time::Instant::now();

            let result = spawn_cancellable_command(&resolved_cmd, &cmd_cwd, cancel).await;

            let log_entry = match result {
                CancellableCommandResult::Completed(output) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let status = if output.status.success() {
                        "success"
                    } else {
                        "failed"
                    };

                    if !output.status.success() {
                        setup_had_failures = true;
                        tracing::warn!(
                            command = %resolved_cmd,
                            stderr = %stderr_raw,
                            "Worktree setup command failed (non-fatal)"
                        );
                    }

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                        ..Default::default()
                    }
                }
                CancellableCommandResult::SpawnError(e) => {
                    setup_had_failures = true;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::warn!(
                        command = %resolved_cmd,
                        error = %e,
                        "Worktree setup command failed (non-fatal)"
                    );

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: STATUS_FAILED.to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                        ..Default::default()
                    }
                }
                CancellableCommandResult::Cancelled => {
                    setup_had_failures = true;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::warn!(
                        command = %resolved_cmd,
                        "Worktree setup command cancelled"
                    );

                    ValidationLogEntry {
                        phase: "setup".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: STATUS_FAILED.to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: "Command cancelled".to_string(),
                        duration_ms,
                        ..Default::default()
                    }
                }
            };

            if let Some(handle) = app_handle {
                let mut event_data = serde_json::json!({
                    "task_id": task_id_str,
                    "phase": log_entry.phase,
                    "command": log_entry.command,
                    "path": log_entry.path,
                    "label": log_entry.label,
                    "status": log_entry.status,
                    "exit_code": log_entry.exit_code,
                    "stdout": log_entry.stdout,
                    "stderr": log_entry.stderr,
                    "duration_ms": log_entry.duration_ms,
                });
                if let Some(ctx) = context {
                    event_data["context"] = serde_json::json!(ctx);
                }
                let _ = handle.emit("merge:validation_step", event_data);
            }
            log.push(log_entry);
        }
    }

    if setup_count > 0 {
        let setup_duration_ms = setup_phase_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = setup_duration_ms,
            command_count = setup_count,
            "run_validation_commands: completed setup phase"
        );
    }

    // Always emit worktree setup completion event so the frontend can show
    // a checkmark (Passed) or skip indicator, even when there are no setup commands.
    if setup_had_failures {
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::worktree_setup(),
            MergePhaseStatus::Failed,
            "Worktree setup completed with warnings (non-fatal)".to_string(),
        );
    } else {
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::worktree_setup(),
            MergePhaseStatus::Passed,
            if has_setup_commands {
                "Worktree setup completed successfully".to_string()
            } else {
                "Worktree setup skipped (no commands)".to_string()
            },
        );
    }

    (log, setup_had_failures)
}

/// Emit `Skipped` progress events and log entries for all validate commands
/// that were not yet executed (fail-fast abort).
///
/// Iterates through all entries/commands and skips those already in the log,
/// emitting `MergePhaseStatus::Skipped` for the remainder.
fn emit_skipped_for_remaining(
    entries: &[MergeAnalysisEntry],
    _merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    log: &mut Vec<ValidationLogEntry>,
    failed_path: &str,
    failed_cmd: &str,
) {
    use crate::domain::entities::merge_progress_event::{map_command_to_phase, MergePhaseStatus};

    let mut past_failure = false;
    for entry in entries {
        let resolved_path = resolve(&entry.path);
        for cmd_str in &entry.validate {
            let resolved_cmd = resolve(cmd_str);

            // Skip commands we already ran (they're already in the log)
            if !past_failure {
                if resolved_cmd == failed_cmd && resolved_path == failed_path {
                    past_failure = true;
                }
                continue;
            }

            // Emit high-level skipped event
            let phase = map_command_to_phase(&resolved_cmd);
            emit_merge_progress(
                app_handle,
                task_id_str,
                phase,
                MergePhaseStatus::Skipped,
                format!("{} skipped (fail-fast)", resolved_cmd),
            );

            // Emit step-level skipped event
            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": "validate",
                        "command": resolved_cmd,
                        "path": resolved_path,
                        "label": entry.label,
                        "status": "skipped",
                        "exit_code": null,
                        "stdout": "",
                        "stderr": "Skipped due to prior validation failure (fail-fast)",
                        "duration_ms": 0,
                    }),
                );
            }

            log.push(ValidationLogEntry {
                phase: "validate".to_string(),
                command: resolved_cmd,
                path: resolved_path.clone(),
                label: entry.label.clone(),
                status: "skipped".to_string(),
                exit_code: None,
                stdout: String::new(),
                stderr: "Skipped due to prior validation failure (fail-fast)".to_string(),
                duration_ms: 0,
                ..Default::default()
            });
        }
    }
}

/// Run validate-phase commands, checking cache where possible.
///
/// When `validation_mode` is `Block` or `AutoFix`, aborts on first failure (fail-fast)
/// and emits `Skipped` events for remaining commands. In `Warn` mode, runs all commands.
///
/// Returns (log_entries, failures, ran_any).
async fn run_validate_phase(
    entries: &[MergeAnalysisEntry],
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    cached_log: Option<&[ValidationLogEntry]>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    validation_mode: &crate::domain::entities::MergeValidationMode,
    cancel: &CancellationToken,
) -> (Vec<ValidationLogEntry>, Vec<ValidationFailure>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut failures = Vec::new();
    let mut ran_any = false;

    let validate_count: usize = entries.iter().map(|e| e.validate.len()).sum();
    let validate_phase_start = std::time::Instant::now();
    if validate_count > 0 {
        tracing::info!(
            task_id = task_id_str,
            command_count = validate_count,
            "run_validation_commands: starting validate phase"
        );
    }

    for entry in entries {
        if entry.validate.is_empty() {
            continue;
        }

        let resolved_path = resolve(&entry.path);
        let cmd_cwd = if resolved_path == "." {
            merge_cwd.to_path_buf()
        } else {
            merge_cwd.join(&resolved_path)
        };

        for cmd_str in &entry.validate {
            let resolved_cmd = resolve(cmd_str);
            ran_any = true;

            // Check cache: skip previously-passed validate commands when SHA matches
            if let Some(cached) = cached_log {
                let cached_hit = cached.iter().find(|c| {
                    c.phase == "validate"
                        && c.command == resolved_cmd
                        && c.path == resolved_path
                        && (c.status == "success" || c.status == "cached")
                });
                if let Some(prev) = cached_hit {
                    tracing::info!(
                        command = %resolved_cmd,
                        "Skipping validation command (cached, SHA unchanged)"
                    );

                    let phase = map_command_to_phase(&resolved_cmd);
                    emit_merge_progress(
                        app_handle,
                        task_id_str,
                        phase,
                        MergePhaseStatus::Passed,
                        format!("{} (cached)", resolved_cmd),
                    );

                    let log_entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd,
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "cached".to_string(),
                        exit_code: prev.exit_code,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                        ..Default::default()
                    };
                    if let Some(handle) = app_handle {
                        let _ = handle.emit(
                            "merge:validation_step",
                            serde_json::json!({
                                "task_id": task_id_str,
                                "phase": log_entry.phase,
                                "command": log_entry.command,
                                "path": log_entry.path,
                                "label": log_entry.label,
                                "status": log_entry.status,
                                "exit_code": log_entry.exit_code,
                                "duration_ms": log_entry.duration_ms,
                            }),
                        );
                    }
                    log.push(log_entry);
                    continue;
                }
            }

            // Emit high-level merge progress event
            let phase = map_command_to_phase(&resolved_cmd);
            emit_merge_progress(
                app_handle,
                task_id_str,
                phase.clone(),
                MergePhaseStatus::Started,
                format!("Running {}", resolved_cmd),
            );

            // Emit "running" event before execution
            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": "validate",
                        "command": resolved_cmd,
                        "path": resolved_path,
                        "label": entry.label,
                        "status": "running",
                    }),
                );
            }

            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                "Running post-merge validation command"
            );

            let start = std::time::Instant::now();

            let result = spawn_cancellable_command(&resolved_cmd, &cmd_cwd, cancel).await;

            let (mut log_entry, mut failure) = match result {
                CancellableCommandResult::Completed(output) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();

                    let failure = if !output.status.success() {
                        tracing::warn!(
                            command = %resolved_cmd,
                            exit_code = ?output.status.code(),
                            stderr = %stderr_raw,
                            "Post-merge validation command failed"
                        );
                        Some(ValidationFailure {
                            command: resolved_cmd.clone(),
                            path: resolved_path.clone(),
                            exit_code: output.status.code(),
                            stderr: format!(
                                "{}{}",
                                if stderr_raw.is_empty() {
                                    ""
                                } else {
                                    &stderr_raw
                                },
                                if stdout_raw.is_empty() {
                                    String::new()
                                } else {
                                    format!("\nstdout: {}", stdout_raw)
                                },
                            ),
                        })
                    } else {
                        tracing::info!(command = %resolved_cmd, "Post-merge validation command passed");
                        None
                    };

                    // Emit high-level merge progress completion event
                    if output.status.success() {
                        emit_merge_progress(
                            app_handle,
                            task_id_str,
                            phase,
                            MergePhaseStatus::Passed,
                            format!("{} passed", resolved_cmd),
                        );
                    } else {
                        emit_merge_progress(
                            app_handle,
                            task_id_str,
                            phase,
                            MergePhaseStatus::Failed,
                            format!("{} failed", resolved_cmd),
                        );
                    }

                    let status = if output.status.success() {
                        "success"
                    } else {
                        "failed"
                    };
                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: status.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                        ..Default::default()
                    };

                    (entry, failure)
                }
                CancellableCommandResult::SpawnError(e) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::error!(command = %resolved_cmd, error = %e, "Failed to execute validation command");

                    emit_merge_progress(
                        app_handle,
                        task_id_str,
                        phase,
                        MergePhaseStatus::Failed,
                        format!("Failed to execute: {}", e),
                    );

                    let failure = ValidationFailure {
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        exit_code: None,
                        stderr: format!("Failed to execute: {}", e),
                    };

                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: STATUS_FAILED.to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                        duration_ms,
                        ..Default::default()
                    };

                    (entry, Some(failure))
                }
                CancellableCommandResult::Cancelled => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    tracing::warn!(
                        command = %resolved_cmd,
                        "Validation command cancelled"
                    );

                    emit_merge_progress(
                        app_handle,
                        task_id_str,
                        phase,
                        MergePhaseStatus::Failed,
                        format!("{} cancelled", resolved_cmd),
                    );

                    let failure = ValidationFailure {
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        exit_code: None,
                        stderr: "Command cancelled".to_string(),
                    };

                    let entry = ValidationLogEntry {
                        phase: "validate".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: STATUS_FAILED.to_string(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: "Command cancelled".to_string(),
                        duration_ms,
                        ..Default::default()
                    };

                    (entry, Some(failure))
                }
            };

            // Retry once if the validation command failed with a real exit code
            // (not spawn error, not cancelled, not already cancelled by token).
            if log_entry.status == STATUS_FAILED
                && log_entry.exit_code.is_some()
                && !cancel.is_cancelled()
            {
                tracing::warn!(
                    command = %resolved_cmd,
                    delay_ms = VALIDATE_RETRY_DELAY_MS,
                    "Validation command failed, retrying once after {}ms (attempt 2/2)",
                    VALIDATE_RETRY_DELAY_MS
                );

                let retry_phase = map_command_to_phase(&resolved_cmd);
                emit_merge_progress(
                    app_handle,
                    task_id_str,
                    retry_phase,
                    MergePhaseStatus::Started,
                    format!("Retrying {} ...", resolved_cmd),
                );

                tokio::time::sleep(std::time::Duration::from_millis(VALIDATE_RETRY_DELAY_MS)).await;

                // Abort retry if cancelled during the delay
                if !cancel.is_cancelled() {
                    let retry_start = std::time::Instant::now();
                    let retry_result =
                        spawn_cancellable_command(&resolved_cmd, &cmd_cwd, cancel).await;

                    if let CancellableCommandResult::Completed(output) = retry_result {
                        let retry_duration_ms = retry_start.elapsed().as_millis() as u64;
                        let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();

                        if output.status.success() {
                            tracing::info!(
                                command = %resolved_cmd,
                                "Validation command succeeded on retry (attempt 2/2)"
                            );

                            let retry_phase = map_command_to_phase(&resolved_cmd);
                            emit_merge_progress(
                                app_handle,
                                task_id_str,
                                retry_phase,
                                MergePhaseStatus::Passed,
                                format!("{} passed on retry", resolved_cmd),
                            );

                            log_entry = ValidationLogEntry {
                                phase: "validate".to_string(),
                                command: resolved_cmd.clone(),
                                path: resolved_path.clone(),
                                label: entry.label.clone(),
                                status: "success".to_string(),
                                exit_code: output.status.code(),
                                stdout: truncate_output(&stdout_raw, 2000),
                                stderr: truncate_output(&stderr_raw, 2000),
                                duration_ms: retry_duration_ms,
                                retried: true,
                                ..Default::default()
                            };
                            failure = None;
                        } else {
                            tracing::warn!(
                                command = %resolved_cmd,
                                stderr = %stderr_raw,
                                "Validation command retry also failed (attempt 2/2)"
                            );

                            let retry_phase = map_command_to_phase(&resolved_cmd);
                            emit_merge_progress(
                                app_handle,
                                task_id_str,
                                retry_phase,
                                MergePhaseStatus::Failed,
                                format!("{} failed on retry", resolved_cmd),
                            );

                            log_entry = ValidationLogEntry {
                                phase: "validate".to_string(),
                                command: resolved_cmd.clone(),
                                path: resolved_path.clone(),
                                label: entry.label.clone(),
                                status: STATUS_FAILED.to_string(),
                                exit_code: output.status.code(),
                                stdout: truncate_output(&stdout_raw, 2000),
                                stderr: truncate_output(&stderr_raw, 2000),
                                duration_ms: retry_duration_ms,
                                retried: true,
                                ..Default::default()
                            };
                            failure = Some(ValidationFailure {
                                command: resolved_cmd.clone(),
                                path: resolved_path.clone(),
                                exit_code: output.status.code(),
                                stderr: format!(
                                    "{}{}",
                                    if stderr_raw.is_empty() {
                                        ""
                                    } else {
                                        &stderr_raw
                                    },
                                    if stdout_raw.is_empty() {
                                        String::new()
                                    } else {
                                        format!("\nstdout: {}", stdout_raw)
                                    },
                                ),
                            });
                        }
                    }
                    // If spawn failed or was cancelled on retry, keep the original failure
                }
            }

            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": log_entry.phase,
                        "command": log_entry.command,
                        "path": log_entry.path,
                        "label": log_entry.label,
                        "status": log_entry.status,
                        "exit_code": log_entry.exit_code,
                        "stdout": log_entry.stdout,
                        "stderr": log_entry.stderr,
                        "duration_ms": log_entry.duration_ms,
                    }),
                );
            }
            log.push(log_entry);
            if let Some(f) = failure {
                failures.push(f);

                // Fail-fast: in Block/AutoFix mode, abort remaining commands on first failure
                use crate::domain::entities::MergeValidationMode;
                if matches!(
                    validation_mode,
                    MergeValidationMode::Block | MergeValidationMode::AutoFix
                ) {
                    tracing::info!(
                        task_id = task_id_str,
                        failed_command = %resolved_cmd,
                        "Fail-fast: aborting remaining validation commands (mode={validation_mode})"
                    );

                    // Emit Skipped events for all remaining commands
                    emit_skipped_for_remaining(
                        entries,
                        merge_cwd,
                        task_id_str,
                        app_handle,
                        resolve,
                        &mut log,
                        &resolved_path,
                        &resolved_cmd,
                    );

                    // Break out of both loops (entry + command)
                    // We return early — the outer loop won't continue
                    let validate_duration_ms = validate_phase_start.elapsed().as_millis() as u64;
                    tracing::info!(
                        task_id = task_id_str,
                        duration_ms = validate_duration_ms,
                        command_count = validate_count,
                        failure_count = failures.len(),
                        "run_validation_commands: completed validate phase (fail-fast)"
                    );
                    return (log, failures, ran_any);
                }
            }
        }
    }

    if validate_count > 0 {
        let validate_duration_ms = validate_phase_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = validate_duration_ms,
            command_count = validate_count,
            failure_count = failures.len(),
            "run_validation_commands: completed validate phase"
        );
    }

    (log, failures, ran_any)
}

/// Load effective analysis, resolve template vars, and run all validate commands.
///
/// Returns `None` if no analysis entries exist (backward compatible — skip validation).
/// Returns `Some(ValidationResult)` with pass/fail details otherwise.
///
/// When `app_handle` is `Some`, emits `merge:validation_step` events for real-time UI streaming.
/// All executed commands are recorded in `ValidationResult::log` for metadata storage.
///
/// When `cached_log` is provided, validate-phase commands that previously passed (status
/// "success" or "cached") are skipped and emitted as "cached" instead of re-running.
/// Setup-phase commands always re-run. Previously-failed commands always re-run.
pub(crate) async fn run_validation_commands(
    project: &Project,
    task: &Task,
    merge_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    cached_log: Option<&[ValidationLogEntry]>,
    validation_mode: &crate::domain::entities::MergeValidationMode,
    cancel: &CancellationToken,
) -> Option<ValidationResult> {
    let overall_start = std::time::Instant::now();
    tracing::info!(
        task_id = task_id_str,
        cwd = %merge_cwd.display(),
        "run_validation_commands: starting validation"
    );

    // Load effective analysis: custom_analysis ?? detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref())?;

    let entries: Vec<MergeAnalysisEntry> = match serde_json::from_str(analysis_json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse project analysis JSON, skipping validation");
            return None;
        }
    };

    if entries.is_empty() {
        return None;
    }

    // Emit dynamic phase list to frontend for timeline rendering.
    // Convert MergeAnalysisEntry → PhaseAnalysisEntry (only validate commands needed).
    // Derive phase list and store in hydration map + emit to frontend
    {
        let phase_entries: Vec<PhaseAnalysisEntry> = entries
            .iter()
            .map(|e| PhaseAnalysisEntry {
                validate: e.validate.clone(),
            })
            .collect();
        let phases = derive_phases_from_analysis(&phase_entries);

        // Always store in hydration map (even without app_handle)
        crate::domain::entities::merge_progress_event::store_merge_phase_list(
            task_id_str,
            phases.clone(),
        );

        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "task:merge_phases",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phases": phases,
                }),
            );
        }
    }

    // Acquire a worktree permit so cleanup_branch_and_worktree_internal knows
    // this worktree is in active use and should not be deleted underneath us.
    // The permit is RAII — it auto-releases when this function returns.
    let _worktree_permit = crate::domain::services::acquire_worktree_permit(merge_cwd);

    // NOTE: Previously, hardening code here would merge worktree_setup from
    // detected_analysis into custom_analysis entries that had empty worktree_setup.
    // This was removed because it treated intentionally empty worktree_setup []
    // (e.g., user explicitly removing target/ symlinks) as "not configured" and
    // overrode the user's choice. If custom_analysis exists, trust it fully.

    // Build template resolver
    let project_root = &project.working_directory;
    let worktree_path = merge_cwd.to_str().unwrap_or(project_root);
    let task_branch = task.task_branch.as_deref().unwrap_or("");

    let project_root_owned = project_root.clone();
    let worktree_path_owned = worktree_path.to_string();
    let task_branch_owned = task_branch.to_string();

    let resolve = |s: &str| -> String {
        s.replace("{project_root}", &project_root_owned)
            .replace("{worktree_path}", &worktree_path_owned)
            .replace("{task_branch}", &task_branch_owned)
    };

    // Phase 1: Setup — skip when merge_cwd == project_root (no worktree created).
    // Running symlink setup with {worktree_path} == {project_root} would create
    // circular symlinks (source == target), potentially destroying node_modules/target.
    let skip_setup = worktree_path == project_root.as_str();
    let (mut log, _setup_had_failures) = if skip_setup {
        tracing::info!(
            task_id = task_id_str,
            "Skipping worktree setup phase: merge_cwd equals project_root (no worktree)"
        );
        // Emit Passed event so the frontend shows a checkmark for WorktreeSetup
        emit_merge_progress(
            app_handle,
            task_id_str,
            MergePhase::worktree_setup(),
            MergePhaseStatus::Passed,
            "Worktree setup skipped (no worktree needed)".to_string(),
        );
        (Vec::new(), false)
    } else {
        run_setup_phase(
            &entries,
            merge_cwd,
            task_id_str,
            app_handle,
            &resolve,
            None,
            cancel,
        )
        .await
    };

    // Phase 2: Validate
    let (validate_log, failures, ran_any) = run_validate_phase(
        &entries,
        merge_cwd,
        task_id_str,
        app_handle,
        cached_log,
        &resolve,
        validation_mode,
        cancel,
    )
    .await;

    log.extend(validate_log);

    if !ran_any {
        let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
        tracing::info!(
            task_id = task_id_str,
            duration_ms = overall_duration_ms,
            "run_validation_commands: no validation commands to run"
        );
        return None;
    }

    let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
    let all_passed = failures.is_empty();
    tracing::info!(
        task_id = task_id_str,
        duration_ms = overall_duration_ms,
        all_passed = all_passed,
        failure_count = failures.len(),
        total_commands = log.len(),
        "run_validation_commands: completed validation"
    );

    // Write full output to disk for failed commands so the fixer agent
    // can read untruncated logs from ~/.ralphx/logs/{task_id}/
    if !all_passed {
        for entry in &mut log {
            if entry.status == STATUS_FAILED {
                entry.attach_failure_logs(task_id_str);
            }
        }
    }

    Some(ValidationResult {
        all_passed,
        failures,
        log,
    })
}

/// Format validation failures as a JSON metadata string for MergeIncomplete.
pub(crate) fn format_validation_error_metadata(
    failures: &[ValidationFailure],
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    let failure_details: Vec<serde_json::Value> = failures
        .iter()
        .map(|f| {
            serde_json::json!({
                "command": f.command,
                "path": f.path,
                "exit_code": f.exit_code,
                "stderr": truncate_str(&f.stderr, 2000),
            })
        })
        .collect();

    serde_json::json!({
        "error": format!("Merge validation failed: {} command(s) failed", failures.len()),
        "validation_failures": failure_details,
        "validation_log": log,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Check if task metadata has the skip_validation flag set, and clear it (one-shot).
pub(super) fn take_skip_validation_flag(task: &mut Task) -> bool {
    let Some(meta_str) = task.metadata.as_ref() else {
        return false;
    };
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(meta_str) else {
        return false;
    };
    let flag = val
        .get("skip_validation")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if flag {
        if let Some(obj) = val.as_object_mut() {
            obj.remove("skip_validation");
            task.metadata = Some(val.to_string());
        }
    }
    flag
}

/// Format validation warnings as a JSON metadata string for Warn mode.
/// Stores the log but allows merge to proceed.
pub(super) fn format_validation_warn_metadata(
    log: &[ValidationLogEntry],
    source_branch: &str,
    target_branch: &str,
) -> String {
    serde_json::json!({
        "validation_log": log,
        "validation_warnings": true,
        "source_branch": source_branch,
        "target_branch": target_branch,
    })
    .to_string()
}

/// Extract cached validation log from task metadata if the source branch SHA matches.
///
/// Returns `Some(entries)` when the previous validation ran against the same source SHA,
/// meaning the branch code has not changed and previously-passed checks can be skipped.
///
/// Note: Caching is effective in worktree mode. In local mode, rebase rewrites the source
/// branch SHA on each retry, so cache hits are rare.
pub(super) fn extract_cached_validation(
    task: &Task,
    current_sha: &str,
) -> Option<Vec<ValidationLogEntry>> {
    let meta_str = task.metadata.as_ref()?;
    let val: serde_json::Value = serde_json::from_str(meta_str).ok()?;
    let stored_sha = val.get("validation_source_sha")?.as_str()?;
    if stored_sha != current_sha {
        return None;
    }
    let log_val = val.get("validation_log")?;
    serde_json::from_value::<Vec<ValidationLogEntry>>(log_val.clone()).ok()
}

/// Run install commands for pre-execution setup.
/// Returns (log_entries, had_failures).
pub(super) async fn run_install_phase(
    entries: &[PreExecAnalysisEntry],
    exec_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    resolve: &(dyn Fn(&str) -> String + Send + Sync),
    context: &str,
    cancel: &CancellationToken,
) -> (Vec<ValidationLogEntry>, bool) {
    let mut log: Vec<ValidationLogEntry> = Vec::new();
    let mut install_had_failures = false;

    for entry in entries {
        let Some(ref cmd_str) = entry.install else {
            continue;
        };

        let resolved_cmd = resolve(cmd_str);
        let resolved_path = resolve(&entry.path);
        let cmd_cwd = if resolved_path == "." {
            exec_cwd.to_path_buf()
        } else {
            exec_cwd.join(&resolved_path)
        };

        // Skip install if node_modules already exists (symlink from setup phase or prior install)
        let nm_path = cmd_cwd.join("node_modules");
        if nm_path.exists() || nm_path.is_symlink() {
            tracing::info!(
                command = %resolved_cmd,
                cwd = %cmd_cwd.display(),
                is_symlink = nm_path.is_symlink(),
                "Skipping install: node_modules already exists"
            );
            log.push(ValidationLogEntry {
                phase: "install".to_string(),
                command: resolved_cmd.clone(),
                path: resolved_path.clone(),
                label: entry.label.clone(),
                status: "skipped".to_string(),
                exit_code: None,
                stdout: String::new(),
                stderr: "node_modules already exists — install skipped".to_string(),
                duration_ms: 0,
                ..Default::default()
            });
            continue;
        }

        // Emit "running" event before execution
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "merge:validation_step",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phase": "install",
                    "command": resolved_cmd,
                    "path": resolved_path,
                    "label": entry.label,
                    "status": "running",
                    "context": context,
                }),
            );
        }

        tracing::info!(
            command = %resolved_cmd,
            cwd = %cmd_cwd.display(),
            "Running pre-execution install command"
        );

        let start = std::time::Instant::now();

        let result = spawn_cancellable_command(&resolved_cmd, &cmd_cwd, cancel).await;

        let mut log_entry = match result {
            CancellableCommandResult::Completed(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                let status = if output.status.success() {
                    "success"
                } else {
                    "failed"
                };

                if !output.status.success() {
                    tracing::warn!(
                        command = %resolved_cmd,
                        stderr = %stderr_raw,
                        "Pre-execution install command failed"
                    );
                }

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: status.to_string(),
                    exit_code: output.status.code(),
                    stdout: truncate_output(&stdout_raw, 2000),
                    stderr: truncate_output(&stderr_raw, 2000),
                    duration_ms,
                    ..Default::default()
                }
            }
            CancellableCommandResult::SpawnError(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::warn!(
                    command = %resolved_cmd,
                    error = %e,
                    "Pre-execution install command failed"
                );

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: STATUS_FAILED.to_string(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: truncate_output(&format!("Failed to execute: {}", e), 2000),
                    duration_ms,
                    ..Default::default()
                }
            }
            CancellableCommandResult::Cancelled => {
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::warn!(
                    command = %resolved_cmd,
                    "Pre-execution install command cancelled"
                );

                ValidationLogEntry {
                    phase: "install".to_string(),
                    command: resolved_cmd.clone(),
                    path: resolved_path.clone(),
                    label: entry.label.clone(),
                    status: STATUS_FAILED.to_string(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: "Command cancelled".to_string(),
                    duration_ms,
                    ..Default::default()
                }
            }
        };

        // Retry once if the install command failed (transient errors like ENOTEMPTY)
        if log_entry.status == STATUS_FAILED {
            tracing::warn!(
                command = %resolved_cmd,
                delay_ms = INSTALL_RETRY_DELAY_MS,
                "Install command failed, retrying after {}ms (attempt 2/2)",
                INSTALL_RETRY_DELAY_MS
            );
            tokio::time::sleep(std::time::Duration::from_millis(INSTALL_RETRY_DELAY_MS)).await;

            // Emit "running" event for retry attempt
            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    "merge:validation_step",
                    serde_json::json!({
                        "task_id": task_id_str,
                        "phase": "install",
                        "command": resolved_cmd,
                        "path": resolved_path,
                        "label": entry.label,
                        "status": "running",
                        "context": context,
                    }),
                );
            }

            let retry_start = std::time::Instant::now();
            let retry_result = spawn_cancellable_command(&resolved_cmd, &cmd_cwd, cancel).await;

            if let CancellableCommandResult::Completed(output) = retry_result {
                let duration_ms = retry_start.elapsed().as_millis() as u64;
                let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                if output.status.success() {
                    tracing::info!(
                        command = %resolved_cmd,
                        "Install command succeeded on retry (attempt 2/2)"
                    );
                    log_entry = ValidationLogEntry {
                        phase: "install".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: "success".to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                        ..Default::default()
                    };
                } else {
                    tracing::warn!(
                        command = %resolved_cmd,
                        stderr = %stderr_raw,
                        "Install command retry also failed (attempt 2/2)"
                    );
                    log_entry = ValidationLogEntry {
                        phase: "install".to_string(),
                        command: resolved_cmd.clone(),
                        path: resolved_path.clone(),
                        label: entry.label.clone(),
                        status: STATUS_FAILED.to_string(),
                        exit_code: output.status.code(),
                        stdout: truncate_output(&stdout_raw, 2000),
                        stderr: truncate_output(&stderr_raw, 2000),
                        duration_ms,
                        ..Default::default()
                    };
                }
            }
            // If spawn failed or was cancelled, keep the original failure log_entry
        }

        // Set failure flag based on final outcome
        if log_entry.status == STATUS_FAILED {
            install_had_failures = true;
        }

        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "merge:validation_step",
                serde_json::json!({
                    "task_id": task_id_str,
                    "phase": log_entry.phase,
                    "command": log_entry.command,
                    "path": log_entry.path,
                    "label": log_entry.label,
                    "status": log_entry.status,
                    "exit_code": log_entry.exit_code,
                    "stdout": log_entry.stdout,
                    "stderr": log_entry.stderr,
                    "duration_ms": log_entry.duration_ms,
                    "context": context,
                }),
            );
        }
        log.push(log_entry);
    }

    (log, install_had_failures)
}

/// Load effective analysis, resolve template vars, and run pre-execution setup commands.
///
/// Returns `None` if no analysis entries exist (backward compatible — skip setup).
/// Returns `Some(PreExecSetupResult)` with success/failure details otherwise.
///
/// When `app_handle` is `Some`, emits `merge:validation_step` events with the provided context
/// for real-time UI streaming. All executed commands are recorded in `PreExecSetupResult::log`
/// for metadata storage.
///
/// This function runs worktree_setup + install commands only. No validate steps.
pub async fn run_pre_execution_setup(
    project: &Project,
    task: &Task,
    exec_cwd: &Path,
    task_id_str: &str,
    app_handle: Option<&tauri::AppHandle>,
    context: &str,
    cancel: &CancellationToken,
) -> Option<PreExecSetupResult> {
    let overall_start = std::time::Instant::now();
    tracing::info!(
        task_id = task_id_str,
        cwd = %exec_cwd.display(),
        "run_pre_execution_setup: starting pre-execution setup"
    );

    // Load effective analysis: custom_analysis ?? detected_analysis
    let analysis_json = project
        .custom_analysis
        .as_ref()
        .or(project.detected_analysis.as_ref())?;

    let entries: Vec<PreExecAnalysisEntry> = match serde_json::from_str(analysis_json) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse project analysis JSON, skipping pre-execution setup");
            return None;
        }
    };

    if entries.is_empty() {
        return None;
    }

    // NOTE: Previously, hardening code here would merge worktree_setup from
    // detected_analysis into custom_analysis entries that had empty worktree_setup.
    // This was removed because it treated intentionally empty worktree_setup []
    // (e.g., user explicitly removing target/ symlinks) as "not configured" and
    // overrode the user's choice. If custom_analysis exists, trust it fully.

    // Build template resolver
    let project_root = &project.working_directory;
    let worktree_path = exec_cwd.to_str().unwrap_or(project_root);
    let task_branch = task.task_branch.as_deref().unwrap_or("");

    let project_root_owned = project_root.clone();
    let worktree_path_owned = worktree_path.to_string();
    let task_branch_owned = task_branch.to_string();

    let resolve = |s: &str| -> String {
        s.replace("{project_root}", &project_root_owned)
            .replace("{worktree_path}", &worktree_path_owned)
            .replace("{task_branch}", &task_branch_owned)
    };

    // Phase 1: Worktree Setup (non-fatal, reuse existing run_setup_phase logic)
    // Convert PreExecAnalysisEntry to MergeAnalysisEntry for reuse
    let merge_entries: Vec<MergeAnalysisEntry> = entries
        .iter()
        .map(|e| MergeAnalysisEntry {
            path: e.path.clone(),
            label: e.label.clone(),
            validate: Vec::new(),
            worktree_setup: e.worktree_setup.clone(),
        })
        .collect();

    // Skip worktree setup when exec_cwd == project_root (no worktree created).
    // Running symlink setup with {worktree_path} == {project_root} would create
    // circular symlinks (source == target), potentially destroying node_modules/target.
    let skip_setup = worktree_path == project_root.as_str();
    let (mut log, _setup_had_failures) = if skip_setup {
        tracing::info!(
            task_id = task_id_str,
            "Skipping worktree setup phase (pre-exec): exec_cwd equals project_root (no worktree)"
        );
        (Vec::new(), false)
    } else {
        run_setup_phase(
            &merge_entries,
            exec_cwd,
            task_id_str,
            app_handle,
            &resolve,
            Some(context),
            cancel,
        )
        .await
    };

    // Phase 2: Install (fatal based on merge_validation_mode)
    let (install_log, install_had_failures) = run_install_phase(
        &entries,
        exec_cwd,
        task_id_str,
        app_handle,
        &resolve,
        context,
        cancel,
    )
    .await;

    log.extend(install_log);

    let overall_duration_ms = overall_start.elapsed().as_millis() as u64;
    let success = !install_had_failures;
    tracing::info!(
        task_id = task_id_str,
        duration_ms = overall_duration_ms,
        success = success,
        total_commands = log.len(),
        "run_pre_execution_setup: completed pre-execution setup"
    );

    // Write full output to disk for failed setup commands
    if !success {
        for entry in &mut log {
            if entry.status == STATUS_FAILED {
                entry.attach_failure_logs(task_id_str);
            }
        }
    }

    Some(PreExecSetupResult { success, log })
}
